// Core detection logic with app-specific decision tree

use crate::config::{is_browser_process, is_meeting_process, is_meeting_url};
use crate::error::DetectionError;
use crate::network::{detect_meeting_network_activity, get_all_network_connections};
use crate::platform::get_browser_tab_urls;
use crate::platform::PlatformDetector;
use std::sync::{Arc, Mutex};

/// State of meeting detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeetingState {
    Inactive,
    Active,
}

/// Reason for meeting detection decision
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionReason {
    /// Native app detected with active network connections
    NativeAppWithNetwork { app_name: String },
    /// Browser tab with meeting URL detected
    BrowserWithMeetingUrl { browser_name: String, url: String },
    /// No meeting detected
    None,
}

/// Detection result with detailed breakdown
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub meeting_app_detected: bool,
    pub meeting_app_name: Option<String>,
    pub meeting_window_detected: bool,
    pub microphone_active: bool,
    pub camera_active: bool,
    pub score: i32, // Kept for backward compatibility, but not used in decision
    pub is_meeting_active: bool,
    pub reason: DetectionReason,
}

/// Main detector that combines all signals
pub struct MeetingDetector {
    platform: Box<dyn PlatformDetector>,
    previous_state: Arc<Mutex<MeetingState>>,
}

impl MeetingDetector {
    pub fn new(platform: Box<dyn PlatformDetector>) -> Self {
        Self {
            platform,
            previous_state: Arc::new(Mutex::new(MeetingState::Inactive)),
        }
    }

    /// Perform a detection cycle and return the result
    /// 
    /// Uses app-specific decision tree:
    /// 
    /// **Tier 1: Native Meeting Apps (Zoom, Teams desktop, Webex desktop)**
    /// - Detection strategy: Network connections are the primary signal
    /// - Why: Native apps maintain persistent network connections when in a meeting
    /// - Zoom-specific: UDP port 8801 is a strong indicator (works with IP addresses)
    /// - Teams/Webex: STUN ports (3478-3481) or meeting domains with ESTABLISHED connections
    /// - Decision: If network connections active → MEETING ACTIVE, else → MEETING INACTIVE
    /// 
    /// **Tier 2: Browser-based Meetings (Google Meet, Teams web, Webex web)**
    /// - Detection strategy: Browser tab URLs are definitive
    /// - Why: Network connections are unreliable for browsers (mixed/encrypted traffic)
    /// - Google Meet: Validates meeting code format (xxx-yyyy-zzz), excludes landing pages
    /// - Teams/Webex web: Pattern matching on meeting URLs
    /// - Decision: If meeting URL detected → MEETING ACTIVE, else → MEETING INACTIVE
    pub fn detect(&self) -> Result<DetectionResult, DetectionError> {
        // Check microphone (for supporting info, not decision)
        let microphone_active = self
            .platform
            .is_microphone_active()
            .unwrap_or(false);

        // Check camera (for supporting info, not decision)
        let camera_active = self
            .platform
            .is_camera_active()
            .unwrap_or(false);

        // Get running processes
        let processes = self.platform.get_running_processes().unwrap_or_default();

        // Window detection disabled for performance (not used in decision tree)
        let meeting_window_detected = false;

        // TIER 1: Check for native meeting apps (Zoom, Teams desktop, Webex desktop)
        // For native apps, network connections are the primary signal
        // Only network connections indicate an active meeting for native apps
        let native_candidates: Vec<&String> = processes
            .iter()
            .filter(|name| is_meeting_process(name) && !is_browser_process(name))
            .collect();

        // `lsof` is run at most once per cycle and shared across candidates,
        // rather than once per candidate.
        if !native_candidates.is_empty() {
            let connections = get_all_network_connections().unwrap_or_default();

            for process_name in native_candidates {
                let (has_network, _count, _details) =
                    detect_meeting_network_activity(process_name, &connections);

                if has_network {
                    return Ok(DetectionResult {
                        meeting_app_detected: true,
                        meeting_app_name: Some(process_name.clone()),
                        meeting_window_detected,
                        microphone_active,
                        camera_active,
                        score: 0, // Not used in new logic
                        is_meeting_active: true,
                        reason: DetectionReason::NativeAppWithNetwork {
                            app_name: process_name.clone(),
                        },
                    });
                }
                // No network connections = no active meeting for native apps
            }
        }

        // TIER 2: Check for browser-based meetings (Google Meet, Teams web, Webex web)
        // For browser-based meetings, meeting URLs are definitive
        if let Ok(browser_urls_map) = get_browser_tab_urls(&processes) {
            for (browser_name, urls) in browser_urls_map {
                for url in urls {
                    if is_meeting_url(&url) {
                        return Ok(DetectionResult {
                            meeting_app_detected: true,
                            meeting_app_name: Some(browser_name.clone()),
                            meeting_window_detected,
                            microphone_active,
                            camera_active,
                            score: 0,
                            is_meeting_active: true,
                            reason: DetectionReason::BrowserWithMeetingUrl {
                                browser_name: browser_name.clone(),
                                url: url.clone(),
                            },
                        });
                    }
                }
            }
        }

        // No meeting detected
        Ok(DetectionResult {
            meeting_app_detected: false,
            meeting_app_name: None,
            meeting_window_detected,
            microphone_active,
            camera_active,
            score: 0,
            is_meeting_active: false,
            reason: DetectionReason::None,
        })
    }

    /// Perform detection and also compute state change in one pass
    pub fn detect_with_state(
        &self,
    ) -> Result<(DetectionResult, Option<MeetingState>), DetectionError> {
        let result = self.detect()?;
        let new_state = if result.is_meeting_active {
            MeetingState::Active
        } else {
            MeetingState::Inactive
        };

        let mut previous = self.previous_state.lock().unwrap();
        let previous_state = *previous;

        let state_change = if new_state != previous_state {
            *previous = new_state;
            Some(new_state)
        } else {
            None
        };

        Ok((result, state_change))
    }
}

