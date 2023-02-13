use clap::Parser;

/// The command line arguments for Mini LeeBee.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Name of the person to greet
    #[arg(short, long, default_value = "info")]
    pub log_level: log::LevelFilter,

    /// If true, profiling will be enabled.
    #[arg(short, long, default_value = "false")]
    pub enable_profiling: bool,
}
