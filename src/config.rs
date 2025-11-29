/// Configuration for meeting detection
/// This defines meeting apps and window title patterns

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

