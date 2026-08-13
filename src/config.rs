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
/// 
/// Meeting codes follow the pattern: lowercase alphanumeric segments separated by hyphens
/// - Format: exactly 3 segments (e.g., "cih-fjjf-pfd")
/// - Each segment: 2-5 characters, lowercase letters and digits
/// - Total length: 8-15 characters (excluding hyphens)
/// 
/// Examples of valid codes:
/// - "abc-def-ghi" (3-3-3 = 9 chars)
/// - "cih-fjjf-pfd" (3-4-3 = 10 chars)
/// - "abcd-efg-hij" (4-3-3 = 10 chars)
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
/// 
/// **Google Meet Detection:**
/// - Validates meeting code format (e.g., "cih-fjjf-pfd")
/// - Excludes non-meeting pages: `/landing`, `/new`, `/join`, empty paths
/// - Meeting codes: 3 segments, 2-5 chars each, lowercase alphanumeric, total 8-15 chars
/// 
/// **Other Services (Teams, Webex, Zoom web):**
/// - Uses pattern matching on URL strings
/// - Matches specific meeting URL patterns for each service
pub fn is_meeting_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    
    // Special handling for Google Meet: must have a valid meeting code in the path
    if url_lower.contains("meet.google.com/") {
        // Extract the path after meet.google.com/
        if let Some(path_start) = url_lower.find("meet.google.com/") {
            let after_domain = &url_lower[path_start + "meet.google.com/".len()..];
            // Extract path segment before query params (# or ?)
            let path_segment = after_domain.split('?').next().unwrap_or(after_domain)
                .split('#').next().unwrap_or(after_domain)
                .trim_end_matches('/');
            
            // Exclude non-meeting pages
            let excluded_paths = ["landing", "new", "join", ""];
            if excluded_paths.contains(&path_segment) {
                return false;
            }
            
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

/// Browser names that are short, common words. These are matched by exact
/// (case-insensitive) equality, because matching them as substrings
/// misclassifies unrelated system processes: "edge" appears in
/// "knowledge-agent", "arc" in "searchpartyd", "tor" in "distnoted".
fn get_browser_exact_names() -> Vec<&'static str> {
    vec!["arc", "edge", "epic", "opera", "tor", "brave", "vivaldi", "yandex"]
}

/// Distinctive fragments matched anywhere in a process name. These also catch
/// helper processes ("Google Chrome Helper (Renderer)",
/// "chrome_crashpad_handler"), which matters: helpers must be recognized as
/// browsers so Tier 1 never inspects their network traffic. A Chrome helper
/// holds ESTABLISHED connections to google.com, which would otherwise be read
/// as an active meeting.
fn get_browser_name_fragments() -> Vec<&'static str> {
    vec![
        // Chromium-based
        "google chrome",
        "chromium",
        "chrome",
        "microsoft edge",
        "msedge",
        "brave browser",
        "opera browser",
        "opera gx",
        // Firefox-based
        "firefox",
        "mozilla",
        "waterfox",
        "palemoon",
        "pale moon",
        "seamonkey",
        // WebKit / Safari
        "safari",
        "webkit",
        // Other
        "tor browser",
        "duckduckgo",
        "maxthon",
        "electron",
    ]
}

/// Check if a process is a browser.
///
/// This runs for every meeting-app candidate on every polling cycle, so it must
/// stay allocation-light and must not spawn subprocesses. A previous version
/// shelled out to `osascript` and `mdls` per process to read the app bundle's
/// category; that cost 5-9 seconds per detection cycle and fell back to this
/// same name matching whenever the lookup failed.
pub fn is_browser_process(process_name: &str) -> bool {
    let process_lower = process_name.trim().to_lowercase();

    if get_browser_exact_names()
        .iter()
        .any(|&browser| process_lower == browser)
    {
        return true;
    }

    get_browser_name_fragments()
        .iter()
        .any(|&fragment| process_lower.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_browsers_and_their_helper_processes() {
        // Helpers must be recognized too: Tier 1 skips browsers, and a Chrome
        // helper's ESTABLISHED connections to google.com would otherwise read
        // as an active meeting.
        for name in [
            "Google Chrome",
            "Google Chrome Helper",
            "Google Chrome Helper (Renderer)",
            "chrome_crashpad_handler",
            "Safari",
            "com.apple.WebKit.WebContent",
            "firefox",
            "Microsoft Edge",
            "msedge",
            "Arc",
            "Brave Browser",
        ] {
            assert!(is_browser_process(name), "{} should be a browser", name);
        }
    }

    /// Short browser names are matched exactly. Substring matching on them
    /// misclassified unrelated macOS system processes.
    #[test]
    fn does_not_misclassify_system_processes_as_browsers() {
        for name in [
            "knowledge-agent",   // contains "edge"
            "searchpartyd",      // contains "arc"
            "distnoted",         // contains "tor"
            "zoom.us",
            "ZoomCefHelper",
            "Microsoft Teams",
        ] {
            assert!(!is_browser_process(name), "{} should not be a browser", name);
        }
    }

    #[test]
    fn matches_native_meeting_apps() {
        assert!(is_meeting_process("zoom.us"));
        assert!(is_meeting_process("Microsoft Teams"));
        assert!(is_meeting_process("webexmta"));
        assert!(!is_meeting_process("Finder"));
    }

    #[test]
    fn validates_google_meet_codes() {
        assert!(is_meeting_url("https://meet.google.com/cih-fjjf-pfd"));
        assert!(is_meeting_url("https://meet.google.com/cih-fjjf-pfd?authuser=4&pli=1"));
        assert!(!is_meeting_url("https://meet.google.com/"));
        assert!(!is_meeting_url("https://meet.google.com/landing"));
        assert!(!is_meeting_url("https://meet.google.com/new"));
    }
}
