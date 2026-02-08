#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::PathBuf;
use std::time::Duration;

use rand::Rng;
use rand::distr::weighted::WeightedIndex;
use rand_distr::Exp;

pub(crate) mod config;
pub(crate) mod download;

use config::{ServiceConfig, load_config};

const MIN_SWITCH_SECS: f64 = 10.0;
const MAX_SWITCH_SECS: f64 = 604_800.0; // 1 week

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_path = parse_args()?;
    let mut config = load_config(&config_path)?;
    tracing::info!(?config_path, sources = config.source.len(), "config loaded");

    let client = reqwest::Client::new();

    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

    loop {
        let source = pick_source(&config)?;
        let switch_secs = roll_switch_duration(config.lambda)?;
        tracing::info!(
            url = %source.url,
            switch_secs,
            "downloading (will switch in {switch_secs:.1}s)"
        );

        tokio::select! {
            result = download::download_stream(&client, &source.url, &config.speed_limit) => {
                match result {
                    Ok(()) => tracing::info!("download completed, switching source"),
                    Err(e) => tracing::warn!("download error: {e:#}, switching source"),
                }
            }
            _ = tokio::time::sleep(Duration::from_secs_f64(switch_secs)) => {
                tracing::info!("switch timer expired, switching source");
            }
            _ = sighup.recv() => {
                match load_config(&config_path) {
                    Ok(new_config) => {
                        config = new_config;
                        tracing::info!("config reloaded via SIGHUP");
                    }
                    Err(e) => {
                        tracing::error!("failed to reload config: {e:#}, keeping old config");
                    }
                }
            }
        }
    }
}

/// Parse command-line arguments. Supports `-c <path>` to set the config file.
/// Defaults to `config.toml` in the current directory.
fn parse_args() -> anyhow::Result<PathBuf> {
    let mut args = std::env::args().skip(1);
    let mut config_path = PathBuf::from("config.toml");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("-c requires a path argument"))?;
                config_path = PathBuf::from(path);
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    Ok(config_path)
}

/// Pick a random source weighted by `source.weight`.
fn pick_source(config: &ServiceConfig) -> anyhow::Result<&config::Source> {
    let weights: Vec<f64> = config.source.iter().map(|s| s.weight).collect();
    let dist =
        WeightedIndex::new(&weights).map_err(|e| anyhow::anyhow!("invalid source weights: {e}"))?;
    let mut rng = rand::rng();
    let idx = rng.sample(dist);
    // Safety: idx is always in bounds since WeightedIndex produces valid indices.
    config
        .source
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("source index out of bounds"))
}

/// Roll a switch duration from an exponential distribution, clamped to [10s, 1 week].
fn roll_switch_duration(lambda: f64) -> anyhow::Result<f64> {
    let exp = Exp::new(lambda).map_err(|e| anyhow::anyhow!("invalid lambda: {e}"))?;
    let mut rng = rand::rng();
    let value: f64 = rng.sample(exp);
    Ok(value.clamp(MIN_SWITCH_SECS, MAX_SWITCH_SECS))
}
