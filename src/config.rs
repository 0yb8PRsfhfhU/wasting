use reqwest::Url;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig {
    pub lambda: f64,
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
