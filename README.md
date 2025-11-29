# Meeting Detection Engine

A cross-platform meeting detection engine for Electron and Node.js applications. Built with Rust and exposed to JavaScript via napi-rs.

## Features

- ✅ Detect if microphone is in use
- ✅ Detect if camera is in use  
- ✅ Detect if major meeting apps are running (Zoom, Teams, Google Meet, Webex)
- ✅ Detect if meeting windows are visible
- ✅ Weighted decision logic to determine meeting status
- ✅ Event-based API (`meeting_started`, `meeting_ended`)
- ✅ Simple JavaScript API

## Supported Platforms

- **macOS** ✅
- **Windows** ✅
- **Linux** ✅

## Installation

```bash
npm install
npm run build
```

## Usage

### Basic Example

```javascript
const { isMeetingActive, onMeetingStart, onMeetingEnd, init } = require('./index');

// Initialize the engine
init();

// Check current status
const active = isMeetingActive();
console.log('Meeting active:', active);

// Listen for meeting start
onMeetingStart(() => {
  console.log('🎥 Meeting started!');
  // Do something when meeting starts
});

// Listen for meeting end
onMeetingEnd(() => {
  console.log('✅ Meeting ended!');
  // Do something when meeting ends
});
```

### Electron Example

```javascript
const { app, BrowserWindow } = require('electron');
const { isMeetingActive, onMeetingStart, onMeetingEnd, init } = require('meeting-detection');

app.whenReady().then(() => {
  init();
  
  // Update UI when meeting status changes
  onMeetingStart(() => {
    console.log('User is in a meeting');
    // Update your app UI
  });
  
  onMeetingEnd(() => {
    console.log('User is no longer in a meeting');
    // Update your app UI
  });
  
  // Poll current status
  setInterval(() => {
    const active = isMeetingActive();
    updateStatusIndicator(active);
  }, 5000);
});
```

## API Reference

### `init()`

Initialize the meeting detection engine. This starts the background polling (every 2 seconds).

**Note:** Call this once when your app starts.

### `isMeetingActive(): boolean`

Returns `true` if a meeting is currently detected, `false` otherwise.

**Example:**
```javascript
const active = isMeetingActive();
if (active) {
  console.log('User is in a meeting');
}
```

### `onMeetingStart(callback: () => void): void`

Register a callback that will be called when a meeting is detected to start.

**Example:**
```javascript
onMeetingStart(() => {
  console.log('Meeting started!');
  // Your logic here
});
```

### `onMeetingEnd(callback: () => void): void`

Register a callback that will be called when a meeting is detected to end.

**Example:**
```javascript
onMeetingEnd(() => {
  console.log('Meeting ended!');
  // Your logic here
});
```

## How It Works

The engine uses a **weighted scoring system** to determine if a meeting is active:

- **Meeting app detected** (Zoom, Teams, etc.): +3 points
- **Meeting window visible**: +2 points
- **Microphone active**: +2 points
- **Camera active**: +1 point

**Threshold:** A meeting is considered active if the total score is ≥ 3.

### Detection Signals

1. **Process Detection**: Checks if known meeting applications are running
2. **Window Detection**: Looks for windows with meeting-related titles (fuzzy matching)
3. **Microphone Detection**: Checks if microphone is in use (platform-specific)
4. **Camera Detection**: Checks if camera is in use (platform-specific)

## Development

### Prerequisites

- Rust (latest stable version)
- Node.js 16+
- npm or yarn

### Building

```bash
# Install dependencies
npm install

# Build for current platform
npm run build

# Build in debug mode
npm run build:debug
```

### Testing

```bash
npm test
```

## Architecture

```
src/
├── lib.rs           # Main entry point, napi-rs bindings
├── detector.rs      # Core detection logic with weighted scoring
├── config.rs        # Meeting app names and window patterns
├── error.rs         # Error types
└── platform/
    ├── mod.rs       # Platform abstraction
    ├── macos.rs     # macOS implementation
    ├── windows.rs   # Windows implementation
    └── linux.rs     # Linux implementation
```

## Permissions

### macOS

The app may need the following permissions:
- **Microphone Access**: System Preferences → Security & Privacy → Microphone
- **Camera Access**: System Preferences → Security & Privacy → Camera
- **Accessibility**: For window detection (System Preferences → Security & Privacy → Accessibility)

### Windows

- **Microphone Access**: Windows Settings → Privacy → Microphone
- **Camera Access**: Windows Settings → Privacy → Camera

### Linux

- **Audio**: May need PulseAudio access
- **Window Manager**: Requires `xdotool` or `wmctrl` for window detection

## Limitations (v1)

- Microphone detection is simplified (uses heuristics)
- Camera detection uses process-based heuristics
- Window detection relies on system scripts (osascript, PowerShell, xdotool)
- Some detection may require user permissions

## Roadmap

- [ ] Improved microphone detection using platform APIs
- [ ] Improved camera detection using platform APIs
- [ ] Native window detection (no scripts)
- [ ] Configurable detection thresholds
- [ ] More meeting apps support
- [ ] Performance optimizations

## License

MIT

## Contributing

Contributions welcome! This is v1, so there's plenty of room for improvement.

## Support

For issues and questions, please open an issue on GitHub.

