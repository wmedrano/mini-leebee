use clap::Parser;
use log::*;
use mini_leebee_proto::mini_leebee_client::MiniLeebeeClient;

pub mod app;
pub mod args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);
    info!("Working directory: {:?}", std::env::current_dir());

    let addr = format!("http://[::1]:{}", args.port);
    info!("Connecting to {}", addr);
    let client = MiniLeebeeClient::connect(addr).await?;
    eframe::run_native(
        "Mini LeeBee",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(app::App::new(client))),
    );
    Ok(())
}
