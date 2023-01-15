use clap::Parser;
use log::*;

pub mod args;

fn main() {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);

    let audio_engine = jack_adapter::JackAdapter::new().unwrap();
    if args.auto_connect {
        audio_engine.auto_connect();
    } else {
        warn!("--auto_connect is set to false. Will not automatically connect ports.");
    }
    std::thread::park();
}
