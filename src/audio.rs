use anyhow::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, Stream, StreamConfig, SupportedStreamConfig,
};
use ringbuf::{HeapRb, Rb};
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

impl DeviceInfo {
    pub fn new(name: String, is_default: bool) -> Self {
        Self { name, is_default }
    }
}

pub struct AudioProcessor {
    host: Host,
    input_devices: Vec<Device>,
    output_devices: Vec<Device>,
    input_device_info: Vec<DeviceInfo>,
    output_device_info: Vec<DeviceInfo>,
    selected_input_device: Option<Device>,
    selected_output_device: Option<Device>,
    selected_input_index: usize,
    selected_output_index: usize,
    loopback_device: Option<Device>,
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    loopback_stream: Option<Stream>,
    mic_buffer: Arc<Mutex<HeapRb<f32>>>,
    app_buffer: Arc<Mutex<HeapRb<f32>>>,
    processed_buffer: Arc<Mutex<HeapRb<f32>>>,
    sample_rate: u32,
    channels: u16,
    is_processing: bool,
    noise_reduction_enabled: bool,
    echo_cancellation_enabled: bool,
}

impl AudioProcessor {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        
        // Enumerate input devices
        let mut input_devices = Vec::new();
        let mut input_device_info = Vec::new();
        let default_input = host.default_input_device();
        let default_input_name = default_input.as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "Unknown".to_string());
        
        for device in host.input_devices()? {
            let device_name = device.name().unwrap_or_else(|_| "Unknown Device".to_string());
            let is_default = device_name == default_input_name;
            input_devices.push(device);
            input_device_info.push(DeviceInfo::new(device_name, is_default));
        }
        
        // Enumerate output devices
        let mut output_devices = Vec::new();
        let mut output_device_info = Vec::new();
        let default_output = host.default_output_device();
        let default_output_name = default_output.as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "Unknown".to_string());
        
        for device in host.output_devices()? {
            let device_name = device.name().unwrap_or_else(|_| "Unknown Device".to_string());
            let is_default = device_name == default_output_name;
            output_devices.push(device);
            output_device_info.push(DeviceInfo::new(device_name, is_default));
        }
        
        // Find default device indices
        let selected_input_index = input_device_info.iter()
            .position(|info| info.is_default)
            .unwrap_or(0);
        let selected_output_index = output_device_info.iter()
            .position(|info| info.is_default)
            .unwrap_or(0);
        
        let selected_input_device = input_devices.get(selected_input_index).cloned();
        let selected_output_device = output_devices.get(selected_output_index).cloned();
        
        if let Some(ref device) = selected_input_device {
            info!("Selected input device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));
        }
        if let Some(ref device) = selected_output_device {
            info!("Selected output device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));
        }

        let buffer_size = 48000; // 1 second at 48kHz
        let mic_buffer = Arc::new(Mutex::new(HeapRb::<f32>::new(buffer_size)));
        let app_buffer = Arc::new(Mutex::new(HeapRb::<f32>::new(buffer_size)));
        let processed_buffer = Arc::new(Mutex::new(HeapRb::<f32>::new(buffer_size)));

        Ok(Self {
            host,
            input_devices,
            output_devices,
            input_device_info,
            output_device_info,
            selected_input_device,
            selected_output_device,
            selected_input_index,
            selected_output_index,
            loopback_device: None,
            input_stream: None,
            output_stream: None,
            loopback_stream: None,
            mic_buffer,
            app_buffer,
            processed_buffer,
            sample_rate: 48000,
            channels: 2,
            is_processing: false,
            noise_reduction_enabled: true,
            echo_cancellation_enabled: true,
        })
    }

    pub fn start_input_capture(&mut self) -> Result<()> {
        if let Some(device) = &self.selected_input_device {
            let config = device.default_input_config()?;
            info!("Input config: {:?}", config);
            
            let sample_rate = config.sample_rate().0;
            let channels = config.channels();
            
            self.sample_rate = sample_rate;
            self.channels = channels;

            let mic_buffer = Arc::clone(&self.mic_buffer);
            
            let stream = device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buffer) = mic_buffer.lock() {
                        for &sample in data {
                            let _ = buffer.push(sample);
                        }
                    }
                },
                |err| error!("Input stream error: {}", err),
                None,
            )?;

            stream.play()?;
            self.input_stream = Some(stream);
            info!("Input capture started");
        }
        Ok(())
    }

    pub fn start_loopback_capture(&mut self) -> Result<()> {
        // This is a simplified implementation
        // In a real application, you'd need platform-specific code to capture system audio
        info!("Loopback capture would be implemented here");
        Ok(())
    }

    pub fn start_processing(&mut self) -> Result<()> {
        self.is_processing = true;
        
        // Spawn processing thread
        let mic_buffer = Arc::clone(&self.mic_buffer);
        let app_buffer = Arc::clone(&self.app_buffer);
        let processed_buffer = Arc::clone(&self.processed_buffer);
        let echo_cancellation = self.echo_cancellation_enabled;
        let noise_reduction = self.noise_reduction_enabled;

        tokio::spawn(async move {
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(1024);
            let ifft = planner.plan_fft_inverse(1024);
            
            loop {
                // Process audio in chunks
                let mut mic_samples = Vec::new();
                let mut app_samples = Vec::new();
                
                // Extract samples from buffers
                if let (Ok(mut mic_buf), Ok(mut app_buf)) = 
                    (mic_buffer.lock(), app_buffer.lock()) {
                    
                    for _ in 0..1024 {
                        if let Some(sample) = mic_buf.pop() {
                            mic_samples.push(sample);
                        } else {
                            mic_samples.push(0.0);
                        }
                        
                        if let Some(sample) = app_buf.pop() {
                            app_samples.push(sample);
                        } else {
                            app_samples.push(0.0);
                        }
                    }
                }

                if mic_samples.len() == 1024 {
                    let processed = Self::process_audio_chunk(
                        &mic_samples,
                        &app_samples,
                        echo_cancellation,
                        noise_reduction,
                        fft.as_ref(),
                        ifft.as_ref(),
                    );

                    // Store processed samples
                    if let Ok(mut proc_buf) = processed_buffer.lock() {
                        for sample in processed {
                            let _ = proc_buf.push(sample);
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });

        info!("Audio processing started");
        Ok(())
    }

    fn process_audio_chunk(
        mic_samples: &[f32],
        app_samples: &[f32],
        echo_cancellation: bool,
        noise_reduction: bool,
        fft: &dyn rustfft::Fft<f32>,
        ifft: &dyn rustfft::Fft<f32>,
    ) -> Vec<f32> {
        let mut processed = mic_samples.to_vec();
        
        if echo_cancellation {
            // Phase inversion for echo cancellation
            for (i, &app_sample) in app_samples.iter().enumerate() {
                if i < processed.len() {
                    processed[i] -= app_sample; // Subtract inverted app audio
                }
            }
        }

        if noise_reduction {
            // Simple spectral subtraction for noise reduction
            processed = Self::spectral_subtraction(&processed, fft, ifft);
        }

        processed
    }

    fn spectral_subtraction(
        samples: &[f32],
        fft: &dyn rustfft::Fft<f32>,
        ifft: &dyn rustfft::Fft<f32>,
    ) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .map(|&x| Complex::new(x, 0.0))
            .collect();
        
        // Pad to FFT size if needed
        buffer.resize(fft.len(), Complex::new(0.0, 0.0));
        
        // Forward FFT
        fft.process(&mut buffer);
        
        // Apply spectral subtraction (simplified)
        for sample in &mut buffer {
            let magnitude = sample.norm();
            let noise_floor = 0.1; // Estimated noise floor
            let alpha = 2.0; // Over-subtraction factor
            
            if magnitude > noise_floor {
                let new_magnitude = magnitude - alpha * noise_floor;
                let new_magnitude = new_magnitude.max(0.1 * magnitude); // Don't over-subtract
                *sample = *sample * (new_magnitude / magnitude);
            }
        }
        
        // Inverse FFT
        ifft.process(&mut buffer);
        
        buffer.iter().map(|c| c.re / buffer.len() as f32).collect()
    }

    pub fn start_loopback_output(&mut self) -> Result<()> {
        if let Some(device) = &self.selected_output_device {
            let config = device.default_output_config()?;
            let processed_buffer = Arc::clone(&self.processed_buffer);
            
            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut buffer) = processed_buffer.lock() {
                        for sample in data.iter_mut() {
                            *sample = buffer.pop().unwrap_or(0.0);
                        }
                    }
                },
                |err| error!("Output stream error: {}", err),
                None,
            )?;

            stream.play()?;
            self.loopback_stream = Some(stream);
            info!("Loopback output started");
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_processing = false;
        
        if let Some(stream) = self.input_stream.take() {
            drop(stream);
        }
        if let Some(stream) = self.output_stream.take() {
            drop(stream);
        }
        if let Some(stream) = self.loopback_stream.take() {
            drop(stream);
        }
        
        info!("Audio processing stopped");
    }

    pub fn set_echo_cancellation(&mut self, enabled: bool) {
        self.echo_cancellation_enabled = enabled;
    }

    pub fn set_noise_reduction(&mut self, enabled: bool) {
        self.noise_reduction_enabled = enabled;
    }

    pub fn is_processing(&self) -> bool {
        self.is_processing
    }

    pub fn get_input_level(&self) -> f32 {
        if let Ok(buffer) = self.mic_buffer.lock() {
            let samples: Vec<f32> = buffer.iter().copied().collect();
            if !samples.is_empty() {
                let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
                return rms;
            }
        }
        0.0
    }

    pub fn get_output_level(&self) -> f32 {
        if let Ok(buffer) = self.processed_buffer.lock() {
            let samples: Vec<f32> = buffer.iter().copied().collect();
            if !samples.is_empty() {
                let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
                return rms;
            }
        }
        0.0
    }

    pub fn get_input_devices(&self) -> &Vec<DeviceInfo> {
        &self.input_device_info
    }

    pub fn get_output_devices(&self) -> &Vec<DeviceInfo> {
        &self.output_device_info
    }

    pub fn get_selected_input_index(&self) -> usize {
        self.selected_input_index
    }

    pub fn get_selected_output_index(&self) -> usize {
        self.selected_output_index
    }

    pub fn set_input_device(&mut self, index: usize) -> Result<()> {
        if index < self.input_devices.len() {
            self.selected_input_index = index;
            self.selected_input_device = self.input_devices.get(index).cloned();
            
            if self.is_processing {
                // Stop current input stream if running
                if let Some(stream) = self.input_stream.take() {
                    drop(stream);
                }
                // Restart with new device
                self.start_input_capture()?;
            }
            
            info!("Input device changed to: {}", 
                  self.input_device_info[index].name);
        }
        Ok(())
    }

    pub fn set_output_device(&mut self, index: usize) -> Result<()> {
        if index < self.output_devices.len() {
            self.selected_output_index = index;
            self.selected_output_device = self.output_devices.get(index).cloned();
            
            if self.is_processing {
                // Stop current output stream if running
                if let Some(stream) = self.loopback_stream.take() {
                    drop(stream);
                }
                // Restart with new device
                self.start_loopback_output()?;
            }
            
            info!("Output device changed to: {}", 
                  self.output_device_info[index].name);
        }
        Ok(())
    }
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        self.stop();
    }
}