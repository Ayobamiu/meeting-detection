# Meeting Detection Engine

A macOS meeting detection engine for Electron and Node.js applications. Built with Rust and exposed to JavaScript via napi-rs.

## Features

- ✅ Two-tier decision tree algorithm for accurate detection
- ✅ Event-based API (`onMeetingStart`, `onMeetingEnd`)
- ✅ Simple JavaScript API

## Supported Platforms & Services

| Service             | Native App | Browser | Detection Method                                                                                                             |
| ------------------- | ---------- | ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **Zoom**            | ✅         | ✅      | Tier 1: Network connections (UDP port 8801)<br>Tier 2: URL patterns (`zoom.us/j/`, `zoom.us/s/`)                             |
| **Google Meet**     | ❌         | ✅      | Tier 2: URL patterns with meeting code validation (`meet.google.com/xxx-yyyy-zzz`)                                           |
| **Microsoft Teams** | ✅         | ✅      | Tier 1: Network connections (STUN/TURN ports)<br>Tier 2: URL patterns (`teams.live.com/v2/`, `teams.microsoft.com/_#/meet/`) |
| **Webex**           | ✅         | ✅      | Tier 1: Network connections (video ports)<br>Tier 2: URL patterns (`*.webex.com/webapp/`, `*.webex.com/meet/`)               |

**Supported Operating Systems:**

- **macOS** ✅ (Intel and Apple Silicon)

## Installation

```bash
npm install meeting-detection
```

The package will automatically build native binaries for your platform during installation.

## Usage

### Basic Example

```javascript
const {
  isMeetingActive,
  onMeetingStart,
  onMeetingEnd,
  init,
  getLastDetectionDetails,
} = require("meeting-detection");

// Initialize the engine (starts background polling every 2 seconds)
init();

// Check current status
const active = isMeetingActive();
console.log("Meeting active:", active);

// Get detailed detection information
const details = getLastDetectionDetails();
if (details) {
  console.log("App:", details.appName);
  console.log("Reason:", details.reason);
  if (details.meetingUrl) {
    console.log("URL:", details.meetingUrl);
  }
}

// Listen for meeting start
onMeetingStart((_, details) => {
  console.log("🎥 Meeting started!");
  console.log("App:", details.appName);
  // Do something when meeting starts
});

// Listen for meeting end
onMeetingEnd((_, details) => {
  console.log("✅ Meeting ended!");
  // Do something when meeting ends
});
```

### Electron Example

```javascript
const { app, BrowserWindow } = require("electron");
const {
  isMeetingActive,
  onMeetingStart,
  onMeetingEnd,
  init,
} = require("meeting-detection");

app.whenReady().then(() => {
  init();

  // Update UI when meeting status changes
  onMeetingStart((_, details) => {
    console.log("User is in a meeting:", details.appName);
    // Update your app UI
  });

  onMeetingEnd((_, details) => {
    console.log("User is no longer in a meeting");
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
  console.log("User is in a meeting");
}
```

### `onMeetingStart(callback: (error: null, details: JsDetectionDetails) => void): void`

Register a callback that will be called when a meeting is detected to start. The callback follows Node.js error-first convention: first parameter is always `null` (no error), second parameter contains the detection details.

**Example:**

```javascript
onMeetingStart((error, details) => {
  // error is always null, details contains the detection information
  console.log("Meeting started!");
  console.log("App:", details.appName);
  console.log("Reason:", details.reason);
  // Your logic here
});

// Or ignore the error parameter:
onMeetingStart((_, details) => {
  console.log("Meeting started!");
  console.log("App:", details.appName);
});
```

### `onMeetingEnd(callback: (error: null, details: JsDetectionDetails) => void): void`

Register a callback that will be called when a meeting is detected to end. The callback follows Node.js error-first convention: first parameter is always `null` (no error), second parameter contains the detection details.

**Example:**

```javascript
onMeetingEnd((error, details) => {
  // error is always null, details contains the detection information
  console.log("Meeting ended!");
  // Your logic here
});

// Or ignore the error parameter:
onMeetingEnd((_, details) => {
  console.log("Meeting ended!");
});
```

### `getLastDetectionDetails(): JsDetectionDetails | null`

Get detailed information about the last detection cycle. Useful for debugging and understanding why the engine thinks a meeting is active or not.

**Returns:**

