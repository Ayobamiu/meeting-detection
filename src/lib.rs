// Main library entry point with napi-rs bindings

mod config;
mod detector;
mod error;
mod platform;

use detector::{MeetingDetector, MeetingState};
use error::DetectionError;
use log::{error, info};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use platform::create_platform_detector;
use std::result::Result as StdResult;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

// Event types for JavaScript
#[derive(Clone, Copy)]
pub enum MeetingEvent {
    Started,
    Ended,
}

// Main detection engine that runs in the background
pub struct DetectionEngine {
    detector: Arc<MeetingDetector>,
    event_tx: broadcast::Sender<MeetingEvent>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    start_callbacks: Arc<std::sync::Mutex<Vec<ThreadsafeFunction<()>>>>,
    end_callbacks: Arc<std::sync::Mutex<Vec<ThreadsafeFunction<()>>>>,
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
        };
        
        // Start event listener
        let start_callbacks = Arc::clone(&engine.start_callbacks);
        let end_callbacks = Arc::clone(&engine.end_callbacks);
        let mut rx = tx.subscribe();
        
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    MeetingEvent::Started => {
                        let callbacks = start_callbacks.lock().unwrap();
                        for callback in callbacks.iter() {
                            let _ = callback.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
                        }
                    }
                    MeetingEvent::Ended => {
                        let callbacks = end_callbacks.lock().unwrap();
                        for callback in callbacks.iter() {
                            let _ = callback.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
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
        
        // Spawn background task to poll every 2 seconds
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            
            while is_running.load(std::sync::atomic::Ordering::Acquire) {
                interval.tick().await;
                
                match detector.check_state_change() {
                    Ok(Some(MeetingState::Active)) => {
                        info!("Meeting started");
                        let _ = tx.send(MeetingEvent::Started);
                    }
                    Ok(Some(MeetingState::Inactive)) => {
                        info!("Meeting ended");
                        let _ = tx.send(MeetingEvent::Ended);
                    }
                    Ok(None) => {
                        // No state change
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

    pub fn add_start_callback(&self, callback: ThreadsafeFunction<()>) {
        let mut callbacks = self.start_callbacks.lock().unwrap();
        callbacks.push(callback);
    }

    pub fn add_end_callback(&self, callback: ThreadsafeFunction<()>) {
        let mut callbacks = self.end_callbacks.lock().unwrap();
        callbacks.push(callback);
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
    
    let tsfn: ThreadsafeFunction<()> = callback
        .create_threadsafe_function(0, |ctx| {
            ctx.env.get_undefined().map(|v| vec![v])
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
    
    let tsfn: ThreadsafeFunction<()> = callback
        .create_threadsafe_function(0, |ctx| {
            ctx.env.get_undefined().map(|v| vec![v])
        })?;
    
    engine.add_end_callback(tsfn);
    Ok(())
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

