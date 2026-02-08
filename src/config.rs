use reqwest::Url;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig {
    /// The lambda of the exponential distribution.
    pub lambda: f64,
    /// The speed limit in bytes per second.
    pub speed_limit: u64,
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
