use std::net::SocketAddr;

use clap::Parser;
use log::*;

pub mod args;
pub mod grpc;

#[tokio::main]
async fn main() {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);

    let jack_adapter = jack_adapter::JackAdapter::new().unwrap();
    if args.auto_connect {
        jack_adapter.auto_connect();
    } else {
        warn!("--auto_connect is set to false. Will not automatically connect ports.");
    }
    let main_service = grpc::MiniLeebeeServer::new(jack_adapter);

    // The reflection service is useful for gRPC tools (like gRPC UI) to
    // interact with the server.
    let reflection_service = match tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(mini_leebee_proto::FILE_DESCRIPTOR_SET)
        .build()
    {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("Failed to initialize proto reflection service: {:?}", e);
            None
        }
    };

    let addr: SocketAddr = format!("[::1]:{}", args.port).parse().unwrap();
    info!("Starting server at {:?}.", addr);
    tonic::transport::Server::builder()
        .add_optional_service(reflection_service)
        .add_service(mini_leebee_proto::mini_leebee_server::MiniLeebeeServer::new(main_service))
        .serve(addr)
        .await
        .unwrap();
}
