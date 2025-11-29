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

/// Meeting URL patterns for browser-based meetings
pub fn get_meeting_url_patterns() -> Vec<&'static str> {
    vec![
        // Google Meet
        // Host: https://meet.google.com/cih-fjjf-pfd?authuser=4&pageId=none&pli=1
        // Visitor: https://meet.google.com/cih-fjjf-pfd
        // Note: Only match URLs with a meeting code (path segment), not just the homepage
        "meet.google.com/",
        
        // Microsoft Teams (web)
        // Host: https://teams.live.com/v2/
        // Visitor: https://teams.live.com/light-meetings/launch?p=...
        "teams.live.com/v2/",
        "teams.live.com/light-meetings/launch",
        "teams.microsoft.com/_#/meet",
        "teams.microsoft.com/_#/conversations",
        
        // Zoom (web)
        "zoom.us/j/",
        "zoom.us/s/",
        "zoom.us/wc/",
        
        // Webex (web)
        // Host: https://web.webex.com/meetings?autosignin=...
        // Visitor in meeting: https://meet*.webex.com/wbxmjs/joinservice/...
        // Visitor after meeting: https://meet*.webex.com/webappng/...
        "web.webex.com/meetings",
        ".webex.com/wbxmjs/joinservice",  // Visitor in meeting (handles subdomains like meet1655.webex.com)
        ".webex.com/webappng",            // Visitor dashboard (after meeting)
        "webex.com/webapp",
        "webex.com/meet",
        "meetings.webex.com",
    ]
}

/// Validate if a string is a valid Google Meet meeting code
/// Meeting codes follow the pattern: lowercase letters separated by hyphens
/// Common formats: "abc-defg-hij" (3-4-3), "abc-def-ghi" (3-3-3), etc.
fn is_valid_google_meet_code(code: &str) -> bool {
    // Meeting codes must be non-empty
    if code.is_empty() {
        return false;
    }
    
    // Split by hyphens to get the segments
    let segments: Vec<&str> = code.split('-').collect();
    
    // Must have exactly 3 segments (standard Google Meet format)
    if segments.len() != 3 {
        return false;
    }
    
    // Each segment must:
    // 1. Be non-empty
    // 2. Contain only lowercase letters (a-z)
    // 3. Be between 2-5 characters (typical range: 3-4 chars per segment)
    for segment in &segments {
        if segment.is_empty() || segment.len() < 2 || segment.len() > 5 {
            return false;
        }
        
        // Must be all lowercase letters
        if !segment.chars().all(|c| c.is_ascii_lowercase()) {
            return false;
        }
    }
    
    // Total code length (excluding hyphens) should be reasonable
    // Typical: 9-11 characters (3-3-3 = 9, 3-4-3 = 10, 4-4-3 = 11)
    let total_chars: usize = segments.iter().map(|s| s.len()).sum();
    if total_chars < 8 || total_chars > 15 {
        return false;
    }
    
    true
}

/// Check if a URL matches any meeting pattern
pub fn is_meeting_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    
    // Special handling for Google Meet: must have a valid meeting code in the path
    if url_lower.contains("meet.google.com/") {
        // Extract the path after meet.google.com/
        if let Some(path_start) = url_lower.find("meet.google.com/") {
            let after_domain = &url_lower[path_start + "meet.google.com/".len()..];
            // Check if there's a path segment (meeting code) before query params or end of string
            let path_segment = after_domain.split('?').next().unwrap_or(after_domain)
                .split('#').next().unwrap_or(after_domain)
                .trim_end_matches('/');
            
            // Validate if it's a proper meeting code using robust algorithm
            return is_valid_google_meet_code(path_segment);
        }
        return false;
    }
    
    // For other services, use simple pattern matching
    get_meeting_url_patterns()
        .iter()
        .any(|&pattern| url_lower.contains(pattern))
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

/// Check if a process name matches any browser (pattern-based, fallback for all platforms)
pub fn is_browser_process_pattern(process_name: &str) -> bool {
    let process_lower = process_name.to_lowercase();
    get_browser_process_names()
        .iter()
        .any(|&browser| process_lower.contains(&browser.to_lowercase()))
}

/// Check if a process is a browser on macOS using bundle categories
/// Uses `mdls` to check if the app's bundle category includes browser categories
pub fn is_browser_process_macos(process_name: &str) -> Result<bool, DetectionError> {
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
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse mdls output: "kMDItemAppStoreCategoryType = "value"" or "kMDItemAppStoreCategoryType = (null)"
            let category_value = if let Some(equals_pos) = output_str.find('=') {
                let value_part = output_str[equals_pos + 1..].trim();
                // Remove quotes if present
                value_part.trim_matches('"').trim()
            } else {
                output_str.trim()
            };
            
            // Log the category so you can learn from it
            if category_value != "(null)" && !category_value.is_empty() {
                info!("Process '{}' category type: {}", process_name, category_value);
            } else {
                info!("Process '{}' category type: (null)", process_name);
            }
            
            // Check for browser-related categories
            // Common categories: "public.app-category.web-browsers", "public.app-category.productivity"
            let category_lower = category_value.to_lowercase();
            let browser_category_patterns = [
                "web-browser",
                "web-browsers",
                "browser",
            ];
            
            if category_value != "(null)" && !category_value.is_empty() {
                if browser_category_patterns.iter().any(|pattern| category_lower.contains(pattern)) {
                    return Ok(true);
                }
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
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse mdls output to extract just the value
            let category_name = if let Some(equals_pos) = output_str.find('=') {
                let value_part = output_str[equals_pos + 1..].trim();
                value_part.trim_matches('"').trim()
            } else {
                output_str.trim()
            };
            
            if category_name != "(null)" && !category_name.is_empty() {
                info!("Process '{}' category name: {}", process_name, category_name);
            } else {
                info!("Process '{}' category name: (null)", process_name);
            }
            
            let category_lower = category_name.to_lowercase();
            if category_name != "(null)" && !category_name.is_empty() {
                if category_lower.contains("browser") || category_lower.contains("web") {
                    return Ok(true);
                }
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
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse mdls output to extract just the value
            let bundle_id = if let Some(equals_pos) = output_str.find('=') {
                let value_part = output_str[equals_pos + 1..].trim();
                value_part.trim_matches('"').trim()
            } else {
                output_str.trim()
            };
            
            info!("Process '{}' bundle ID: {}", process_name, bundle_id);
            
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
    
    // Use macOS bundle categories
    for process in processes {
        if is_browser_process_macos(process)? {
            browsers.push(process.clone());
        }
    }
    
    Ok(browsers)
}

