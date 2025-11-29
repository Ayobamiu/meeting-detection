/// Configuration for meeting detection
/// This defines meeting apps and window title patterns

use crate::error::DetectionError;

/// Known meeting application process names per platform
pub fn get_meeting_app_processes() -> Vec<&'static str> {
    vec![
        // Zoom
        "zoom",
        "zoom.us",
        "ZoomOpener",
        "zTsc",
        // Microsoft Teams
        "teams",
        "ms-teams",
        "Microsoft Teams",
        // Google Meet (runs in browser, but we check for Chrome/Edge with specific windows)
        "Google Chrome",
        "chrome",
        "Microsoft Edge",
        "msedge",
        // Webex
        "webexmta",
        "webex",
        "Cisco Webex",
        // Other common ones
        "skype",
        "Skype",
    ]
}

/// Window title patterns that indicate a meeting (fuzzy matching)
pub fn get_meeting_window_patterns() -> Vec<&'static str> {
    vec![
        "meeting",
        "call",
        "conference",
        "webex",
        "zoom",
        "teams",
        "google meet",
        "hangouts",
        "video call",
        "audio call",
    ]
}

/// Check if a process name matches any meeting app
pub fn is_meeting_process(process_name: &str) -> bool {
    let process_lower = process_name.to_lowercase();
    get_meeting_app_processes()
        .iter()
        .any(|&app| process_lower.contains(&app.to_lowercase()))
}

/// Check if a window title matches any meeting pattern (fuzzy, case-insensitive)
pub fn is_meeting_window(window_title: &str) -> bool {
    let title_lower = window_title.to_lowercase();
    get_meeting_window_patterns()
        .iter()
        .any(|&pattern| title_lower.contains(pattern))
}

/// Comprehensive list of browser process names (for non-macOS platforms)
pub fn get_browser_process_names() -> Vec<&'static str> {
    vec![
        // Chromium-based browsers
        "Google Chrome",
        "chrome",
        "Chromium",
        "chromium",
        "Microsoft Edge",
        "msedge",
        "Edge",
        "Brave Browser",
        "brave",
        "Brave",
        "Opera",
        "opera",
        "Opera Browser",
        "Vivaldi",
        "vivaldi",
        "Yandex",
        "yandex",
        "Arc",
        "arc",
        // Firefox-based
        "Firefox",
        "firefox",
        "Firefox Developer Edition",
        "Firefox Nightly",
        // Safari (also on macOS but included for completeness)
        "Safari",
        "safari",
        "Safari Technology Preview",
        // Other browsers
        "Tor Browser",
        "tor",
        "DuckDuckGo",
        "duckduckgo",
        "Epic Privacy Browser",
        "epic",
        "Maxthon",
        "maxthon",
        "Pale Moon",
        "palemoon",
        "Waterfox",
        "waterfox",
        "SeaMonkey",
        "seamonkey",
        // Electron-based browsers/apps that might host meetings
        "Electron",
        "electron",
    ]
}

/// Check if a process name matches any browser (pattern-based, for non-macOS)
pub fn is_browser_process_pattern(process_name: &str) -> bool {
    let process_lower = process_name.to_lowercase();
    get_browser_process_names()
        .iter()
        .any(|&browser| process_lower.contains(&browser.to_lowercase()))
}

/// Check if a process is a browser on macOS using bundle categories
/// Uses `mdls` to check if the app's bundle category includes browser categories
#[cfg(target_os = "macos")]
fn is_browser_process_macos(process_name: &str) -> Result<bool, DetectionError> {
    use log::info;
    use std::process::Command;
    
    // First, try to find the app bundle path using AppleScript
    let script = format!(
        r#"
        tell application "System Events"
            try
                set appProcess to first process whose name is "{}"
                set appPath to POSIX path of (file of appProcess as alias)
                return appPath
            on error
                return ""
            end try
        end tell
        "#,
        process_name
    );
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| DetectionError::SystemError(format!("Failed to run osascript: {}", e)))?;
    
    if !output.status.success() {
        // If we can't get the path, fall back to pattern matching
        return Ok(is_browser_process_pattern(process_name));
    }
    
    let app_path = String::from_utf8(output.stdout)
        .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?
        .trim()
        .to_string();
    
    if app_path.is_empty() {
        // Fall back to pattern matching if we can't find the app
        return Ok(is_browser_process_pattern(process_name));
    }
    
    // Try to get app store category type (most reliable for browsers)
    let category_output = Command::new("mdls")
        .arg("-name")
        .arg("kMDItemAppStoreCategoryType")
        .arg(&app_path)
        .output();
    
    if let Ok(output) = category_output {
        if output.status.success() {
            let category_str = String::from_utf8_lossy(&output.stdout);
            
            // Log the category so you can learn from it
            info!("Process '{}' category: {}", process_name, category_str.trim());
            
            // Check for browser-related categories
            // Common categories: "public.app-category.web-browsers", "public.app-category.productivity"
            let category_lower = category_str.to_lowercase();
            let browser_category_patterns = [
                "web-browser",
                "web-browsers",
                "browser",
            ];
            
            if browser_category_patterns.iter().any(|pattern| category_lower.contains(pattern)) {
                return Ok(true);
            }
        }
    }
    
    // Also check the human-readable category name
    let category_name_output = Command::new("mdls")
        .arg("-name")
        .arg("kMDItemAppStoreCategory")
        .arg(&app_path)
        .output();
    
    if let Ok(output) = category_name_output {
        if output.status.success() {
            let category_name = String::from_utf8_lossy(&output.stdout);
            info!("Process '{}' category name: {}", process_name, category_name.trim());
            
            let category_lower = category_name.to_lowercase();
            if category_lower.contains("browser") || category_lower.contains("web") {
                return Ok(true);
            }
        }
    }
    
    // Check bundle identifier as fallback
    let bundle_id_output = Command::new("mdls")
        .arg("-name")
        .arg("kMDItemCFBundleIdentifier")
        .arg(&app_path)
        .output();
    
    if let Ok(output) = bundle_id_output {
        if output.status.success() {
            let bundle_id = String::from_utf8_lossy(&output.stdout);
            info!("Process '{}' bundle ID: {}", process_name, bundle_id.trim());
            
            let bundle_id_lower = bundle_id.to_lowercase();
            let browser_bundle_ids = [
                "com.google.chrome",
                "com.microsoft.edgemac",
                "com.brave.browser",
                "com.operasoftware.opera",
                "com.vivaldi.vivaldi",
                "org.mozilla.firefox",
                "com.apple.safari",
                "org.torproject.torbrowser",
                "com.duckduckgo.mac.browser",
                "com.epicbrowser.epic",
            ];
            
            if browser_bundle_ids.iter().any(|id| bundle_id_lower.contains(id)) {
                return Ok(true);
            }
        }
    }
    
    // Fall back to pattern matching if bundle detection fails
    Ok(is_browser_process_pattern(process_name))
}

/// Filter processes to only browsers
/// Uses macOS bundle categories on macOS, pattern matching on other platforms
pub fn filter_browser_processes(
    processes: &[String],
) -> Result<Vec<String>, DetectionError> {
    let mut browsers = Vec::new();
    
    #[cfg(target_os = "macos")]
    {
        // Use macOS bundle categories
        for process in processes {
            if is_browser_process_macos(process)? {
                browsers.push(process.clone());
            }
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        // Use pattern matching for Windows/Linux
        for process in processes {
            if is_browser_process_pattern(process) {
                browsers.push(process.clone());
            }
        }
    }
    
    Ok(browsers)
}

