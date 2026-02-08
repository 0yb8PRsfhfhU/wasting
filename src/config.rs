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
