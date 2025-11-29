// Core detection logic with weighted scoring and state management

use crate::config::{is_meeting_process, is_meeting_window};
use crate::error::DetectionError;
use crate::platform::PlatformDetector;
use log::{debug, info, warn};
use std::sync::{Arc, Mutex};

/// Detection scores for weighted decision logic
const SCORE_MEETING_APP: i32 = 3;
const SCORE_MEETING_WINDOW: i32 = 2;
const SCORE_MICROPHONE: i32 = 2;
const SCORE_CAMERA: i32 = 1;
const THRESHOLD: i32 = 3; // Minimum score to consider meeting active

/// State of meeting detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeetingState {
    Inactive,
    Active,
}

/// Detection result with detailed breakdown
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub meeting_app_detected: bool,
    pub meeting_app_name: Option<String>,
    pub meeting_window_detected: bool,
    pub microphone_active: bool,
    pub camera_active: bool,
    pub score: i32,
    pub is_meeting_active: bool,
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
    pub fn detect(&self) -> Result<DetectionResult, DetectionError> {
        // Check microphone (gracefully handle permission errors)
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

        // Check camera (gracefully handle permission errors)
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

        // Check for meeting apps in running processes
        let processes = self.platform.get_running_processes().unwrap_or_else(|e| {
            debug!("Process detection error: {}", e);
            Vec::new()
        });

        // Process logging removed - too noisy. Use RUST_LOG=debug if needed.

        // Find the first matching meeting app and capture its name
        let meeting_app_name = processes
            .iter()
            .find(|process| is_meeting_process(process))
            .map(|s| s.clone());
        
        let meeting_app_detected = meeting_app_name.is_some();

        // Check for meeting windows
        let windows = self.platform.get_visible_windows().unwrap_or_else(|e| {
            debug!("Window detection error: {}", e);
            Vec::new()
        });

        // Log all visible window titles for introspection
        info!("Visible windows ({}): {:?}", windows.len(), windows);

        let meeting_window_detected = windows
            .iter()
            .any(|window| is_meeting_window(window));

        // Check browser tab URLs (for browser-based meetings)
        use crate::platform::get_browser_tab_urls;
        let _ = get_browser_tab_urls(); // URLs are already logged inside get_browser_tab_urls()

        // Calculate weighted score
        let mut score = 0;
        if meeting_app_detected {
            score += SCORE_MEETING_APP;
        }
        if meeting_window_detected {
            score += SCORE_MEETING_WINDOW;
        }
        if microphone_active {
            score += SCORE_MICROPHONE;
        }
        if camera_active {
            score += SCORE_CAMERA;
        }

        let is_meeting_active = score >= THRESHOLD;

        debug!(
            "Detection: app={}, window={}, mic={}, cam={}, score={}, active={}",
            meeting_app_detected, meeting_window_detected, microphone_active, camera_active, score, is_meeting_active
        );

        Ok(DetectionResult {
            meeting_app_detected,
            meeting_app_name,
            meeting_window_detected,
            microphone_active,
            camera_active,
            score,
            is_meeting_active,
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

