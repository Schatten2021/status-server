#![doc=include_str!("../README.md")]


#![cfg_attr(not(debug_assertions), deny(missing_docs))]
#![cfg_attr(debug_assertions, warn(missing_docs))]
#![warn(clippy::pedantic)]
#![warn(clippy::complexity, clippy::suspicious, clippy::perf, clippy::style, clippy::allow_attributes_without_reason)]
#![allow(
    clippy::needless_continue,
    reason = "adding a `continue` often makes the code easier to read."
)]
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    reason = "don't want these lints."
)]
#![cfg_attr(not(debug_assertions), deny(clippy::undocumented_unsafe_blocks))]
#![cfg_attr(debug_assertions, warn(clippy::undocumented_unsafe_blocks))]

mod start_server;
mod config_check;
#[cfg(feature = "auth")]
mod auth_stuff;

#[macro_use]
extern crate tracing;

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing::level_filters::LevelFilter;

#[cfg(debug_assertions)]
const LEVEL: LevelFilter = LevelFilter::TRACE;
#[cfg(not(debug_assertions))]
const LEVEL: LevelFilter = LevelFilter::INFO;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_max_level(LEVEL)
        .init();

    let args = Args::parse();
    debug!("parsed args: {args:?}");
    match args.command {
        Command::Run { host, port } => start_server::start(args.config_file, &host, port),
        Command::CheckConfig => {
            if let Err(()) = config_check::check(&args.config_file) {
                return ExitCode::FAILURE;
            }
        }
        Command::Auth(command) => if let Err(()) = command.command.run() {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

#[derive(clap::Parser, Debug)]
#[command(version, about="a custom status server")]
struct Args {
    /// The path of the config file
    #[arg(short, long, alias="config", default_value="config.toml", global=true)]
    config_file: PathBuf,

    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    #[clap(alias="serve")]
    /// runs the server.
    Run {
        /// The host (*excluding port*) to bind to.
        #[arg(short='b', long="bind", alias="host", default_value="0.0.0.0")]
        host: String,

        /// The port to bind to.
        #[arg(short, long, default_value_t=5000)]
        port: u16,
    },
    #[clap(alias="config-test", alias="config-check", alias="test-config")]
    /// checks that the current config actually works.
    CheckConfig,

    #[cfg(feature = "auth")]
    /// various commands relating to authentication, including generating new passwords.
    Auth(AuthParams),
}
#[cfg(feature = "auth")]
#[derive(clap::Args, Debug)]
struct AuthParams {
    #[clap(subcommand)]
    command: auth_stuff::Commands
}