use thiserror::Error;

#[derive(Debug, Error)]
pub enum AlexaError {
    #[error("Session expired. Run `alexa-cli auth login` to refresh.")]
    SessionExpired,

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited by Amazon. Please wait before retrying.")]
    RateLimited,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

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


#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn test_from_status_401_returns_session_expired() {
        let err = AlexaError::from_status(StatusCode::UNAUTHORIZED, "body");
        assert!(matches!(err, AlexaError::SessionExpired));
    }

    #[test]
    fn test_from_status_403_returns_session_expired() {
        let err = AlexaError::from_status(StatusCode::FORBIDDEN, "body");
        assert!(matches!(err, AlexaError::SessionExpired));
    }

    #[test]
    fn test_from_status_429_returns_rate_limited() {
        let err = AlexaError::from_status(StatusCode::TOO_MANY_REQUESTS, "body");
        assert!(matches!(err, AlexaError::RateLimited));
    }

    #[test]
    fn test_from_status_500_returns_api_error() {
        let err = AlexaError::from_status(StatusCode::INTERNAL_SERVER_ERROR, "server error");
        match err {
            AlexaError::ApiError { status, message } => {
                assert_eq!(status, 500);
                assert_eq!(message, "server error");
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[test]
    fn test_from_status_truncates_long_body() {
        let long_body = "x".repeat(500);
        let err = AlexaError::from_status(StatusCode::BAD_REQUEST, &long_body);
        match err {
            AlexaError::ApiError { message, .. } => {
                assert_eq!(message.len(), 200);
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            AlexaError::SessionExpired.to_string(),
            "Session expired. Run `alexa-cli auth login` to refresh."
        );
        assert_eq!(
            AlexaError::RateLimited.to_string(),
            "Rate limited by Amazon. Please wait before retrying."
        );
    }
}
