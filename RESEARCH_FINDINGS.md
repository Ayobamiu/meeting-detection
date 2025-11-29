# Research Findings: macOS Meeting Detection Methods

## Overview
Research on detecting active meetings on macOS through network connections, audio/video streams, process state, and window state.

---

## 1. Network Connections for Active Video Calls

### Findings

#### **Method 1: `lsof` Command (Recommended)**
- **What it does**: Lists open files/network connections for processes
- **Command**: `lsof -i -P | grep -E "(zoom|teams|webex|google)"`
- **Pros**: 
  - Shows TCP/UDP connections with remote IPs/domains
  - Can filter by process name or connection state
  - Works for all meeting apps
- **Cons**: 
  - Requires parsing text output
  - May need root permissions for some connections
- **Implementation**: Parse `lsof` output in Rust, filter for meeting domains

#### **Method 2: `netstat` Command**
- **What it does**: Shows network connections and routing tables
- **Command**: `netstat -an | grep ESTABLISHED`
- **Pros**: Shows connection state (ESTABLISHED, LISTEN, etc.)
- **Cons**: 
  - Doesn't show process names directly (need to correlate with PID)
  - Less detailed than `lsof`
- **Implementation**: Combine with process detection to match connections

#### **Method 3: Network Framework (Native macOS)**
- **What it does**: Use macOS Network framework APIs
- **Pros**: Native, programmatic access
- **Cons**: 
  - Requires Objective-C/Swift FFI or Rust bindings
  - More complex implementation
  - May require additional permissions
- **Implementation**: Use `Network.framework` via FFI or create Rust bindings

### Meeting Service Domains to Monitor

**Zoom:**
- `zoom.us` (main domain)
- `*.zoom.us` (subdomains)
- `*.zoom.com`
- Common ports: 443 (HTTPS), 8801-8810 (UDP for video)

**Microsoft Teams:**
- `teams.microsoft.com`
- `*.teams.microsoft.com`
- `*.office.com`
- Common ports: 443, 3478-3481 (STUN/TURN)

**Google Meet:**
- `meet.google.com`
- `*.google.com` (but too broad, need specific filtering)
- Common ports: 443, 19302-19309 (UDP for video)

**Webex:**
- `webex.com`
- `*.webex.com`
- `cisco.com` (some Webex services)
- Common ports: 443, 9000-9999 (UDP for video)

### Network Activity Patterns

**Active Meeting Indicators:**
1. **Persistent TCP connections** to meeting domains (ESTABLISHED state)
2. **High data transfer** (bytes sent/received increasing)
3. **UDP connections** for video/audio streaming (common in video calls)
4. **Multiple simultaneous connections** (typical for video conferencing)

**Implementation Approach:**
```rust
// Pseudo-code
1. Run `lsof -i -P -n` to get all network connections
2. Parse output, filter for meeting domains
3. Check connection state (ESTABLISHED = active)
4. Monitor data transfer rates (if available)
5. Count connections per process (more = likely active meeting)
```

---

## 2. Audio/Video Streams: Active Streams

### Findings

#### **Method 1: CoreAudio Framework (Native macOS)**
- **What it does**: Access audio device state and active streams
- **APIs**: 
  - `AudioObjectGetPropertyData` - Get audio device properties
  - `AudioHardwareGetProperty` - Get system audio state
- **Pros**: 
  - Native macOS API
  - Can detect which app is using mic/camera
  - Real-time detection
- **Cons**: 
  - Requires Rust FFI bindings to CoreAudio
  - Complex API
  - May need permissions
- **Implementation**: Create Rust bindings to CoreAudio or use existing crate (`coreaudio-rs`)

#### **Method 2: `lsof` for Device Files**
- **What it does**: Check which processes have audio/video devices open
- **Command**: `lsof /dev/audio*` or `lsof | grep -E "(audio|video)"`
- **Pros**: 
  - Simple, works immediately
  - Shows process names
- **Cons**: 
  - macOS doesn't use `/dev/audio*` (that's Linux)
  - Less reliable on macOS
- **Implementation**: Not recommended for macOS

#### **Method 3: System Indicators**
- **What it does**: macOS shows orange/green dots in menu bar when mic/camera active
- **Pros**: Visual indicator exists
- **Cons**: 
  - No programmatic API to read this directly
  - Would need screen capture or accessibility API
