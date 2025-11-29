use thiserror::Error;

/// Custom error types for the meeting detection engine
#[derive(Error, Debug)]
pub enum DetectionError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    
    #[error("System error: {0}")]
    SystemError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl DetectionError {
    /// Check if this is a permission error (for graceful handling)
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, DetectionError::PermissionDenied(_))
    }
}

