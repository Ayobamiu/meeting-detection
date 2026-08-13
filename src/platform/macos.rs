// macOS-specific implementation
// Uses CoreAudio for microphone, system_profiler for camera, sysinfo for processes

use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use std::process::Command;
use std::sync::Mutex;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub struct MacOSDetector {
    /// Reused across cycles so each poll refreshes an existing snapshot instead
    /// of building a whole new one. Behind a Mutex because `PlatformDetector`
    /// takes `&self` and the engine polls from a background task.
    system: Mutex<System>,
}

impl MacOSDetector {
    pub fn new() -> Result<Self, DetectionError> {
        // Only process data is ever read, so skip the CPU/memory/disk/network
        // probes that `System::new_all` would perform on every refresh.
        let system = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        );

        Ok(Self {
            system: Mutex::new(system),
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

    /// Returns the set of running process names, deduplicated.
    ///
    /// Deduplication matters for the caller: Chrome alone contributes dozens of
    /// identically named helper processes, and the detector does per-name work
    /// for each entry it is handed.
    fn get_running_processes(&self) -> Result<Vec<String>, DetectionError> {
        let mut system = self
            .system
            .lock()
            .map_err(|e| DetectionError::SystemError(format!("Process list poisoned: {}", e)))?;

        system.refresh_processes_specifics(ProcessRefreshKind::new());

        let names: std::collections::HashSet<String> = system
            .processes()
            .values()
            .map(|process| process.name().to_string())
            .collect();

        Ok(names.into_iter().collect())
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
        
        Ok(titles)
    }
}

// Helper functions for browser and network detection (not part of trait yet)
// These can be called directly when needed

/// Check whether an app is in the running-process list (case-insensitive).
fn is_app_running(processes: &[String], app_name: &str) -> bool {
    processes
        .iter()
        .any(|process| process.eq_ignore_ascii_case(app_name))
}

/// Get browser tab URLs for meeting detection
/// Returns map of browser name -> list of URLs
/// This is exported from platform module for use in detector
///
/// Only browsers present in `processes` are queried. Two reasons: sending an
/// Apple Event to an app that is installed but not running *launches* it, so an
/// unguarded poll would start a browser on the user's machine every 2 seconds;
/// and each `osascript` spawn costs ~150ms that is pure waste for a browser
/// nobody is using.
pub fn get_browser_tab_urls(
    processes: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, DetectionError> {
    let mut browser_urls = std::collections::HashMap::new();

    let sources: [(&str, fn() -> Result<Vec<String>, DetectionError>); 3] = [
        ("Google Chrome", get_chrome_tab_urls),
        ("Safari", get_safari_tab_urls),
        ("Microsoft Edge", get_edge_tab_urls),
    ];

    for (browser_name, get_urls) in sources {
        if !is_app_running(processes, browser_name) {
            continue;
        }

        if let Ok(urls) = get_urls() {
            if !urls.is_empty() {
                browser_urls.insert(browser_name.to_string(), urls);
            }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_app_names_case_insensitively() {
        let processes = vec!["Google Chrome".to_string(), "Safari".to_string()];

        assert!(is_app_running(&processes, "Google Chrome"));
        assert!(is_app_running(&processes, "safari"));
        assert!(!is_app_running(&processes, "Microsoft Edge"));
    }

    /// Manual diagnostic against the live machine — browsers must actually be
    /// running for it to say anything, so it is not part of the CI suite.
    /// Run with: cargo test -- --ignored --nocapture
    ///
    /// Reports counts only. Tab URLs are the user's browsing history.
    #[test]
    #[ignore]
    fn probe_live_machine() {
        use crate::config::{is_browser_process, is_meeting_process};
        use crate::network::get_all_network_connections;

        let detector = MacOSDetector::new().unwrap();
        let processes = detector.get_running_processes().unwrap();
        println!("\nunique process names: {}", processes.len());

        let candidates: Vec<&String> = processes
            .iter()
            .filter(|name| is_meeting_process(name) && !is_browser_process(name))
            .collect();
        println!("tier 1 candidates: {:?}", candidates);

        let connections = get_all_network_connections().unwrap();
        println!("network connections parsed: {}", connections.len());

        for candidate in &candidates {
            let matched = connections
                .iter()
                .filter(|c| &&c.process_name == candidate)
                .count();
            println!("  {} -> {} connections matched by name", candidate, matched);
        }

        let tabs = get_browser_tab_urls(&processes).unwrap();
        for (browser, urls) in &tabs {
            println!("tier 2: {} -> {} tabs readable", browser, urls.len());
        }
        assert!(!processes.is_empty());
    }
}