- **Implementation**: Not practical

### Active Stream Detection Strategy

**For Microphone:**
1. Use CoreAudio to enumerate audio input devices
2. Check which processes have active audio input streams
3. Filter for meeting apps (Zoom, Teams, etc.)

**For Camera:**
1. Check for processes accessing camera via system APIs
2. macOS may require Screen Recording permission
3. Alternative: Check for video-related process activity

**Implementation Approach:**
```rust
// Pseudo-code
1. Use coreaudio-rs crate to access CoreAudio
2. Enumerate audio devices
3. Get active audio streams per device
4. Match streams to meeting app processes
5. If meeting app has active stream → likely in meeting
```

**Note**: This is more complex and may require additional permissions. For v1, we might want to keep this simplified.

---

## 3. Process State: Meeting Apps Specific States

### Findings

#### **Method 1: Window Titles (Current Approach)**
- **What it does**: Check window titles for meeting indicators
- **Examples**:
  - Zoom: "Zoom Meeting", "Zoom - [Meeting Name]"
  - Teams: "Microsoft Teams | Call", "Teams | Meeting"
  - Webex: "Cisco Webex Meetings", "Webex - [Meeting]"
- **Pros**: 
  - Simple, already implemented
  - Reliable indicator
- **Cons**: 
  - Window might be minimized
  - Title might not always contain keywords
- **Implementation**: Already done, can enhance

#### **Method 2: Process-Specific Indicators**
- **What it does**: Check for meeting app-specific process states
- **Zoom**: 
  - Process name: `zoom.us`, `ZoomOpener`, `zTsc`
  - Window titles: "Zoom Meeting", "Zoom - [name]"
- **Teams**: 
  - Process name: `Microsoft Teams`, `Teams`
  - Window titles: "Microsoft Teams | Call", "Teams | Meeting"
- **Webex**: 
  - Process name: `webexmta`, `Cisco Webex`
  - Window titles: "Cisco Webex Meetings"
- **Pros**: App-specific, more accurate
- **Cons**: Need to maintain list per app
- **Implementation**: Enhance current process detection

#### **Method 3: Accessibility API**
- **What it does**: Use macOS Accessibility API to get app state
- **Pros**: Can get more detailed app state
- **Cons**: 
  - Requires Accessibility permission
  - More complex
  - May not expose meeting state directly
- **Implementation**: Use AppleScript or Accessibility API via FFI

### Process State Detection Strategy

**For Each Meeting App:**

**Zoom:**
- Check for `zoom.us` process
- Check window titles for "Meeting", "Zoom"
- Check for network connections to `zoom.us`

**Teams:**
- Check for `Microsoft Teams` or `Teams` process
- Check window titles for "Call", "Meeting"
- Check for network connections to `teams.microsoft.com`

**Webex:**
- Check for `webexmta` or `Cisco Webex` process
- Check window titles for "Webex", "Meeting"
- Check for network connections to `webex.com`

**Google Meet (Browser):**
- Check for browser process (Chrome, Safari, Edge)
- Check browser tab URLs for `meet.google.com`
- Check window titles for "Google Meet"

---

## 4. Window State: Active Meeting Windows vs. Idle App Windows

### Findings

#### **Method 1: Window Visibility & Focus**
- **What it does**: Check if window is visible and in focus
- **AppleScript**: 
  ```applescript
  tell application "System Events"
    get visible of windows of process "zoom.us"
    get frontmost of process "zoom.us"
  end tell
  ```
- **Pros**: 
  - Simple via AppleScript
  - Can detect if window is minimized
- **Cons**: 
  - Window might be visible but not active
  - User might be in another app
- **Implementation**: Enhance current window detection

#### **Method 2: Window Title Changes**
- **What it does**: Monitor window title changes
- **Pros**: 
  - Title often changes when meeting starts
  - Example: "Zoom" → "Zoom Meeting"
- **Cons**: 
  - Need to track previous state
  - Not all apps change titles
- **Implementation**: Track window title history

#### **Method 3: User Activity Detection**
- **What it does**: Monitor if user is interacting with the window
- **macOS API**: `HIDIdleTime` or similar
- **Pros**: Can detect idle vs. active
- **Cons**: 
  - Doesn't mean meeting isn't active (user might be listening)
  - More complex
