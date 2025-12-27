use reqwest::Url;

#[derive(Clone, PartialEq)]
pub struct CrossfadeState {
    pub prev_url: Url,
    pub curr_url: Url,
    pub fade_progress: f64,
}
