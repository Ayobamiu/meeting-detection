# Meeting Detection Engine

A macOS meeting detection engine for Electron and Node.js applications. Built with Rust and exposed to JavaScript via napi-rs.

## Features

- ✅ Two-tier decision tree algorithm for accurate detection
- ✅ Event-based API (`onMeetingStart`, `onMeetingEnd`)
- ✅ Simple JavaScript API

## Supported Platforms & Services

| Service             | Native App | Browser | Detection Method                                    |
| ------------------- | ---------- | ------- | --------------------------------------------------- |
| **Zoom**            | ✅         | ✅      | Network (UDP 8801) or URL patterns                  |
| **Google Meet**     | ❌         | ✅      | URL patterns with meeting code validation           |
| **Microsoft Teams** | ✅         | ✅      | Network (STUN/TURN) or URL patterns (incl. teams.cloud.microsoft) |
| **Webex**           | ✅         | ✅      | Network (video ports) or URL patterns               |

**Supported Operating Systems:** macOS (Intel and Apple Silicon)

Microsoft Teams is transitioning to the new URL **teams.cloud.microsoft** (same app, better security). The engine supports both the legacy domains and the new cloud.microsoft domain for browser and native app detection.

## Installation

```bash
npm install meeting-detection
```

The package will automatically build native binaries for your platform during installation.

## Usage

```javascript
const { init, isMeetingActive, onMeetingStart, onMeetingEnd, getLastDetectionDetails } = require("meeting-detection");

// Initialize (starts background polling every 2 seconds)
init();

// Check current status
const active = isMeetingActive();

// Get detailed information
const details = getLastDetectionDetails();
if (details) {
  console.log("App:", details.appName);
  console.log("Reason:", details.reason);
  if (details.meetingUrl) console.log("URL:", details.meetingUrl);
}

// Listen for events (callbacks use error-first pattern: (_, details) => {})
onMeetingStart((_, details) => {
  console.log("🎥 Meeting started:", details.appName);
});

onMeetingEnd((_, details) => {
  console.log("✅ Meeting ended");
});
```

**For Electron apps:** Use `ipcMain` in the main process to forward events to the renderer process, as this native addon only works in the main process.

## API Reference

### `init()`

Initialize the engine (starts background polling every 2 seconds). Call once when your app starts.

### `isMeetingActive(): boolean`

Returns `true` if a meeting is currently detected, `false` otherwise.

```javascript
if (isMeetingActive()) {
  console.log("User is in a meeting");
}
```

### `onMeetingStart(callback: (error: null, details: JsDetectionDetails) => void): void`

Register callback for when a meeting starts. Uses Node.js error-first convention (error is always `null`).

```javascript
onMeetingStart((_, details) => {
  console.log("Meeting started:", details.appName);
});
```

### `onMeetingEnd(callback: (error: null, details: JsDetectionDetails) => void): void`

Register callback for when a meeting ends. Uses Node.js error-first convention (error is always `null`).

```javascript
onMeetingEnd((_, details) => {
  console.log("Meeting ended");
});
```

### `getLastDetectionDetails(): JsDetectionDetails | null`

Get detailed information about the last detection cycle. Returns `null` if no detection has been performed yet.

```javascript
const details = getLastDetectionDetails();
if (details) {
  console.log("App:", details.appName, "Reason:", details.reason);
}
```

## Type Reference

### `JsDetectionDetails`

The detection details object returned by callbacks and `getLastDetectionDetails()`. Exported for TypeScript projects.

```typescript
import { JsDetectionDetails } from "meeting-detection";

interface JsDetectionDetails {
  active: boolean;              // Whether a meeting is currently active
  score: number;                // Legacy scoring (for backward compatibility)
  appName?: string;              // Meeting app name (e.g., "Zoom", "Safari", "Chrome")
  reason: string;                // Detection reason: "NativeAppWithNetwork(Zoom)", "BrowserWithMeetingUrl(Safari)", or "None"
  meetingUrl?: string;           // Meeting URL if detected in browser
  signals: SignalsBreakdown;     // Breakdown of detection signals (for debugging)
}

interface SignalsBreakdown {
  meetingApp: SignalDetails;
  meetingWindow: SignalDetails;
  microphone: SignalDetails;
  camera: SignalDetails;
}

interface SignalDetails {
  active: boolean;
  weight: number;
}
```

## How It Works

The engine uses a **two-tier decision tree** to detect active meetings:

### Tier 1: Native Meeting Apps (Zoom, Teams desktop, Webex desktop)

- Detects native meeting app processes
- Checks for active network connections (Zoom: UDP 8801, Teams: STUN/TURN, Webex: video ports)
- **Decision**: Network activity detected → **MEETING ACTIVE**

### Tier 2: Browser-Based Meetings (Google Meet, Teams web, Webex web)

- Scans browser tabs (Chrome, Safari, Edge) using AppleScript
- Validates meeting URLs against known patterns (Google Meet codes: `xxx-yyyy-zzz` format)
- **Decision**: Valid meeting URL found → **MEETING ACTIVE**

## Permissions

**macOS:** Requires **Accessibility** permission for browser tab detection (System Preferences → Security & Privacy → Accessibility). Grant permission to Terminal/Node.js when prompted.

**Note:** No microphone or camera permissions required. Detection uses network activity and browser tabs only.

## Limitations

- macOS only (Intel and Apple Silicon)
- Requires Accessibility permission for browser tab detection
- Polling interval: 2 seconds (not real-time)

## Roadmap

- [ ] Windows support
- [ ] Linux support
- [ ] More meeting apps support (Jitsi, BlueJeans, etc.)
- [ ] Configurable polling interval
- [ ] Performance optimizations
- [ ] Real-time detection (event-driven instead of polling)

## License

MIT

## Contributing

Contributions welcome! This is v1, so there's plenty of room for improvement.

## Support

For issues and questions, please open an issue on GitHub.