- **Implementation**: Use system idle time APIs

### Window State Detection Strategy

**Active Window Indicators:**
1. Window is visible (not minimized)
2. Window title contains meeting keywords
3. Process is in foreground (frontmost)
4. Window title changed recently (meeting started)

**Idle Window Indicators:**
1. Window is minimized
2. Window title is generic (e.g., just "Zoom" without "Meeting")
3. Process is in background
4. No recent window title changes

**Implementation Approach:**
```rust
// Pseudo-code
1. Get all windows for meeting app processes
2. Check if window is visible
3. Check if window title contains meeting keywords
4. Check if process is frontmost
5. Track window title changes over time
6. Combine indicators to determine active vs. idle
```

---

## Browser Tab Detection: URLs + Window Titles

### Findings

#### **Method 1: AppleScript for Browser URLs**
- **Chrome**: 
  ```applescript
  tell application "Google Chrome"
    get URL of every tab of every window
  end tell
  ```
- **Safari**: 
  ```applescript
  tell application "Safari"
    get URL of every tab of every window
  end tell
  ```
- **Edge**: Similar to Chrome
- **Pros**: 
  - Works for all tabs (not just active)
  - Simple via AppleScript
- **Cons**: 
  - Requires AppleScript execution
  - May be slow for many tabs
- **Implementation**: Use AppleScript, parse URLs

#### **Method 2: Combine with Window Titles**
- **What it does**: Get both URL and window title
- **Pros**: 
  - More accurate (URL + title)
  - Can detect meeting even if tab is minimized
- **Cons**: 
  - Two API calls per browser
- **Implementation**: Get URLs and titles, match both

### Meeting URLs to Detect

**Google Meet:**
- `meet.google.com/*`
- `meet.google.com/xxx-xxxx-xxx`

**Teams (Web):**
- `teams.microsoft.com/_#/conversations/*`
- `teams.microsoft.com/_#/meet/*`
- `teams.live.com/*`

**Zoom (Web):**
- `zoom.us/j/*`
- `zoom.us/s/*`
- `*.zoom.us/j/*`

**Webex (Web):**
- `*.webex.com/webapp/*`
- `*.webex.com/meet/*`
- `meetings.webex.com/*`

---

## Recommended Implementation Priority

### Phase 1 (High Priority - Most Reliable):
1. **Network Connections** via `lsof`
   - Detect connections to meeting domains
   - Check for ESTABLISHED connections
   - Monitor connection count per process

2. **Window Titles** (enhance current)
   - Improve pattern matching
   - Track title changes
   - Check for meeting-specific keywords

3. **Browser Tab URLs** (new)
   - Get URLs from Chrome/Safari/Edge
   - Match against meeting URL patterns
   - Combine with window titles

### Phase 2 (Medium Priority - More Complex):
4. **Process State** (enhance current)
   - App-specific detection logic
   - Better process name matching
   - Combine with network connections

5. **Window State** (enhance current)
   - Check visibility
   - Check if frontmost
   - Track window state changes

### Phase 3 (Lower Priority - Complex/Experimental):
6. **Audio/Video Streams** via CoreAudio
   - Requires FFI bindings
   - May need permissions
   - More complex but very accurate

---

## Implementation Notes

### Permissions Required:
- **Network Monitoring**: May need admin/root for some connections
- **Accessibility**: For window state detection (optional)
- **Screen Recording**: For camera detection (optional, not recommended for v1)

### Performance Considerations:
- `lsof` can be slow with many processes - cache results
- AppleScript for browser URLs - batch calls, don't call every 2 seconds
- Network connection monitoring - can be resource intensive

### Privacy Considerations:
- Network monitoring sees connection destinations (domains, not content)
- Window titles may contain meeting names (sensitive)
- URLs contain meeting IDs (sensitive)
- Consider "safe mode" that doesn't log sensitive data

---

## Next Steps

1. **Implement network connection detection** via `lsof`
2. **Enhance browser tab URL detection** via AppleScript
3. **Improve window title matching** with app-specific patterns
4. **Combine all signals** for more accurate detection
5. **Test with real meetings** to validate accuracy

