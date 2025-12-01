# Electron JS usage

**Electron has two types of processes:**

1. **Main process** (backend): Runs Node.js, controls the app lifecycle, can use native modules.
2. **Renderer process** (frontend): Runs your UI (HTML/CSS/JS), like a browser tab, cannot use native modules.

**Why it matters:**

- This project is a native addon (Rust compiled to Node.js), so it only works in the main process.
- Your UI code runs in the renderer process and cannot directly call `onMeetingStart()`.

**Solution — IPC (Inter-Process Communication):**

```javascript
// MAIN PROCESS (main.js)
const { init, onMeetingStart, onMeetingEnd } = require("meeting-detection");
const { ipcMain } = require("electron");

init();

onMeetingStart((details) => {
  // Send event to renderer (UI)
  mainWindow.webContents.send("meeting-started", details);
});

// RENDERER PROCESS (renderer.js)
const { ipcRenderer } = require("electron");

// Listen for events from main process
ipcRenderer.on("meeting-started", (event, details) => {
  console.log("Meeting started:", details.appName);
  // Update your UI here
});
```

**In short:** The detection runs in the main process. Use IPC to forward events to the renderer so the UI can react.
