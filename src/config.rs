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

/// The parts of a URL that meeting matching needs.
///
/// Matching runs against the host and the route separately. Testing patterns
/// against the raw URL string, as this used to, reports a meeting for any page
/// that merely *mentions* a meeting domain — a search results page, a link in a
/// forum thread, a docs page about Teams.
struct ParsedUrl {
    /// Lowercased host, with userinfo and port removed.
    host: String,
    /// Path, plus the fragment when present. Teams is a hash-router app, so the
    /// part that identifies a meeting lives in the fragment:
    /// `https://teams.microsoft.com/_#/l/meetup-join/19:meeting_...` has path
    /// `/_` and fragment `/l/meetup-join/19:meeting_...`.
    ///
    /// Always ends in `/` so that prefix tests can require a segment boundary:
    /// `/meet/` must not match the route `/meetings` (the calendar).
    route: String,
    /// Query string without the leading `?`, empty when absent.
    query: String,
}

/// Parse an absolute http(s) URL into the pieces used for matching.
///
/// Returns `None` for anything that is not http(s), which drops `about:`,
/// `file:`, and extension pages before they reach any pattern.
fn parse_url(url: &str) -> Option<ParsedUrl> {
    let url = url.trim().to_lowercase();

    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    // Authority ends at the first `/`, `?`, or `#`.
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let (authority, rest) = after_scheme.split_at(authority_end);

    // Strip userinfo. Without this, `https://meet.google.com@evil.com/abc-def-ghi`
    // would look like it is hosted on meet.google.com.
    let host_port = match authority.rfind('@') {
        Some(at) => &authority[at + 1..],
        None => authority,
    };

    // Strip the port. IPv6 literals keep their brackets and simply match no
    // meeting host.
    let host = match host_port.rfind(':') {
        Some(colon) if !host_port.ends_with(']') => &host_port[..colon],
        _ => host_port,
    };

    if host.is_empty() {
        return None;
    }

    let (before_fragment, fragment) = match rest.find('#') {
        Some(hash) => (&rest[..hash], &rest[hash + 1..]),
        None => (rest, ""),
    };

    // Split `path?query` on both sides of the `#`. A hash-router app carries
    // its parameters inside the fragment (`/v2/#/route?meetingjoin=true`), so
    // both halves have to be collected or those parameters are invisible.
    fn split_query(part: &str) -> (&str, &str) {
        match part.find('?') {
            Some(mark) => (&part[..mark], &part[mark + 1..]),
            None => (part, ""),
        }
    }

    let (path, path_query) = split_query(before_fragment);
    let (fragment_path, fragment_query) = split_query(fragment);

    let mut route = String::with_capacity(path.len() + fragment_path.len() + 2);
    route.push_str(path);
    if !fragment_path.is_empty() {
        route.push('#');
        route.push_str(fragment_path);
    }
    if !route.ends_with('/') {
        route.push('/');
    }

    let mut query = String::with_capacity(path_query.len() + fragment_query.len() + 1);
    query.push_str(path_query);
    if !fragment_query.is_empty() {
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(fragment_query);
    }

    Some(ParsedUrl {
        host: host.to_string(),
        route,
        query,
    })
}

/// Whether `host` is `domain` itself or a subdomain of it.
fn host_matches(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{}", domain))
}

/// Routes that mean "a Microsoft Teams meeting is open".
///
/// Deliberately excludes the rest of the Teams web app. `/_#/conversations` is
/// chat, `/_#/calendar` and `/_#/meetings` list meetings without joining one,
/// and the app root is just Teams being open. Treating those as meetings means
/// anyone who leaves Teams open in a tab reads as permanently in a meeting.
fn get_teams_meeting_routes() -> Vec<&'static str> {
    vec![
        "/l/meetup-join/",    // join link, and the in-call route
        "/meetup-join/",      // same route without the /l prefix
        "/pre-join-calling/", // join screen
        "/modern-calling/",   // 1:1 and group calls
        "/light-meetings/launch/", // teams.live.com anonymous join
        "/meet-now/",
        "/meet/", // note: does not match "/meetings/" — see ParsedUrl::route
    ]
}

