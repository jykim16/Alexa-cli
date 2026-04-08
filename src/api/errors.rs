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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_status_401_returns_session_expired() {
        let status = reqwest::StatusCode::UNAUTHORIZED; // 401
        let err = AlexaError::from_status(status, "unauthorized");
        assert!(matches!(err, AlexaError::SessionExpired));
    }

    #[test]
    fn test_from_status_403_returns_session_expired() {
        let status = reqwest::StatusCode::FORBIDDEN; // 403
        let err = AlexaError::from_status(status, "forbidden");
        assert!(matches!(err, AlexaError::SessionExpired));
    }

    #[test]
    fn test_from_status_429_returns_rate_limited() {
        let status = reqwest::StatusCode::TOO_MANY_REQUESTS; // 429
        let err = AlexaError::from_status(status, "too many requests");
        assert!(matches!(err, AlexaError::RateLimited));
    }

    #[test]
    fn test_from_status_500_returns_api_error() {
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR; // 500
        let err = AlexaError::from_status(status, "internal server error");
        assert!(matches!(err, AlexaError::ApiError { status: 500, .. }));
        if let AlexaError::ApiError { status, message } = err {
            assert_eq!(status, 500);
            assert!(message.contains("internal server error"));
        }
    }

    #[test]
    fn test_from_status_404_returns_api_error() {
        let status = reqwest::StatusCode::NOT_FOUND; // 404
        let err = AlexaError::from_status(status, "not found");
        assert!(matches!(err, AlexaError::ApiError { status: 404, .. }));
        if let AlexaError::ApiError { status, message } = err {
            assert_eq!(status, 404);
            assert!(message.contains("not found"));
        }
    }

    #[test]
    fn test_from_status_message_truncated_to_200_chars() {
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let long_body: String = "x".repeat(300);
        let err = AlexaError::from_status(status, &long_body);
        if let AlexaError::ApiError { message, .. } = err {
            assert_eq!(message.len(), 200);
        } else {
            panic!("expected ApiError variant");
        }
    }

    #[test]
    fn test_display_session_expired() {
        let err = AlexaError::SessionExpired;
        let s = err.to_string();
        assert!(s.contains("Session expired") || s.contains("auth login"));
    }

    #[test]
    fn test_display_not_authenticated() {
        let err = AlexaError::NotAuthenticated;
        let s = err.to_string();
        assert!(s.contains("Not authenticated") || s.contains("auth login"));
    }

    #[test]
    fn test_display_device_not_found() {
        let err = AlexaError::DeviceNotFound("my-echo".to_string());
        let s = err.to_string();
        assert!(s.contains("my-echo"));
    }

    #[test]
    fn test_display_rate_limited() {
        let err = AlexaError::RateLimited;
        let s = err.to_string();
        assert!(s.contains("Rate limited") || s.contains("wait"));
    }

    #[test]
    fn test_display_api_error() {
        let err = AlexaError::ApiError {
            status: 503,
            message: "service unavailable".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("503"));
        assert!(s.contains("service unavailable"));
    }

    #[test]
    fn test_display_other() {
        let err = AlexaError::Other("something went wrong".to_string());
        let s = err.to_string();
        assert!(s.contains("something went wrong"));
    }
}
