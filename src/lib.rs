// Main library entry point with napi-rs bindings

mod config;
mod detector;
mod error;
mod network;
mod platform;

use detector::{DetectionResult, MeetingDetector, MeetingState};
use error::DetectionError;
use log::{error, info};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use platform::create_platform_detector;
use std::result::Result as StdResult;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

// NOTE: These constants must match the weights in `detector.rs`
const JS_SCORE_MEETING_APP: i32 = 3;
const JS_SCORE_MEETING_WINDOW: i32 = 2;
const JS_SCORE_MICROPHONE: i32 = 2;
const JS_SCORE_CAMERA: i32 = 1;

// Event types for internal engine
#[derive(Clone, Copy)]
pub enum MeetingEvent {
    Started,
    Ended,
}

// Structures exposed to JavaScript for explainability
#[napi(object)]
#[derive(Clone)]
pub struct SignalDetails {
    pub active: bool,
    pub weight: i32,
}

#[napi(object)]
#[derive(Clone)]
pub struct SignalsBreakdown {
    pub meeting_app: SignalDetails,
    pub meeting_window: SignalDetails,
    pub microphone: SignalDetails,
    pub camera: SignalDetails,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsDetectionDetails {
    pub active: bool,
    pub score: i32,
    pub app_name: Option<String>,
    pub signals: SignalsBreakdown,
}

fn detection_result_to_js(result: &DetectionResult) -> JsDetectionDetails {
    // Calculate score for backward compatibility (not used in decision logic)
    let mut score = 0;
    if result.meeting_app_detected {
        score += JS_SCORE_MEETING_APP;
    }
    if result.meeting_window_detected {
        score += JS_SCORE_MEETING_WINDOW;
    }
    if result.microphone_active {
        score += JS_SCORE_MICROPHONE;
    }
    if result.camera_active {
        score += JS_SCORE_CAMERA;
    }

    JsDetectionDetails {
        active: result.is_meeting_active,
        score,
        app_name: result.meeting_app_name.clone(),
        signals: SignalsBreakdown {
            meeting_app: SignalDetails {
                active: result.meeting_app_detected,
                weight: JS_SCORE_MEETING_APP,
            },
            meeting_window: SignalDetails {
                active: result.meeting_window_detected,
                weight: JS_SCORE_MEETING_WINDOW,
            },
            microphone: SignalDetails {
                active: result.microphone_active,
                weight: JS_SCORE_MICROPHONE,
            },
            camera: SignalDetails {
                active: result.camera_active,
                weight: JS_SCORE_CAMERA,
            },
        },
    }
}

// Main detection engine that runs in the background
pub struct DetectionEngine {
    detector: Arc<MeetingDetector>,
    event_tx: broadcast::Sender<MeetingEvent>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_callbacks: Arc<std::sync::Mutex<Vec<ThreadsafeFunction<JsDetectionDetails>>>>,
    end_callbacks: Arc<std::sync::Mutex<Vec<ThreadsafeFunction<JsDetectionDetails>>>>,
    last_result: Arc<std::sync::Mutex<Option<DetectionResult>>>,
}

impl DetectionEngine {
    pub fn new() -> StdResult<Self, DetectionError> {
        let platform = create_platform_detector()?;
        let detector = Arc::new(MeetingDetector::new(platform));
        let (tx, _) = broadcast::channel(16); // Buffer up to 16 events

        let engine = Self {
            detector: Arc::clone(&detector),
            event_tx: tx.clone(),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            start_callbacks: Arc::new(std::sync::Mutex::new(Vec::new())),
            end_callbacks: Arc::new(std::sync::Mutex::new(Vec::new())),
            last_result: Arc::new(std::sync::Mutex::new(None)),
        };

        // Start event listener
        let start_callbacks = Arc::clone(&engine.start_callbacks);
        let end_callbacks = Arc::clone(&engine.end_callbacks);
        let last_result = Arc::clone(&engine.last_result);
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                // Snapshot last detection details at the moment of the event
                let details_opt = {
                    let guard = last_result.lock().unwrap();
                    guard.as_ref().map(detection_result_to_js)
                };

                if details_opt.is_none() {
                    continue;
                }
                let details = details_opt.unwrap();

                match event {
                    MeetingEvent::Started => {
                        let callbacks = start_callbacks.lock().unwrap();
                        for callback in callbacks.iter() {
                            let _ = callback.call(
                                Ok(details.clone()),
                                ThreadsafeFunctionCallMode::NonBlocking,
                            );
                        }
                    }
                    MeetingEvent::Ended => {
                        let callbacks = end_callbacks.lock().unwrap();
                        for callback in callbacks.iter() {
                            let _ = callback.call(
                                Ok(details.clone()),
                                ThreadsafeFunctionCallMode::NonBlocking,
                            );
                        }
                    }
                }
            }
        });

        Ok(engine)
    }

    pub fn start_polling(&self) {
        if self.is_running.load(std::sync::atomic::Ordering::Acquire) {
            return; // Already running
        }
        
        self.is_running.store(true, std::sync::atomic::Ordering::Release);
        let detector = Arc::clone(&self.detector);
        let tx = self.event_tx.clone();
        let is_running = Arc::clone(&self.is_running);
        let last_result = Arc::clone(&self.last_result);
        
        // Spawn background task to poll every 2 seconds
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            
            while is_running.load(std::sync::atomic::Ordering::Acquire) {
                interval.tick().await;
                
                match detector.detect_with_state() {
                    Ok((result, state_change)) => {
                        // Store last detection result for explainability
                        {
                            let mut guard = last_result.lock().unwrap();
                            *guard = Some(result);
                        }

                        match state_change {
                            Some(MeetingState::Active) => {
                                info!("Meeting started");
                                let _ = tx.send(MeetingEvent::Started);
                            }
                            Some(MeetingState::Inactive) => {
                                info!("Meeting ended");
                                let _ = tx.send(MeetingEvent::Ended);
                            }
                            None => {
                                // No state change
                            }
                        }
                    }
                    Err(e) => {
                        error!("Detection error: {}", e);
                    }
                }
            }
        });
    }

    pub fn stop_polling(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Release);
    }

    pub fn is_meeting_active(&self) -> StdResult<bool, DetectionError> {
        match self.detector.get_current_state()? {
            MeetingState::Active => Ok(true),
            MeetingState::Inactive => Ok(false),
        }
    }

    pub fn add_start_callback(&self, callback: ThreadsafeFunction<JsDetectionDetails>) {
        let mut callbacks = self.start_callbacks.lock().unwrap();
        callbacks.push(callback);
    }

    pub fn add_end_callback(&self, callback: ThreadsafeFunction<JsDetectionDetails>) {
        let mut callbacks = self.end_callbacks.lock().unwrap();
        callbacks.push(callback);
    }

    pub fn get_last_details(&self) -> Option<JsDetectionDetails> {
        let guard = self.last_result.lock().unwrap();
        guard.as_ref().map(detection_result_to_js)
    }
}

