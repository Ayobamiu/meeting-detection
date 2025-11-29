// Windows-specific implementation
// Uses WASAPI for audio, WinAPI for windows, sysinfo for processes

use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use log::debug;
use sysinfo::System;

pub struct WindowsDetector {
    system: System,
}

impl WindowsDetector {
    pub fn new() -> Result<Self, DetectionError> {
        Ok(Self {
            system: System::new_all(),
        })
    }
}

impl PlatformDetector for WindowsDetector {
    fn is_microphone_active(&self) -> Result<bool, DetectionError> {
        // On Windows, use WASAPI to check microphone state
        // For v1, simplified approach
        // TODO: Implement proper WASAPI detection
        
        // For now, return false (will be improved)
        Ok(false)
    }

    fn is_camera_active(&self) -> Result<bool, DetectionError> {
        // On Windows, check camera usage
        // For v1, check if video-related processes are running
        let processes = self.get_running_processes()?;
        let camera_keywords = vec!["zoom", "teams", "camera", "skype"];
        
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
        // On Windows, use WinAPI to enumerate windows
        // For v1, we'll use a PowerShell script as a simpler approach
        
        use std::process::Command;
        
        let script = r#"
            Add-Type @"
                using System;
                using System.Runtime.InteropServices;
                using System.Text;
                public class Win32 {
                    [DllImport("user32.dll")]
                    public static extern int EnumWindows(EnumWindowsProc enumProc, IntPtr lParam);
                    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
                    [DllImport("user32.dll")]
                    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
                    [DllImport("user32.dll")]
                    public static extern bool IsWindowVisible(IntPtr hWnd);
                }
"@
            $titles = @()
            [Win32]::EnumWindows([Win32+EnumWindowsProc]{
                param($hWnd, $lParam)
                if ([Win32]::IsWindowVisible($hWnd)) {
                    $sb = New-Object System.Text.StringBuilder 256
                    [Win32]::GetWindowText($hWnd, $sb, $sb.Capacity) | Out-Null
                    $title = $sb.ToString()
                    if ($title) { $titles += $title }
                }
                return $true
            }, 0)
            $titles -join ","
        "#;
        
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(script)
            .output()
            .map_err(|e| DetectionError::SystemError(format!("Failed to run PowerShell: {}", e)))?;
        
        if !output.status.success() {
            return Err(DetectionError::SystemError(
                "Failed to get window titles via PowerShell".to_string(),
            ));
        }
        
        let titles_str = String::from_utf8(output.stdout)
            .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
        
        let titles: Vec<String> = titles_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        debug!("Found {} visible windows", titles.len());
        Ok(titles)
    }
}

