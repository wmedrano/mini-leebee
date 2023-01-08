use clap::Parser;

/// The command line arguments for Mini LeeBee.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Name of the person to greet
    #[arg(short, long, default_value = "info")]
    pub log_level: log::LevelFilter,

    /// If true, ports will auto connect.
    #[arg(short, long, default_value = "true")]
    pub auto_connect: bool,
}
