// Network connection detection for meeting apps

use crate::error::DetectionError;
use std::process::Command;

/// Network connection information
#[derive(Debug, Clone)]
pub struct NetworkConnection {
    pub process_name: String,
    pub protocol: String, // TCP, UDP
    pub remote_address: String,
    pub remote_port: u16,
    pub state: String, // ESTABLISHED, LISTEN, etc.
}

/// Meeting service domain patterns to detect
pub fn get_meeting_domains() -> Vec<&'static str> {
    vec![
        // Zoom
        "zoom.us",
        "zoom.com",
        // Teams
        "teams.microsoft.com",
        "office.com",
        // Webex
        "webex.com",
        "cisco.com",
        // Google Meet
        "meet.google.com",
        "google.com", // Too broad, but we'll filter by port/context
    ]
}

/// Meeting service ports for video/audio streaming
pub fn get_meeting_video_ports() -> Vec<u16> {
    vec![
        8801,    // Zoom UDP video/audio
        8802,    // Zoom alternative
        3478,    // STUN (Teams, Webex)
        3479,    // STUN alternative
        3480,    // STUN alternative
        3481,    // STUN alternative
        9000,    // Webex video range start
        9999,    // Webex video range end
        19302,   // Google Meet UDP range start
        19309,   // Google Meet UDP range end
    ]
}

/// Decode the COMMAND column produced by `lsof +c 0`.
///
/// With truncation disabled, lsof escapes non-printable and space characters as
/// `\x` followed by two hex digits, so "Google Chrome Helper" arrives as
/// `Google\x20Chrome\x20Helper`.
fn decode_lsof_name(raw: &str) -> String {
    if !raw.contains("\\x") {
        return raw.to_string();
    }

    let mut decoded = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' && i + 3 < chars.len() && chars[i + 1] == 'x' {
            let hex: String = chars[i + 2..i + 4].iter().collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                decoded.push(byte as char);
                i += 4;
                continue;
            }
        }
        decoded.push(chars[i]);
        i += 1;
    }

    decoded
}

/// Parse lsof output to extract network connections
pub fn parse_lsof_output(output: &str) -> Vec<NetworkConnection> {
    let mut connections = Vec::new();

    for line in output.lines() {
        // Skip header line
        if line.starts_with("COMMAND") || line.trim().is_empty() {
            continue;
        }

        // Parse lsof format:
        // COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let process_name = decode_lsof_name(parts[0]);
        // NODE holds the protocol. Reading it from the column rather than
        // searching the whole line avoids matching "TCP"/"UDP" that appear in a
        // process name or a resolved host name.
        let protocol = match parts[7] {
            "TCP" => "TCP",
            "UDP" => "UDP",
            _ => "UNKNOWN",
        }
        .to_string();

        // Extract remote address and port from NAME field (last field)
        let name_field = parts[8..].join(" ");
        
        // Parse connection info: local->remote or remote:port
        let (remote_address, remote_port, state) = parse_connection_name(&name_field);
        
        connections.push(NetworkConnection {
            process_name,
            protocol,
            remote_address,
            remote_port,
            state,
        });
    }
    
    connections
}

/// Parse connection name field from lsof output
/// Examples:
/// - "localhost:58660->localhost:10011 (ESTABLISHED)"
/// - "10.0.0.199:53127->144-195-35.zoom.us:8801"
/// - "144-195-35.zoom.us:8801"
fn parse_connection_name(name: &str) -> (String, u16, String) {
    // Extract state if present
    let state = if name.contains("ESTABLISHED") {
        "ESTABLISHED".to_string()
    } else if name.contains("LISTEN") {
        "LISTEN".to_string()
    } else if name.contains("CLOSED") {
        "CLOSED".to_string()
    } else {
        "UNKNOWN".to_string()
    };
    
    // Extract remote address and port
    // Look for pattern: ->remote:port or remote:port
    if let Some(arrow_pos) = name.find("->") {
        // Format: local->remote:port
        let remote_part = &name[arrow_pos + 2..];
        if let Some(colon_pos) = remote_part.find(':') {
            let address = remote_part[..colon_pos].to_string();
            let port_str = remote_part[colon_pos + 1..]
                .split_whitespace()
                .next()
                .unwrap_or("0");
            let port = port_str.parse().unwrap_or(0);
            return (address, port, state);
        }
    } else if let Some(colon_pos) = name.find(':') {
        // Format: remote:port (no arrow)
        let address = name[..colon_pos].to_string();
        let port_str = name[colon_pos + 1..]
            .split_whitespace()
            .next()
            .unwrap_or("0");
        let port = port_str.parse().unwrap_or(0);
        return (address, port, state);
    }
    
    ("".to_string(), 0, state)
}

/// Run `lsof` once and return every network connection on the system.
///
/// Callers should invoke this a single time per detection cycle and share the
/// result across processes, rather than re-running `lsof` per candidate.
pub fn get_all_network_connections() -> Result<Vec<NetworkConnection>, DetectionError> {
    let output = Command::new("lsof")
        // `+c 0` disables COMMAND-column truncation. lsof otherwise clips
        // process names to 9 characters, so "ZoomCefHelper" arrived as
        // "ZoomCefHe" and never matched the name reported by sysinfo.
        .arg("+c")
        .arg("0")
        .arg("-i")
        .arg("-P")
        .arg("-n")
        .output()
        .map_err(|e| DetectionError::SystemError(format!("Failed to run lsof: {}", e)))?;

    // lsof exits non-zero when it cannot stat some file descriptors, which is
    // routine for processes owned by other users. Partial output is still
    // usable, so only treat an empty result as a failure.
    let output_str = String::from_utf8_lossy(&output.stdout).into_owned();

    if !output.status.success() && output_str.trim().is_empty() {
        return Err(DetectionError::SystemError(
            "Failed to get network connections via lsof".to_string(),
        ));
    }

    Ok(parse_lsof_output(&output_str))
}