// Global engine instance (thread-safe)
static ENGINE: Mutex<Option<Arc<DetectionEngine>>> = Mutex::new(None);

fn get_engine() -> StdResult<Arc<DetectionEngine>, DetectionError> {
    let mut engine_opt = ENGINE.lock().unwrap();
    
    if let Some(ref engine) = *engine_opt {
        return Ok(Arc::clone(engine));
    }
    
    let engine = Arc::new(DetectionEngine::new()?);
    *engine_opt = Some(Arc::clone(&engine));
    Ok(engine)
}

// JavaScript API

/// Check if a meeting is currently active
#[napi]
pub fn is_meeting_active() -> Result<bool> {
    let engine = get_engine().map_err(|e| Error::from_reason(e.to_string()))?;
    engine
        .is_meeting_active()
        .map_err(|e| Error::from_reason(e.to_string()))
}

/// Register a callback for when a meeting starts
#[napi]
pub fn on_meeting_start(
    callback: JsFunction,
) -> Result<()> {
    let engine = get_engine().map_err(|e| Error::from_reason(e.to_string()))?;
    
    let tsfn: ThreadsafeFunction<JsDetectionDetails> =
        callback.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<JsDetectionDetails>| {
            Ok(vec![ctx.value])
        })?;
    
    engine.add_start_callback(tsfn);
    Ok(())
}

/// Register a callback for when a meeting ends
#[napi]
pub fn on_meeting_end(
    callback: JsFunction,
) -> Result<()> {
    let engine = get_engine().map_err(|e| Error::from_reason(e.to_string()))?;
    
    let tsfn: ThreadsafeFunction<JsDetectionDetails> =
        callback.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<JsDetectionDetails>| {
            Ok(vec![ctx.value])
        })?;
    
    engine.add_end_callback(tsfn);
    Ok(())
}

/// Get details about the last detection cycle.
/// This is useful for debugging and explaining why the engine thinks a meeting is active or not.
#[napi]
pub fn get_last_detection_details() -> Result<Option<JsDetectionDetails>> {
    let engine = get_engine().map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(engine.get_last_details())
}

// Initialize logging and start the detection engine
#[napi]
pub fn init() -> Result<()> {
    // Initialize logger
    let _ = env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init();
    
    // Initialize and start the engine
    let engine = get_engine().map_err(|e| Error::from_reason(e.to_string()))?;
    engine.start_polling();
    info!("Meeting detection engine initialized and started");
    Ok(())
}

