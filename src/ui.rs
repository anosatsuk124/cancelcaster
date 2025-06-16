use crate::audio::AudioProcessor;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct CancelCasterApp {
    audio_processor: Arc<Mutex<AudioProcessor>>,
    is_running: bool,
    echo_cancellation: bool,
    noise_reduction: bool,
    input_level: f32,
    output_level: f32,
    selected_input_device: usize,
    selected_output_device: usize,
}

impl CancelCasterApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Result<Self, Box<dyn std::error::Error>> {
        let audio_processor = Arc::new(Mutex::new(AudioProcessor::new()?));
        
        let (selected_input_device, selected_output_device) = if let Ok(processor) = audio_processor.lock() {
            (processor.get_selected_input_index(), processor.get_selected_output_index())
        } else {
            (0, 0)
        };
        
        Ok(Self {
            audio_processor,
            is_running: false,
            echo_cancellation: true,
            noise_reduction: true,
            input_level: 0.0,
            output_level: 0.0,
            selected_input_device,
            selected_output_device,
        })
    }
}

impl eframe::App for CancelCasterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update audio levels
        if let Ok(processor) = self.audio_processor.lock() {
            self.input_level = processor.get_input_level();
            self.output_level = processor.get_output_level();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("CancelCaster - Audio Noise Cancellation");
            ui.separator();

            // Control Panel
            ui.horizontal(|ui| {
                if ui.button(if self.is_running { "Stop" } else { "Start" }).clicked() {
                    if self.is_running {
                        if let Ok(mut processor) = self.audio_processor.lock() {
                            processor.stop();
                        }
                        self.is_running = false;
                    } else {
                        if let Ok(mut processor) = self.audio_processor.lock() {
                            match self.start_audio_processing(&mut processor) {
                                Ok(()) => self.is_running = true,
                                Err(e) => {
                                    eprintln!("Failed to start audio processing: {}", e);
                                }
                            }
                        }
                    }
                }

                ui.separator();
                
                ui.label("Status:");
                ui.colored_label(
                    if self.is_running { egui::Color32::GREEN } else { egui::Color32::RED },
                    if self.is_running { "Running" } else { "Stopped" }
                );
            });

            ui.separator();

            // Device Selection
            ui.heading("Audio Devices");
            
            // Get device info (clone to avoid borrowing issues)
            let (input_devices, output_devices) = if let Ok(processor) = self.audio_processor.lock() {
                (processor.get_input_devices().clone(), processor.get_output_devices().clone())
            } else {
                (Vec::new(), Vec::new())
            };
            
            let mut input_device_changed = None;
            let mut output_device_changed = None;
            
            // Input device selection
            ui.horizontal(|ui| {
                ui.label("Input Device:");
                
                if !input_devices.is_empty() && self.selected_input_device < input_devices.len() {
                    egui::ComboBox::from_id_source("input_device")
                        .selected_text(&input_devices[self.selected_input_device].name)
                        .show_ui(ui, |ui| {
                            for (i, device_info) in input_devices.iter().enumerate() {
                                let text = if device_info.is_default {
                                    format!("{} (Default)", device_info.name)
                                } else {
                                    device_info.name.clone()
                                };
                                
                                if ui.selectable_value(&mut self.selected_input_device, i, text).changed() {
                                    input_device_changed = Some(i);
                                }
                            }
                        });
                }
            });
            
            // Output device selection
            ui.horizontal(|ui| {
                ui.label("Output Device:");
                
                if !output_devices.is_empty() && self.selected_output_device < output_devices.len() {
                    egui::ComboBox::from_id_source("output_device")
                        .selected_text(&output_devices[self.selected_output_device].name)
                        .show_ui(ui, |ui| {
                            for (i, device_info) in output_devices.iter().enumerate() {
                                let text = if device_info.is_default {
                                    format!("{} (Default)", device_info.name)
                                } else {
                                    device_info.name.clone()
                                };
                                
                                if ui.selectable_value(&mut self.selected_output_device, i, text).changed() {
                                    output_device_changed = Some(i);
                                }
                            }
                        });
                }
            });
            
            // Apply device changes
            if let Some(index) = input_device_changed {
                if let Ok(mut processor) = self.audio_processor.lock() {
                    if let Err(e) = processor.set_input_device(index) {
                        eprintln!("Failed to set input device: {}", e);
                    }
                }
            }
            
            if let Some(index) = output_device_changed {
                if let Ok(mut processor) = self.audio_processor.lock() {
                    if let Err(e) = processor.set_output_device(index) {
                        eprintln!("Failed to set output device: {}", e);
                    }
                }
            }

            ui.separator();

            // Settings
            ui.heading("Settings");
            
            let mut noise_changed = false;
            
            ui.checkbox(&mut self.echo_cancellation, "Echo Cancellation")
                .on_hover_text("Removes application audio from microphone input using phase inversion");
            
            if ui.checkbox(&mut self.noise_reduction, "Noise Reduction").changed() {
                noise_changed = true;
            }
            ui.label("Reduces background noise using spectral subtraction");

            // Apply setting changes
            if noise_changed {
                if let Ok(mut processor) = self.audio_processor.lock() {
                    processor.set_echo_cancellation(self.echo_cancellation);
                    processor.set_noise_reduction(self.noise_reduction);
                }
            }

            ui.separator();

            // Audio Levels
            ui.heading("Audio Levels");
            
            ui.horizontal(|ui| {
                ui.label("Input:");
                ui.add(egui::ProgressBar::new(self.input_level * 10.0).show_percentage());
            });
            
            ui.horizontal(|ui| {
                ui.label("Output:");
                ui.add(egui::ProgressBar::new(self.output_level * 10.0).show_percentage());
            });

            ui.separator();

            // Information
            ui.heading("Information");
            ui.label("• This application captures microphone input and system audio");
            ui.label("• It applies phase inversion to cancel echo from applications");
            ui.label("• Noise reduction is applied using spectral subtraction");
            ui.label("• Processed audio is sent to loopback for use in other applications");
            
            ui.separator();
            
            // Debug Info
            if ui.collapsing("Debug Information", |ui| {
                ui.label(format!("Echo Cancellation: {}", self.echo_cancellation));
                ui.label(format!("Noise Reduction: {}", self.noise_reduction));
                ui.label(format!("Input Level: {:.3}", self.input_level));
                ui.label(format!("Output Level: {:.3}", self.output_level));
            }).header_response.clicked() {}
        });

        // Request repaint for real-time updates
        ctx.request_repaint();
    }
}

impl CancelCasterApp {
    fn start_audio_processing(&self, processor: &mut AudioProcessor) -> Result<(), Box<dyn std::error::Error>> {
        processor.start_input_capture()?;
        processor.start_loopback_capture()?;
        processor.start_processing()?;
        processor.start_loopback_output()?;
        Ok(())
    }
}