// Linux-specific implementation
// Uses PulseAudio for audio, X11/Wayland for windows, sysinfo for processes

use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use log::debug;
use sysinfo::System;

pub struct LinuxDetector {
    system: System,
}

impl LinuxDetector {
    pub fn new() -> Result<Self, DetectionError> {
        Ok(Self {
            system: System::new_all(),
        })
    }
}

impl PlatformDetector for LinuxDetector {
    fn is_microphone_active(&self) -> Result<bool, DetectionError> {
        // On Linux, use PulseAudio to check microphone state
        // For v1, simplified approach
        // TODO: Implement proper PulseAudio detection
        
        Ok(false)
    }

    fn is_camera_active(&self) -> Result<bool, DetectionError> {
        // On Linux, check if camera device is in use
        // Check /dev/video* devices or use v4l2
        // For v1, simplified: check for video-related processes
        let processes = self.get_running_processes()?;
        let camera_keywords = vec!["zoom", "teams", "skype", "v4l2"];
        
        let has_camera_app = processes.iter().any(|p| {
            let p_lower = p.to_lowercase();
            camera_keywords.iter().any(|keyword| p_lower.contains(keyword))
        });
        
        Ok(has_camera_app)
    }

    fn get_running_processes(&self) -> Result<Vec<String>, DetectionError> {
        let mut system = System::new_all();
        system.refresh_all();
        
        let mut processes = Vec::new();
        
        for (_, process) in system.processes() {
            processes.push(process.name().to_string());
        }
        
        debug!("Found {} running processes", processes.len());
        Ok(processes)
    }

    fn get_visible_windows(&self) -> Result<Vec<String>, DetectionError> {
        // On Linux, use xdotool or wmctrl to get window titles
        // Try xdotool first, fallback to wmctrl
        
        use std::process::Command;
        
        // Try xdotool
        let output = Command::new("xdotool")
            .arg("search")
            .arg("--onlyvisible")
            .arg("--name")
            .arg(".*")
            .output();
        
        if let Ok(output) = output {
            if output.status.success() {
                // Get window names for each window ID
                let window_ids = String::from_utf8_lossy(&output.stdout);
                let mut titles = Vec::new();
                
                for id in window_ids.lines() {
                    if let Ok(id_num) = id.trim().parse::<u64>() {
                        if let Ok(name_output) = Command::new("xdotool")
                            .arg("getwindowname")
                            .arg(id_num.to_string())
                            .output()
                        {
                            if let Ok(title) = String::from_utf8(name_output.stdout) {
                                let title = title.trim().to_string();
                                if !title.is_empty() {
                                    titles.push(title);
                                }
                            }
                        }
                    }
                }
                
                debug!("Found {} visible windows via xdotool", titles.len());
                return Ok(titles);
            }
        }
        
        // Fallback to wmctrl
        let output = Command::new("wmctrl")
            .arg("-l")
            .output()
            .map_err(|e| DetectionError::SystemError(format!("Failed to run wmctrl: {}", e)))?;
        
        if !output.status.success() {
            return Err(DetectionError::SystemError(
                "Failed to get window titles (xdotool and wmctrl both failed)".to_string(),
            ));
        }
        
        let output_str = String::from_utf8(output.stdout)
            .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
        
        // wmctrl output format: "0x01234567  0 desktop-name Window Title"
        let titles: Vec<String> = output_str
            .lines()
            .filter_map(|line| {
                // Skip to the window title (after the 3rd space-separated field)
                line.splitn(4, ' ')
                    .nth(3)
                    .map(|s| s.trim().to_string())
            })
            .filter(|s| !s.is_empty())
            .collect();
        
        debug!("Found {} visible windows via wmctrl", titles.len());
        Ok(titles)
    }
}

