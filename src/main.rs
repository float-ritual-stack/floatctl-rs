mod cli;
mod config;
mod filters;
mod input;
mod model;
mod render_json;
mod render_md;
mod slug;
mod state;
mod util;

use anyhow::Context;
use clap::Parser;
use cli::Cli;
use config::Config;
use util::{AppError, expand_path, init_logger};

fn main() -> std::process::ExitCode {
    init_logger();

    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            err.exit_code()
        }
    }
}

fn run() -> Result<std::process::ExitCode, AppError> {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .as_ref()
        .map(|p| expand_path(p))
        .transpose()
        .context("failed to resolve config path")
        .map_err(AppError::io)?;

    let mut config = Config::load(config_path.as_deref())
        .context("failed to load configuration")
        .map_err(AppError::config)?;

    config.apply_cli(&cli).map_err(AppError::config)?;

    let outcome = util::execute(config, cli.force, cli.dry_run)?;
    Ok(outcome.into_exit_code())
}
