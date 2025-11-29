// Core detection logic with app-specific decision tree

use crate::config::{is_meeting_process, is_meeting_window, is_meeting_url};
use crate::error::DetectionError;
use crate::network::detect_meeting_network_activity;
use crate::platform::PlatformDetector;
use crate::platform::{get_browser_tab_urls, is_browser_process};
use log::{debug, info, warn};
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
    /// Native app detected with meeting window visible
    NativeAppWithWindow { app_name: String },
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
    /// Uses app-specific decision tree:
    /// - Tier 1: Native apps (Zoom, Teams desktop) - check network OR window
    /// - Tier 2: Browser-based (Meet, Teams web, Webex) - check browser tab URLs
    pub fn detect(&self) -> Result<DetectionResult, DetectionError> {
        // Check microphone (for supporting info, not decision)
        let microphone_active = self
            .platform
            .is_microphone_active()
            .unwrap_or_else(|e| {
                if e.is_permission_denied() {
                    warn!("Microphone permission denied: {}", e);
                } else {
                    debug!("Microphone detection error: {}", e);
                }
                false
            });

        // Check camera (for supporting info, not decision)
        let camera_active = self
            .platform
            .is_camera_active()
            .unwrap_or_else(|e| {
                if e.is_permission_denied() {
                    warn!("Camera permission denied: {}", e);
                } else {
                    debug!("Camera detection error: {}", e);
                }
                false
            });

        // Get running processes
        let processes = self.platform.get_running_processes().unwrap_or_else(|e| {
            debug!("Process detection error: {}", e);
            Vec::new()
        });

        // Get visible windows
        let windows = self.platform.get_visible_windows().unwrap_or_else(|e| {
            debug!("Window detection error: {}", e);
            Vec::new()
        });
        info!("Visible windows ({}): {:?}", windows.len(), windows);

        let meeting_window_detected = windows
            .iter()
            .any(|window| is_meeting_window(window));

        // TIER 1: Check for native meeting apps (Zoom, Teams desktop, Webex desktop)
        for process_name in &processes {
            if is_meeting_process(process_name) {
                // Check if it's a browser first (browsers are handled in Tier 2)
                if let Ok(true) = is_browser_process(process_name) {
                    continue; // Skip browsers, handle in Tier 2
                }

                // It's a native meeting app - check network connections first
                if let Ok((has_network, _count, _details)) = detect_meeting_network_activity(process_name) {
                    if has_network {
                        debug!("Native app '{}' detected with active network connections", process_name);
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
                }

                // No network, but check for meeting window
                if meeting_window_detected {
                    debug!("Native app '{}' detected with meeting window", process_name);
                    return Ok(DetectionResult {
                        meeting_app_detected: true,
                        meeting_app_name: Some(process_name.clone()),
                        meeting_window_detected: true,
                        microphone_active,
                        camera_active,
                        score: 0,
                        is_meeting_active: true,
                        reason: DetectionReason::NativeAppWithWindow {
                            app_name: process_name.clone(),
                        },
                    });
                }
            }
        }

        // TIER 2: Check for browser-based meetings (Google Meet, Teams web, Webex web)
        if let Ok(browser_urls_map) = get_browser_tab_urls() {
            for (browser_name, urls) in browser_urls_map {
                for url in urls {
                    if is_meeting_url(&url) {
                        info!("Detected meeting URL in {}: {}", browser_name, url);
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
        debug!("No meeting detected");
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

    /// Check if state changed and return the new state
    pub fn check_state_change(&self) -> Result<Option<MeetingState>, DetectionError> {
        let (_result, state_change) = self.detect_with_state()?;
        Ok(state_change)
    }

    /// Get current meeting state without triggering state change
    pub fn get_current_state(&self) -> Result<MeetingState, DetectionError> {
        let result = self.detect()?;
        Ok(if result.is_meeting_active {
            MeetingState::Active
        } else {
            MeetingState::Inactive
        })
    }
}

