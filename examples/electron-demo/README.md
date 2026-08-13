# Electron demo

A minimal Electron app that consumes `meeting-detection`, for trying the engine
and for verifying a release before publishing.

```bash
cd examples/electron-demo
npm install
npm start
```

The window shows live status, the last detection details, and a log of
meeting start/end events. Join or leave a meeting and watch it change.

## What it demonstrates

The addon is native, so it loads **only in the main process** — the renderer
cannot `require` it. `main.js` owns the engine and forwards events over IPC;
`preload.js` exposes a narrow API to the page behind `contextIsolation`. That
is the pattern to copy into a real app.

## Self-test

```bash
npm run selftest
```

Runs headless for a few seconds, asserts the integration, and exits non-zero on
failure. It checks that the addon loads under Electron's ABI, that `init()`
works, that `isMeetingActive()` returns a boolean without blocking the main
thread, and that status updates actually reach the renderer over IPC.

The main-thread check matters: `isMeetingActive()` used to run a full detection
synchronously on every call, which froze the UI for seconds at a time.

## Note on permissions

The first time the demo reads browser tabs, macOS asks for **Automation**
permission for Electron (System Settings → Privacy & Security → Automation).
Decline it and native-app detection still works, but browser-based meetings
will not be detected. A packaged app asks for this under its own name.
