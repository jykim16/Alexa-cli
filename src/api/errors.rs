use thiserror::Error;

#[derive(Debug, Error)]
pub enum AlexaError {
    #[error("Not authenticated. Run `alexa-cli auth login` first.")]
    NotAuthenticated,

    #[error("Session expired. Run `alexa-cli auth login` to refresh.")]
    SessionExpired,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited by Amazon. Please wait before retrying.")]
    RateLimited,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl AlexaError {
    pub fn from_status(status: reqwest::StatusCode, body: &str) -> Self {
        match status.as_u16() {
            401 | 403 => Self::SessionExpired,
            429 => Self::RateLimited,
            _ => Self::ApiError {
                status: status.as_u16(),
                message: body.chars().take(200).collect(),
            },
        }
    }
}
