mod audio;
mod ui;

use eframe::egui;
use tracing_subscriber;
use ui::CancelCasterApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure native options for the GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0])
            .with_min_inner_size([350.0, 500.0])
            .with_resizable(true),
        ..Default::default()
    };

    // Run the GUI application
    eframe::run_native(
        "CancelCaster",
        options,
        Box::new(|cc| {
            Box::new(CancelCasterApp::new(cc).unwrap_or_else(|e| {
                eprintln!("Failed to create application: {}", e);
                std::process::exit(1);
            }))
        }),
    )?;

    Ok(())
}
