use clap::Parser;
use log::*;

pub mod app;
pub mod args;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);
    info!("Working directory: {:?}", std::env::current_dir());

    eframe::run_native(
        "Mini LeeBee",
        eframe::NativeOptions::default(),
        Box::new(|_| {
            let jack_adapter = jack_adapter::JackAdapter::new().unwrap();
            jack_adapter.auto_connect();
            let state = mini_leebee_state::State::new(jack_adapter);
            Box::new(app::App::new(args, state))
        }),
    )
    .unwrap();
    Ok(())
}
