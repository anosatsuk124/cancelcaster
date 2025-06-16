# CancelCaster

A cross-platform audio noise cancellation application built in Rust with egui. CancelCaster captures microphone input and system audio, applies phase inversion for echo cancellation, and noise reduction using spectral subtraction.

## Features

- **Real-time audio processing**: Captures microphone input and processes it in real-time
- **Echo cancellation**: Uses phase inversion to remove application audio from microphone input
- **Noise reduction**: Applies spectral subtraction to reduce background noise
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modern GUI**: Built with egui for a clean, responsive interface
- **Audio level monitoring**: Real-time input and output level visualization

## Architecture

### Audio Processing Pipeline

1. **Input Capture**: Captures audio from the default microphone
2. **Application Audio Capture**: Captures system audio output (loopback)
3. **Phase Inversion**: Subtracts inverted application audio from microphone input
4. **Noise Reduction**: Applies spectral subtraction in the frequency domain
5. **Output**: Sends processed audio to loopback for use in other applications

### Key Components

- `audio.rs`: Core audio processing logic using cpal for cross-platform audio I/O
- `ui.rs`: egui-based user interface with real-time controls and monitoring
- `main.rs`: Application entry point and initialization

## Dependencies

- **cpal**: Cross-platform audio library
- **rustfft**: Fast Fourier Transform for frequency domain processing
- **ringbuf**: Lock-free ring buffers for audio data
- **egui/eframe**: Immediate mode GUI framework
- **tokio**: Async runtime for audio processing tasks

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Usage

1. **Start the application**: Click the "Start" button to begin audio processing
2. **Configure settings**:
   - Toggle "Echo Cancellation" to enable/disable phase inversion
   - Toggle "Noise Reduction" to enable/disable spectral subtraction
3. **Monitor levels**: Watch the input and output audio level meters
4. **Stop processing**: Click "Stop" to halt audio processing

## Technical Details

### Echo Cancellation

The application implements echo cancellation by:
1. Capturing both microphone input and system audio output
2. Applying phase inversion (subtraction) to remove system audio from mic input
3. This prevents feedback loops when using the processed audio in communication apps

### Noise Reduction

Noise reduction uses spectral subtraction:
1. Transforms audio to frequency domain using FFT
2. Estimates noise floor and applies over-subtraction
3. Reconstructs clean audio using inverse FFT
4. Prevents over-subtraction artifacts by maintaining minimum signal levels

### Cross-Platform Audio

The application uses platform-specific audio backends:
- **Windows**: WASAPI through cpal
- **macOS**: CoreAudio through cpal
- **Linux**: ALSA/PulseAudio through cpal

## Limitations

- Application audio capture (loopback) is simplified in this implementation
- Real-world deployment would require platform-specific loopback implementations
- Noise reduction algorithm is basic - production systems would use more sophisticated methods
- No audio latency optimization - suitable for non-real-time applications

## Future Enhancements

- Platform-specific loopback audio capture
- Advanced noise reduction algorithms (Wiener filtering, deep learning-based)
- Audio device selection
- Latency optimization
- Configuration persistence
- Audio format customization

## License

This project is open source. Feel free to modify and distribute.