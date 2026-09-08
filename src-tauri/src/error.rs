use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{0}")]
    Ocr(String),

    #[error("{0}")]
    Calendar(String),

    #[error("Calendar access is denied. Please enable Calendar access in Settings.")]
    CalendarPermissionDenied,

    #[error("Model '{0}' not found in manifest")]
    ModelNotFound(String),

    #[error("Insufficient disk space: {available_mb} MB free, but {required_mb} MB required.")]
    InsufficientDiskSpace { required_mb: u64, available_mb: u64 },

    #[error("{0}")]
    Download(String),

    #[error("SHA-256 checksum mismatch! Expected: {0}")]
    ChecksumMismatch(String),

    #[error("{0}")]
    Inference(String),

    #[error("Inference timed out after {0} seconds")]
    InferenceTimeout(u64),

    #[error("{0}")]
    Io(String),
}

impl AppError {
    pub fn contains(&self, pat: &str) -> bool {
        self.to_string().contains(pat)
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Download(err.to_string())
    }
}

impl From<std::ffi::NulError> for AppError {
    fn from(err: std::ffi::NulError) -> Self {
        AppError::Io(format!("Invalid null-terminated string: {}", err))
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(err: tokio::task::JoinError) -> Self {
        AppError::Inference(format!("Background task panicked: {}", err))
    }
}

impl PartialEq<str> for AppError {
    fn eq(&self, other: &str) -> bool {
        match self {
            AppError::Ocr(s)
            | AppError::Calendar(s)
            | AppError::Download(s)
            | AppError::Inference(s)
            | AppError::Io(s) => s.as_str() == other,
            AppError::CalendarPermissionDenied => {
                other == "Calendar access is denied. Please enable Calendar access in Settings."
            }
            AppError::ModelNotFound(id) => {
                let expected = format!("Model '{}' not found in manifest", id);
                expected == other
            }
            AppError::InsufficientDiskSpace {
                required_mb,
                available_mb,
            } => {
                let expected = format!(
                    "Insufficient disk space: {} MB free, but {} MB required.",
                    available_mb, required_mb
                );
                expected == other
            }
            AppError::ChecksumMismatch(s) => {
                let expected = format!("SHA-256 checksum mismatch! Expected: {}", s);
                expected == other
            }
            AppError::InferenceTimeout(secs) => {
                let expected = format!("Inference timed out after {} seconds", secs);
                expected == other
            }
        }
    }
}

impl PartialEq<&str> for AppError {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for AppError {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_and_contains() {
        let err = AppError::Ocr("vision failed".to_string());
        assert_eq!(err.to_string(), "vision failed");
        assert!(err.contains("vision"));
        assert_eq!(err, "vision failed");

        let err = AppError::Calendar("invalid event".to_string());
        assert_eq!(err.to_string(), "invalid event");
        assert_eq!(err, "invalid event");

        let err = AppError::CalendarPermissionDenied;
        assert!(err.to_string().contains("Calendar access is denied"));

        let err = AppError::ModelNotFound("smollm-test".to_string());
        assert_eq!(err.to_string(), "Model 'smollm-test' not found in manifest");
        assert!(err.contains("smollm-test"));

        let err = AppError::InsufficientDiskSpace {
            required_mb: 500,
            available_mb: 100,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient disk space: 100 MB free, but 500 MB required."
        );

        let err = AppError::Download("network error".to_string());
        assert_eq!(err.to_string(), "network error");

        let err = AppError::ChecksumMismatch("hash123".to_string());
        assert!(err.to_string().contains("SHA-256 checksum mismatch"));
        assert!(err.contains("hash123"));

        let err = AppError::Inference("inference failed".to_string());
        assert_eq!(err.to_string(), "inference failed");

        let err = AppError::InferenceTimeout(5);
        assert_eq!(err.to_string(), "Inference timed out after 5 seconds");

        let err = AppError::Io("file not found".to_string());
        assert_eq!(err.to_string(), "file not found");
    }

    #[test]
    fn test_error_serialize() {
        let err = AppError::Ocr("test ocr failure".to_string());
        let json = serde_json::to_string(&err).expect("serialize");
        assert_eq!(json, "\"test ocr failure\"");
    }

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
        assert!(app_err.contains("missing file"));

        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let app_err: AppError = json_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }
}
