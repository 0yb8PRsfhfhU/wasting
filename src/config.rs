use std::path::Path;

use reqwest::Url;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig {
    /// The lambda of the exponential distribution.
    pub lambda: f64,

    #[serde(default)]
    /// The speed limit in bytes per second.
    pub speed_limit: Option<SpeedLimit>,

    /// The sources to download from.
    pub source: Vec<Source>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Source {
    pub url: Url,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SpeedLimit {
    Static(u64),
    Dynamic(Vec<DynamicSpeedLimitPoint>),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DynamicSpeedLimitPoint {
    pub time: time::Time,
    pub speed_limit: u64,
}

impl SpeedLimit {
    /// Resolve the current effective speed limit in bytes per second.
    ///
    /// For `Static`, returns the fixed value.
    /// For `Dynamic`, uses a step function: finds the most recent time point
    /// at or before the current local time-of-day. If the current time is
    /// before all points, wraps around to the last point (previous day's
    /// carry-over).
    pub fn resolve(&self) -> anyhow::Result<u64> {
        match self {
            Self::Static(limit) => Ok(*limit),
            Self::Dynamic(points) => resolve_dynamic(points),
        }
    }
}

fn resolve_dynamic(points: &[DynamicSpeedLimitPoint]) -> anyhow::Result<u64> {
    let now = time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let current_time = now.time();

    // Find the last point where point.time <= current_time (step function).
    // Points are iterated in order; we pick the last one that qualifies.
    let resolved = points
        .iter()
        .filter(|p| p.time <= current_time)
        .max_by_key(|p| p.time);

    // If none found (current time is before all points), wrap to the last
    // point by time (represents previous day's setting carrying over midnight).
    let point = resolved.or_else(|| points.iter().max_by_key(|p| p.time));

    point
        .map(|p| p.speed_limit)
        .ok_or_else(|| anyhow::anyhow!("dynamic speed limit has no points"))
}

/// Load and parse the service configuration from a TOML file.
/// Reused for both initial load and SIGHUP hot-reload.
pub fn load_config(path: &Path) -> anyhow::Result<ServiceConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read config file {}: {e}", path.display()))?;
    let config: ServiceConfig = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse config file {}: {e}", path.display()))?;

    anyhow::ensure!(!config.source.is_empty(), "config must have at least one source");
    anyhow::ensure!(config.lambda > 0.0, "lambda must be positive");

    Ok(config)
}