/// Hosts that serve the Teams web app, including the new unified
/// cloud.microsoft domain.
/// <https://support.microsoft.com/en-us/office/what-is-cloud-microsoft-7ba4c8b9-d062-4444-84a5-fca6c3006d2b>
fn get_teams_hosts() -> Vec<&'static str> {
    vec![
        "teams.microsoft.com",
        "teams.live.com",
        "teams.cloud.microsoft",
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

/// Check whether an open tab is an active meeting.
///
/// Matching is host-first, then route. Both must agree, so a page that merely
/// mentions a meeting domain in its path, query, or fragment is not a meeting.
///
/// **Google Meet** — host `meet.google.com`, first path segment is a valid
/// meeting code (`cih-fjjf-pfd`). Excludes `/landing`, `/new`, `/join`, `/`.
///
/// **Microsoft Teams** — `teams.microsoft.com`, `teams.live.com`, or
/// `teams.cloud.microsoft`, with a meeting route. Chat, calendar, files, and
/// the app root are not meetings; see `get_teams_meeting_routes`.
///
/// **Zoom web client** — any `zoom.us` host with `/j/`, `/s/`, or `/wc/`.
///
/// **Webex** — any `webex.com` host with a join or in-meeting route.
pub fn is_meeting_url(url: &str) -> bool {
    let Some(parsed) = parse_url(url) else {
        return false;
    };

    // Google Meet: the meeting code is the whole signal.
    if host_matches(&parsed.host, "meet.google.com") {
        let segment = parsed.route.trim_matches('/');
        // A code never contains a further path segment, query, or fragment.
        if segment.contains('/') || segment.contains('#') {
            return false;
        }
        if ["landing", "new", "join", ""].contains(&segment) {
            return false;
        }
        return is_valid_google_meet_code(segment);
    }

    // Microsoft Teams, including the new cloud.microsoft domain.
    if get_teams_hosts()
        .iter()
        .any(|&host| host_matches(&parsed.host, host))
    {
        // The v2 web app signals an anonymous join through the query string
        // rather than the route.
        if parsed.query.contains("meetingjoin=true") {
            return true;
        }

        return get_teams_meeting_routes()
            .iter()
            .any(|&route| parsed.route.contains(route));
    }

    // Zoom web client. Native Zoom is handled by network detection in Tier 1.
    if host_matches(&parsed.host, "zoom.us") {
        return ["/j/", "/s/", "/wc/"]
            .iter()
            .any(|&route| parsed.route.contains(route));
    }

    // Webex. Subdomains are per-site and per-region (meet1655.webex.com), so
    // every webex.com host is in scope and the route decides.
    if host_matches(&parsed.host, "webex.com") {
        // `/webappng` is dropped on purpose: the previous pattern list labelled
        // it "Visitor dashboard (after meeting)" while still counting it as a
        // meeting. `/meetings` is dropped for the same reason — it lists
        // meetings rather than joining one, and does not match "/meeting/".
        return ["/wbxmjs/joinservice/", "/meet/", "/meeting/", "/join/"]
            .iter()
            .any(|&route| parsed.route.contains(route));
    }

    false
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
        assert!(!is_meeting_url("https://meet.google.com/not-a-code-at-all-here"));
    }

    /// The Teams web app is a full workspace. Only a meeting route counts, or
    /// leaving Teams open in a tab reads as permanently in a meeting.
    #[test]
    fn teams_chat_and_workspace_are_not_meetings() {
        for url in [
            "https://teams.cloud.microsoft/v2/",
            "https://teams.cloud.microsoft/_#/apps",
            "https://teams.cloud.microsoft/_#/school/conversations/general",
            "https://teams.microsoft.com/_#/conversations/general?threadId=19:abc",
            "https://teams.microsoft.com/_#/calendar",
            "https://teams.microsoft.com/_#/meetings",
            "https://teams.microsoft.com/_#/files",
            "https://teams.live.com/v2/",
        ] {
            assert!(!is_meeting_url(url), "{} must not be a meeting", url);
        }
    }

    #[test]
    fn teams_meeting_routes_are_detected() {
        for url in [
            "https://teams.microsoft.com/_#/l/meetup-join/19:meeting_abc@thread.v2/0",
            "https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc/0?context=%7b%7d",
            "https://teams.cloud.microsoft/_#/l/meetup-join/19:meeting_abc@thread.v2/0",
            "https://teams.cloud.microsoft/_#/pre-join-calling/19:meeting_abc",
            "https://teams.cloud.microsoft/_#/modern-calling/19:abc",
            "https://teams.live.com/light-meetings/launch?p=abc123",
            "https://teams.live.com/v2/?meetingjoin=true",
            "https://teams.microsoft.com/_#/meet",
        ] {
            assert!(is_meeting_url(url), "{} must be a meeting", url);
        }
    }

    /// Matching used to run against the whole URL string, so any page that
    /// mentioned a meeting domain in its path, query, or fragment counted.
    #[test]
    fn pages_that_merely_mention_a_meeting_domain_are_not_meetings() {
        for url in [
            "https://www.google.com/search?q=teams.cloud.microsoft",
            "https://news.ycombinator.com/item?id=1#teams.cloud.microsoft",
            "https://en.wikipedia.org/wiki/Webex.com/meet",
            "https://support.microsoft.com/en-us/office/what-is-cloud-microsoft",
            "https://example.com/redirect?to=https://meet.google.com/cih-fjjf-pfd",
            "https://blog.example.com/how-to-use-zoom.us/j/123",
        ] {
            assert!(!is_meeting_url(url), "{} must not be a meeting", url);
        }
    }

    /// Userinfo must not be mistaken for the host.
    #[test]
    fn userinfo_cannot_impersonate_a_meeting_host() {
        assert!(!is_meeting_url("https://meet.google.com@evil.com/cih-fjjf-pfd"));
        assert!(!is_meeting_url("https://teams.microsoft.com@evil.com/_#/meet"));
    }

    #[test]
    fn matches_subdomains_but_not_lookalike_domains() {
        assert!(is_meeting_url("https://meet1655.webex.com/wbxmjs/joinservice/sites/x"));
        assert!(is_meeting_url("https://us05web.zoom.us/wc/join/123456789"));
        assert!(!is_meeting_url("https://zoom.us.evil.com/j/123456789"));
        assert!(!is_meeting_url("https://notzoom.us/j/123456789"));
        assert!(!is_meeting_url("https://webex.com.attacker.net/meet/someone"));
    }

    #[test]
    fn zoom_and_webex_meeting_routes() {
        assert!(is_meeting_url("https://zoom.us/j/1234567890"));
        assert!(is_meeting_url("https://zoom.us/s/1234567890"));
        assert!(is_meeting_url("https://webex.com/meet/someone"));
        // Listing pages are not meetings.
        assert!(!is_meeting_url("https://web.webex.com/meetings?autosignin=true"));
        assert!(!is_meeting_url("https://zoom.us/profile"));
        assert!(!is_meeting_url("https://zoom.us/"));
    }

    #[test]
    fn ignores_non_http_schemes() {
        assert!(!is_meeting_url("about:blank"));
        assert!(!is_meeting_url("file:///Users/me/meet.google.com/cih-fjjf-pfd"));
        assert!(!is_meeting_url(""));
    }

    #[test]
    fn parses_url_parts() {
        let parsed = parse_url("https://Teams.Microsoft.com:443/_#/l/meetup-join/19:x?a=b").unwrap();
        assert_eq!(parsed.host, "teams.microsoft.com");
        assert_eq!(parsed.route, "/_#/l/meetup-join/19:x/");
        assert_eq!(parsed.query, "a=b");

        // Parameters before the fragment, and on both sides of it.
        let parsed = parse_url("https://teams.live.com/v2/?meetingjoin=true").unwrap();
        assert_eq!(parsed.route, "/v2/");
        assert_eq!(parsed.query, "meetingjoin=true");

        let parsed = parse_url("https://teams.live.com/v2/?a=1#/route?meetingjoin=true").unwrap();
        assert_eq!(parsed.route, "/v2/#/route/");
        assert_eq!(parsed.query, "a=1&meetingjoin=true");
    }
}
