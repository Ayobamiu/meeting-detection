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
        
        let process_name = parts[0].to_string();
        let protocol = if line.contains("TCP") { "TCP" } else if line.contains("UDP") { "UDP" } else { "UNKNOWN" }.to_string();
        
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

/// Get network connections for a specific process
pub fn get_network_connections_for_process(process_name: &str) -> Result<Vec<NetworkConnection>, DetectionError> {
    let output = Command::new("lsof")
        .arg("-i")
        .arg("-P")
        .arg("-n")
        .output()
        .map_err(|e| DetectionError::SystemError(format!("Failed to run lsof: {}", e)))?;
    
    if !output.status.success() {
        return Err(DetectionError::SystemError(
            "Failed to get network connections via lsof".to_string(),
        ));
    }
    
    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| DetectionError::SystemError(format!("Invalid UTF-8: {}", e)))?;
    
    let all_connections = parse_lsof_output(&output_str);
    
    // Filter for the specific process (exact match)
    let process_connections: Vec<NetworkConnection> = all_connections
        .into_iter()
        .filter(|conn| conn.process_name == process_name)
        .collect();
    
    Ok(process_connections)
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
) -> Result<(bool, usize, Vec<String>), DetectionError> {
    let connections = get_network_connections_for_process(process_name)?;
    
    let meeting_domains = get_meeting_domains();
    let video_ports = get_meeting_video_ports();
    
    let mut meeting_connections = Vec::new();
    let mut details = Vec::new();
    
    for conn in &connections {
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
    Ok((has_meeting, meeting_connections.len(), details))
}

