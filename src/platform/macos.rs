// macOS-specific implementation
// Uses CoreAudio for microphone, system_profiler for camera, sysinfo for processes

use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use log::debug;
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
                return windowList
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
        
        // Parse the AppleScript output (it returns a comma-separated list)
        let titles: Vec<String> = titles_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        debug!("Found {} visible windows", titles.len());
        Ok(titles)
    }
}

