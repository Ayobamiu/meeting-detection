// Platform abstraction layer
// This trait defines what each platform must implement

mod macos;
mod windows;
mod linux;

use crate::error::DetectionError;

/// Platform-specific implementation trait
/// Each platform (macOS, Windows, Linux) implements this trait
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

/// Create the appropriate platform detector based on the current OS
pub fn create_platform_detector() -> Result<Box<dyn PlatformDetector>, DetectionError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSDetector::new()?))
    }
    
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsDetector::new()?))
    }
    
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxDetector::new()?))
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(DetectionError::PlatformNotSupported(
            std::env::consts::OS.to_string(),
        ))
    }
}

// Re-export platform modules for testing
#[cfg(test)]
pub use macos::MacOSDetector;
#[cfg(test)]
pub use windows::WindowsDetector;
#[cfg(test)]
pub use linux::LinuxDetector;

