// Platform abstraction layer for macOS

mod macos;

use crate::error::DetectionError;

/// Platform-specific implementation trait for macOS
pub trait PlatformDetector: Send + Sync {
    /// Check if microphone is currently in use
    fn is_microphone_active(&self) -> Result<bool, DetectionError>;
    
    /// Check if camera is currently in use
    fn is_camera_active(&self) -> Result<bool, DetectionError>;
    
    /// Get list of running process names
    fn get_running_processes(&self) -> Result<Vec<String>, DetectionError>;
    
    /// Get list of visible window titles
    fn get_visible_windows(&self) -> Result<Vec<String>, DetectionError>;
}

/// Create the macOS platform detector
pub fn create_platform_detector() -> Result<Box<dyn PlatformDetector>, DetectionError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSDetector::new()?))
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Err(DetectionError::PlatformNotSupported(
            format!("This library only supports macOS. Current OS: {}", std::env::consts::OS),
        ))
    }
}

// Re-export platform modules for testing (if needed)
// #[cfg(test)]
// pub use macos::MacOSDetector;

// Export platform-specific helper functions
pub use macos::get_browser_tab_urls;
pub use macos::is_browser_process;

