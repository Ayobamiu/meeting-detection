# Project Summary

## What We Built

A **cross-platform meeting detection engine** that:
- Detects microphone/camera usage
- Detects meeting apps (Zoom, Teams, Meet, Webex)
- Detects meeting windows
- Uses weighted scoring to determine meeting status
- Emits events when meetings start/end
- Exposes a simple JavaScript API

## Architecture Decisions

### 1. **Platform Abstraction**
- Used Rust traits (like interfaces) to abstract platform-specific code
- Each platform (macOS, Windows, Linux) implements the `PlatformDetector` trait
- Makes it easy to add new platforms or improve existing ones

### 2. **Weighted Decision Logic**
- Meeting app: +3 points
- Meeting window: +2 points  
- Microphone: +2 points
- Camera: +1 point
- Threshold: ≥3 = meeting active

This ensures we don't get false positives (e.g., just mic active = not a meeting).

### 3. **Event System**
- Uses `tokio::broadcast` for event distribution
- Thread-safe callback storage with `Arc<Mutex<Vec<>>>`
- Non-blocking JavaScript callbacks via `ThreadsafeFunction`

### 4. **Error Handling**
- Custom error types with `thiserror`
- Graceful degradation: permission errors don't crash the app
- Errors are logged but detection continues with other signals

### 5. **Polling Strategy**
- Polls every 2 seconds (as requested)
- State machine tracks previous state to emit events only on changes
- Background task runs independently of JavaScript event loop

## Current Implementation Status

### ✅ Fully Implemented
- Project structure and build system
- Platform abstraction layer
- Core detection logic with weighted scoring
- Event system (meeting_started, meeting_ended)
- JavaScript API (isMeetingActive, onMeetingStart, onMeetingEnd)
- Process detection (all platforms)
- Window detection (all platforms)
- State management and event emission

### ⚠️ Simplified (v1)
- **Microphone detection**: Returns `false` for now (placeholder)
  - *Why*: Requires platform-specific audio APIs (CoreAudio, WASAPI, PulseAudio)
  - *Impact*: Detection still works via app/window/camera signals
  - *Future*: Can be improved with proper audio device enumeration

- **Camera detection**: Uses process-based heuristics
  - *Why*: Direct camera access requires permissions and platform APIs
  - *Impact*: Works for common apps (Zoom, Teams, etc.)
  - *Future*: Can be improved with AVFoundation/WinAPI/v4l2

- **Window detection**: Uses system scripts
  - macOS: `osascript` (AppleScript)
  - Windows: PowerShell
  - Linux: `xdotool` / `wmctrl`
  - *Why*: Simplest cross-platform approach for v1
  - *Future*: Native window APIs for better performance

## File Structure Explained

```
src/
├── lib.rs              # Entry point, napi bindings, global engine
├── detector.rs          # Core logic: scoring, state machine
├── config.rs            # Meeting app names, window patterns
├── error.rs             # Custom error types
└── platform/
    ├── mod.rs           # Trait definition, platform factory
    ├── macos.rs         # macOS: osascript for windows, sysinfo for processes
    ├── windows.rs       # Windows: PowerShell for windows, sysinfo for processes
    └── linux.rs         # Linux: xdotool/wmctrl for windows, sysinfo for processes
```

## Key Rust Patterns Used

1. **Traits for Abstraction**: `PlatformDetector` trait
2. **Result for Error Handling**: `Result<T, DetectionError>`
3. **Arc for Shared Ownership**: Thread-safe shared references
4. **Mutex for Mutability**: Thread-safe mutable data
5. **Tokio for Async**: Background polling and event handling
6. **napi-rs for FFI**: JavaScript bindings

## Testing the Project

1. **Build**:
   ```bash
   npm install
   npm run build
   ```

2. **Run test**:
   ```bash
   node test.js
   ```

3. **Expected behavior**:
   - Engine initializes
   - Polls every 2 seconds
   - Shows current status
   - Emits events when meetings start/end

## Next Steps for Improvement

### Short Term (Easy Wins)
1. Add more meeting apps to `config.rs`
2. Improve window title patterns
3. Add better logging/debugging
4. Add unit tests

### Medium Term (More Work)
1. Implement proper microphone detection:
   - macOS: CoreAudio device enumeration
   - Windows: WASAPI audio session detection
   - Linux: PulseAudio source monitoring

2. Implement proper camera detection:
   - macOS: AVFoundation device access
   - Windows: Media Foundation
   - Linux: v4l2 device monitoring

3. Native window detection (no scripts):
   - macOS: CoreGraphics/AppKit
   - Windows: WinAPI EnumWindows
   - Linux: X11/Wayland APIs

### Long Term (Advanced)
1. Machine learning for better detection
2. Configurable thresholds
3. Plugin system for custom apps
4. Performance optimizations
5. Battery usage optimization

## For JavaScript Developers

### Understanding the Code Flow

1. **JavaScript calls `init()`**
   → Creates `DetectionEngine`
   → Starts background polling task

2. **Background task (every 2 seconds)**
   → Calls `detector.detect()`
   → Checks all signals (app, window, mic, cam)
   → Calculates score
   → Compares to threshold
   → Emits event if state changed

3. **Event emission**
   → Broadcasts to all subscribers
   → Calls JavaScript callbacks via `ThreadsafeFunction`

4. **JavaScript calls `isMeetingActive()`**
   → Synchronously checks current state
   → Returns boolean

### Key Concepts

- **Arc**: Shared ownership (like multiple references in JS, but thread-safe)
- **Mutex**: Lock for safe mutation (like a lock in async JS)
- **Result**: Error handling (like try/catch, but explicit)
- **Trait**: Interface/contract (like TypeScript interface)
- **Tokio**: Async runtime (like Node.js event loop, but for Rust)

## Common Questions

**Q: Why Rust?**
A: Performance, memory safety, and excellent FFI support via napi-rs.

**Q: Why not pure JavaScript?**
A: System APIs (processes, windows, audio devices) require native code.

**Q: Can I add my own meeting app?**
A: Yes! Edit `src/config.rs` and add the process name.

**Q: How do I adjust sensitivity?**
A: Edit the scores/threshold in `src/detector.rs`.

**Q: Why are mic/camera detection simplified?**
A: v1 focuses on getting the core working. These can be improved incrementally.

## Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **napi-rs**: https://napi.rs/
- **Tokio**: https://tokio.rs/
- **sysinfo**: https://docs.rs/sysinfo/

