mod api;
mod cli;
mod config;
mod history;

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;

use crate::cli::Cli;
use crate::config::DEFAULT_MODEL;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("askds: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Some(key) = cli.key.as_deref() {
        let key = key.trim();
        if key.is_empty() {
            bail!("the API key cannot be empty");
        }

        let mut saved = config::load()?;
        saved.api_key = Some(key.to_string());
        let path = config::save(&saved)?;
        eprintln!("Saved API key to {}", path.display());
        return Ok(());
    }

    let prompt = cli.prompt.join(" ");
    let prompt = prompt.trim();
    if prompt.is_empty() {
        eprintln!("askds: a question is required");
        eprintln!();
        eprintln!("Usage: askds <prompt>");
        eprintln!("       askds -s <prompt>");
        eprintln!("       askds --key <api key>");
        eprintln!();
        eprintln!("Run `askds --help` for more information.");
        std::process::exit(2);
    }

    let config = config::load()?;

    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .or_else(|| config.api_key.clone())
        .filter(|key| !key.is_empty());

    let Some(api_key) = api_key else {
        bail!(
            "no API key configured\n\nSet one with:\n  askds --key <api key>\n\n\
             or set the DEEPSEEK_API_KEY environment variable."
        );
    };

    let model = cli
        .model
        .clone()
        .or_else(|| config.model.clone())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let terminal_history = if cli.no_history {
        None
    } else {
        match history::capture() {
            Ok(history) if history.trim().is_empty() => None,
            Ok(history) => Some(history),
            Err(err) => {
                eprintln!("askds: warning: {err}; asking without terminal history");
                None
            }
        }
    };

    api::ask(&api_key, &model, prompt, terminal_history.as_deref())
}