- `null` if no detection has been performed yet
- `JsDetectionDetails` object (see Type Reference below)

**Example:**

```javascript
const details = getLastDetectionDetails();
if (details) {
  console.log("Active:", details.active);
  console.log("App:", details.appName);
  console.log("Reason:", details.reason);
}
```

## Type Reference

### `JsDetectionDetails`

The detection details object returned by callbacks and `getLastDetectionDetails()`. This type is exported and can be imported in TypeScript projects.

```typescript
interface JsDetectionDetails {
  active: boolean; // Whether a meeting is currently active
  score: number; // Legacy scoring (for backward compatibility)
  appName?: string; // Name of the meeting app (e.g., "Zoom", "Safari", "Chrome")
  reason: string; // Detection reason (e.g., "NativeAppWithNetwork(Zoom)", "BrowserWithMeetingUrl(Safari)")
  meetingUrl?: string; // Meeting URL if detected in browser
  signals: SignalsBreakdown; // Breakdown of detection signals
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

**Properties:**

- `active`: `boolean` - Whether a meeting is currently active
- `appName`: `string | undefined` - Name of the meeting app (e.g., "Zoom", "Safari", "Chrome", "Microsoft Edge")
- `reason`: `string` - Detection reason:
  - `"NativeAppWithNetwork(Zoom)"` - Native app detected with active network connections
  - `"BrowserWithMeetingUrl(Safari)"` - Browser tab with meeting URL detected
  - `"None"` - No meeting detected
- `meetingUrl`: `string | undefined` - Full meeting URL if detected in browser (e.g., `"https://meet.google.com/abc-def-ghi"`)
- `score`: `number` - Legacy scoring system (kept for backward compatibility, not used in decision logic)
- `signals`: `SignalsBreakdown` - Breakdown of individual detection signals (for debugging)

**Example:**

```typescript
import { JsDetectionDetails } from "meeting-detection";

onMeetingStart((_, details: JsDetectionDetails) => {
  if (details.active) {
    console.log(`Meeting active in ${details.appName}`);
    if (details.meetingUrl) {
      console.log(`URL: ${details.meetingUrl}`);
    }
  }
});
```

## How It Works

The engine uses a **two-tier decision tree** to accurately detect active meetings:

### Tier 1: Native Meeting Apps (Zoom, Teams desktop, Webex desktop)

- Detects if a native meeting app process is running
- Checks for active network connections to meeting domains
- **Decision**: If network activity detected → **MEETING ACTIVE**

### Tier 2: Browser-Based Meetings (Google Meet, Teams web, Webex web)

- Detects if a browser has a tab with a meeting URL
- Validates meeting URLs (e.g., Google Meet codes must match `xxx-yyyy-zzz` format)
- **Decision**: If valid meeting URL found → **MEETING ACTIVE**

### Detection Signals

1. **Process Detection**: Checks if known meeting applications are running (Zoom, Teams, Webex)
2. **Network Connection Detection**: Monitors active connections to meeting service domains and ports
   - Zoom: UDP port 8801, connections to `zoom.us`
   - Teams: STUN/TURN ports, connections to `teams.microsoft.com`
   - Webex: Video ports, connections to `webex.com`
3. **Browser Tab Detection**: Uses AppleScript to retrieve URLs from Chrome, Safari, and Edge tabs
4. **URL Pattern Matching**: Validates meeting URLs against known patterns
   - Google Meet: `meet.google.com/xxx-yyyy-zzz` (with code validation)
   - Teams: `teams.live.com/v2/`, `teams.microsoft.com/_#/meet/`
   - Webex: `*.webex.com/webapp/`, `*.webex.com/meet/`

## Permissions

### macOS

The app may need the following permissions:

- **Accessibility**: For browser tab URL detection (System Preferences → Security & Privacy → Accessibility)
  - Required for AppleScript to access browser tabs
  - Grant permission to Terminal/Node.js when prompted

**Note**: The engine does not require microphone or camera permissions. It detects meetings through network activity and browser tabs.

## Limitations

- **macOS only**: Currently supports macOS (Intel and Apple Silicon) only
- **Browser detection**: Requires Accessibility permission for AppleScript to access browser tabs
- **Network detection**: Uses `lsof` command which may require appropriate system permissions
- **Polling interval**: Detection runs every 2 seconds (not real-time)

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
