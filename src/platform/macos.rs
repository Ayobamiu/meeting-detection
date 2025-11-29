// macOS-specific implementation
// Uses CoreAudio for microphone, system_profiler for camera, sysinfo for processes

use crate::config::is_browser_process_macos;
use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use log::debug;
use std::process::Command;
use sysinfo::System;

pub struct MacOSDetector {
    system: System,
}

impl MacOSDetector {
    pub fn new() -> Result<Self, DetectionError> {
        Ok(Self {
            system: System::new_all(),
        })
    }
}

impl PlatformDetector for MacOSDetector {
    fn is_microphone_active(&self) -> Result<bool, DetectionError> {
        // On macOS, we can use CoreAudio to check if any input device is active
        // For v1, we'll use a simpler approach: check if any process is using audio
        // This is a simplified check - in production you might want more sophisticated detection
        
        // Try to detect using system_profiler or check audio device state
        // For now, we'll use a heuristic: if certain audio-related processes are active
        // This is a simplified v1 approach
        
        // Alternative: Use CoreAudio to enumerate devices and check if input is active
        // For v1 simplicity, we'll return false and let other signals handle it
        // TODO: Implement proper CoreAudio detection
        
        // For now, return false (will be improved in future versions)
        // This allows the system to work but mic detection will need proper implementation
        Ok(false)
    }

    fn is_camera_active(&self) -> Result<bool, DetectionError> {
        // On macOS, check if camera is in use
        // We can use system_profiler or check for processes using camera
        
        // For v1, we'll check if any process has camera access
        // This is a simplified approach
        
        // Use lsof to check if any process has /dev/video* open
        // Or check system_profiler SPCameraDataType
        
        // For now, simplified: check if common video apps are running
        // This is not perfect but works for v1
        let processes = self.get_running_processes()?;
        let camera_keywords = vec!["zoom", "teams", "facetime", "photo booth", "quicktime"];
        
        let has_camera_app = processes.iter().any(|p| {
            let p_lower = p.to_lowercase();
            camera_keywords.iter().any(|keyword| p_lower.contains(keyword))
        });
        
        Ok(has_camera_app)
    }

    fn get_running_processes(&self) -> Result<Vec<String>, DetectionError> {
        // Refresh system info
        let mut system = System::new_all();
        system.refresh_all();
        
        let mut processes = Vec::new();
        
        for (_, process) in system.processes() {
            // Get process name
            processes.push(process.name().to_string());
        }
        
        debug!("Found {} running processes", processes.len());
        Ok(processes)
    }

    fn get_visible_windows(&self) -> Result<Vec<String>, DetectionError> {
        // On macOS, getting window titles requires using AppKit/CoreGraphics
        // For v1, we'll use a simpler approach with osascript
        
        use std::process::Command;
        
        // Use AppleScript to get window titles
        // Fixed: Use text item delimiters to properly format the list
        let script = r#"
            tell application "System Events"
                set windowList to {}
                repeat with proc in processes
                    try
                        set windowTitles to name of windows of proc
                        repeat with title in windowTitles
                            set end of windowList to title
                        end repeat
                    end try
                end repeat
                set AppleScript's text item delimiters to ", "
                set resultString to windowList as string
                set AppleScript's text item delimiters to ""
                return resultString
            end tell
        "#;
        
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| DetectionError::SystemError(format!("Failed to run osascript: {}", e)))?;
        
        if !output.status.success() {
            return Err(DetectionError::SystemError(
                "Failed to get window titles via AppleScript".to_string(),
            ));
        }
        
        let titles_str = String::from_utf8(output.stdout)
            .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
        
        // Parse the AppleScript output (now properly comma-separated)
        let titles: Vec<String> = titles_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("item 1 of")) // Filter out malformed entries
            .collect();
        
        debug!("Found {} visible windows", titles.len());
        Ok(titles)
    }
}

// Helper functions for browser and network detection (not part of trait yet)
// These can be called directly when needed

/// Check if a process is a browser using macOS app categories
pub fn is_browser_process(process_name: &str) -> Result<bool, DetectionError> {
    is_browser_process_macos(process_name)
}

/// Get browser tab URLs for meeting detection
/// Returns map of browser name -> list of URLs
/// This is exported from platform module for use in detector
pub fn get_browser_tab_urls() -> Result<std::collections::HashMap<String, Vec<String>>, DetectionError> {
    use log::info;
    let mut browser_urls = std::collections::HashMap::new();
    
    // Get URLs from Chrome
    if let Ok(chrome_urls) = get_chrome_tab_urls() {
        if !chrome_urls.is_empty() {
            info!("Chrome tabs ({}): {:?}", chrome_urls.len(), chrome_urls);
            browser_urls.insert("Google Chrome".to_string(), chrome_urls);
        }
    }
    
    // Get URLs from Safari
    if let Ok(safari_urls) = get_safari_tab_urls() {
        if !safari_urls.is_empty() {
            info!("Safari tabs ({}): {:?}", safari_urls.len(), safari_urls);
            browser_urls.insert("Safari".to_string(), safari_urls);
        }
    }
    
    // Get URLs from Edge
    if let Ok(edge_urls) = get_edge_tab_urls() {
        if !edge_urls.is_empty() {
            info!("Edge tabs ({}): {:?}", edge_urls.len(), edge_urls);
            browser_urls.insert("Microsoft Edge".to_string(), edge_urls);
        }
    }
    
    Ok(browser_urls)
}

/// Get URLs from all Chrome tabs
fn get_chrome_tab_urls() -> Result<Vec<String>, DetectionError> {
    let script = r#"
        tell application "Google Chrome"
            set urlList to {}
            repeat with w in windows
                repeat with t in tabs of w
                    set end of urlList to URL of t
                end repeat
            end repeat
            set AppleScript's text item delimiters to ", "
            set resultString to urlList as string
            set AppleScript's text item delimiters to ""
            return resultString
        end tell
    "#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let urls_str = String::from_utf8(output.stdout)
                .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
            
            // Parse comma-separated URLs
            let urls: Vec<String> = urls_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            Ok(urls)
        }
        _ => Ok(Vec::new()), // Chrome not running or error - return empty
    }
}

/// Get URLs from all Safari tabs
fn get_safari_tab_urls() -> Result<Vec<String>, DetectionError> {
    let script = r#"
        tell application "Safari"
            set urlList to {}
            repeat with w in windows
                repeat with t in tabs of w
                    set end of urlList to URL of t
                end repeat
            end repeat
            set AppleScript's text item delimiters to ", "
            set resultString to urlList as string
            set AppleScript's text item delimiters to ""
            return resultString
        end tell
    "#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let urls_str = String::from_utf8(output.stdout)
                .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
            
            // Parse comma-separated URLs
            let urls: Vec<String> = urls_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            Ok(urls)
        }
        _ => Ok(Vec::new()), // Safari not running or error - return empty
    }
}

/// Get URLs from all Edge tabs
fn get_edge_tab_urls() -> Result<Vec<String>, DetectionError> {
    let script = r#"
        tell application "Microsoft Edge"
            set urlList to {}
            repeat with w in windows
                repeat with t in tabs of w
                    set end of urlList to URL of t
                end repeat
            end repeat
            set AppleScript's text item delimiters to ", "
            set resultString to urlList as string
            set AppleScript's text item delimiters to ""
            return resultString
        end tell
    "#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let urls_str = String::from_utf8(output.stdout)
                .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
            
            // Parse comma-separated URLs
            let urls: Vec<String> = urls_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            Ok(urls)
        }
        _ => Ok(Vec::new()), // Edge not running or error - return empty
    }
}

