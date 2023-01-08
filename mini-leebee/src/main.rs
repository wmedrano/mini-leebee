use clap::Parser;
use eframe::egui;
use log::*;

pub mod app;
pub mod args;

fn main() {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);
    let options = eframe::NativeOptions {
        // The target is a small screen with 320x240 resolution.
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    let audio_engine = audio_engine::AudioEngine::new().unwrap();
    if args.auto_connect {
        audio_engine.auto_connect();
    } else {
        warn!("--auto_connect is set to false. Will not automatically connect ports.");
    }

    eframe::run_native(
        "Mini LeeBee",
        options,
        Box::new(|_cc| Box::new(app::App::new(audio_engine))),
    )
}
