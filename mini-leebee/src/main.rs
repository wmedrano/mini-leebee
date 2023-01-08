use clap::Parser;
use log::*;

pub mod args;

fn main() {
    let args = args::Arguments::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("{:?}", args);
}