/// Check if network connections indicate an active meeting
/// Returns (has_meeting_connections, connection_count, details)
/// 
/// App-specific detection logic:
/// - Zoom: UDP port 8801 is a strong indicator (works with IP addresses, not just domains)
/// - Teams/Webex: STUN ports (3478-3481) or meeting domains with ESTABLISHED connections
/// - Google Meet: Meeting domains with ESTABLISHED connections or video ports (19302-19309)
pub fn detect_meeting_network_activity(
    process_name: &str,
    all_connections: &[NetworkConnection],
) -> (bool, usize, Vec<String>) {
    let meeting_domains = get_meeting_domains();
    let video_ports = get_meeting_video_ports();

    let mut meeting_connections = Vec::new();
    let mut details = Vec::new();

    for conn in all_connections
        .iter()
        .filter(|conn| conn.process_name == process_name)
    {
        // Check if connection is to a meeting domain
        let is_meeting_domain = meeting_domains.iter().any(|domain| {
            conn.remote_address.contains(domain)
        });
        
        // Check if connection is on a video/audio port
        let is_video_port = video_ports.contains(&conn.remote_port);
        
        // Check if connection is established (active)
        // Only ESTABLISHED connections indicate active meetings
        // CLOSED, CLOSING, or other states mean connection is ending/ended
        let is_established = conn.state == "ESTABLISHED";
        
        // Zoom-specific: UDP port 8801 is a strong indicator
        // UDP connections often show "UNKNOWN" state, so we check the port
        // But only if it's a recent/active connection (not CLOSED)
        let is_zoom_udp = conn.protocol == "UDP" 
            && conn.remote_port == 8801
            && conn.state != "CLOSED";
        
        // Meeting connection if:
        // 1. Meeting domain AND ESTABLISHED (must be active, not just any state), OR
        // 2. Meeting domain AND video port (video ports indicate active streaming), OR
        // 3. Zoom UDP on port 8801 (Zoom-specific, but not if CLOSED)
        let is_meeting_connection = (is_meeting_domain && (is_established || is_video_port)) || is_zoom_udp;
        
        if is_meeting_connection {
            meeting_connections.push(conn.clone());
            details.push(format!(
                "{} {} {}:{} ({})",
                conn.protocol, conn.process_name, conn.remote_address, conn.remote_port, conn.state
            ));
        }
    }
    
    let has_meeting = !meeting_connections.is_empty();
    (has_meeting, meeting_connections.len(), details)
}


#[cfg(test)]
mod tests {
    use super::*;

    /// `lsof +c 0` escapes spaces in the COMMAND column.
    #[test]
    fn decodes_escaped_process_names() {
        assert_eq!(
            decode_lsof_name("Google\\x20Chrome\\x20Helper"),
            "Google Chrome Helper"
        );
        assert_eq!(decode_lsof_name("zoom.us"), "zoom.us");
        assert_eq!(decode_lsof_name("ZoomCefHelper"), "ZoomCefHelper");
    }

    /// Names longer than 9 characters must survive parsing. lsof truncates them
    /// unless `+c 0` is passed, which previously broke the exact-match filter
    /// for every app with a long process name (Teams, Webex, Zoom's helpers).
    #[test]
    fn parses_full_length_process_names() {
        let output = "\
COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
Microsoft\\x20Teams 123 user 55u IPv4 0x1 0t0 TCP 10.0.0.1:5000->52.112.1.1:3478 (ESTABLISHED)";

        let connections = parse_lsof_output(output);

        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].process_name, "Microsoft Teams");
        assert_eq!(connections[0].remote_port, 3478);
        assert_eq!(connections[0].state, "ESTABLISHED");
        assert_eq!(connections[0].protocol, "TCP");
    }

    #[test]
    fn reads_protocol_from_the_node_column() {
        // A remote host containing "TCP" must not make a UDP row read as TCP.
        let output = "\
zoom.us 1 user 5u IPv4 0x1 0t0 UDP 10.0.0.1:5000->tcp-edge.zoom.us:8801";

        let connections = parse_lsof_output(output);

        assert_eq!(connections[0].protocol, "UDP");
    }

    #[test]
    fn detects_zoom_udp_media_traffic() {
        let connections = parse_lsof_output(
            "zoom.us 1 user 5u IPv4 0x1 0t0 UDP 10.0.0.1:5000->170.114.52.4:8801",
        );

        let (active, count, _) = detect_meeting_network_activity("zoom.us", &connections);

        assert!(active);
        assert_eq!(count, 1);
    }

    #[test]
    fn ignores_connections_belonging_to_other_processes() {
        let connections = parse_lsof_output(
            "SomethingElse 1 user 5u IPv4 0x1 0t0 UDP 10.0.0.1:5000->170.114.52.4:8801",
        );

        let (active, _, _) = detect_meeting_network_activity("zoom.us", &connections);

        assert!(!active);
    }

    #[test]
    fn idle_https_traffic_is_not_a_meeting() {
        // A closing connection to a meeting domain must not count as active.
        let connections = parse_lsof_output(
            "zoom.us 1 user 5u IPv4 0x1 0t0 TCP 10.0.0.1:5000->zoom.us:443 (CLOSE_WAIT)",
        );

        let (active, _, _) = detect_meeting_network_activity("zoom.us", &connections);

        assert!(!active);
    }
}
