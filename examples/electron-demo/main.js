// Electron main process.
//
// meeting-detection is a native addon, so it can only be loaded here — the
// renderer cannot require it. Events are forwarded to the UI over IPC.
//
//   npm start              live demo window
//   npm run selftest       headless assertions, exits non-zero on failure

const { app, BrowserWindow, ipcMain } = require("electron");
const path = require("node:path");

const detection = require("meeting-detection");

const SELFTEST = process.argv.includes("--selftest");
const POLL_INTERVAL_MS = 1000;

let mainWindow = null;
let pollTimer = null;
let lastActive = null;

/** Everything the self-test needs to observe, recorded as the app runs. */
const observed = {
  moduleLoaded: false,
  initCalled: false,
  isMeetingActiveType: null,
  detailsShape: null,
  rendererAcks: 0,
  events: [],
  cachedCallMs: null,
};

function log(...args) {
  console.log("[main]", ...args);
}

function send(channel, payload) {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(channel, payload);
  }
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 720,
    height: 640,
    title: "Meeting Detection Demo",
    show: !SELFTEST,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.loadFile(path.join(__dirname, "index.html"));
}

function startDetection() {
  observed.moduleLoaded = typeof detection.init === "function";

  detection.init();
  observed.initCalled = true;
  log("engine initialized");

  detection.onMeetingStart((_, details) => {
    const event = {
      type: "start",
      at: new Date().toISOString(),
      appName: details?.appName ?? null,
      reason: details?.reason ?? null,
      meetingUrl: details?.meetingUrl ?? null,
    };
    observed.events.push(event);
    log("MEETING STARTED", JSON.stringify(event));
    send("meeting-event", event);
  });

  detection.onMeetingEnd((_, details) => {
    const event = {
      type: "end",
      at: new Date().toISOString(),
      appName: details?.appName ?? null,
      reason: details?.reason ?? null,
    };
    observed.events.push(event);
    log("MEETING ENDED", JSON.stringify(event));
    send("meeting-event", event);
  });

  // Poll for the status card. This is the call that used to block the main
  // thread for seconds per invocation.
  pollTimer = setInterval(() => {
    const start = process.hrtime.bigint();
    const active = detection.isMeetingActive();
    observed.cachedCallMs = Number(process.hrtime.bigint() - start) / 1e6;

    if (active !== lastActive) {
      log(`status changed -> ${active ? "IN A MEETING" : "not in a meeting"}`);
      lastActive = active;
    }

    send("status", {
      active,
      details: detection.getLastDetectionDetails(),
      pollCostMs: observed.cachedCallMs,
    });
  }, POLL_INTERVAL_MS);
}

// Renderer confirms it received and rendered an update, which is what proves
// the IPC path works rather than just that main can call the addon.
ipcMain.on("renderer-ack", (_event, payload) => {
  observed.rendererAcks += 1;
  if (observed.rendererAcks === 1) {
    log("renderer acknowledged first status update:", JSON.stringify(payload));
  }
});

ipcMain.handle("get-details", () => detection.getLastDetectionDetails());

async function runSelfTest() {
  const failures = [];
  const check = (name, condition, detail) => {
    if (condition) {
      console.log(`  PASS  ${name}${detail ? ` — ${detail}` : ""}`);
    } else {
      console.log(`  FAIL  ${name}${detail ? ` — ${detail}` : ""}`);
      failures.push(name);
    }
  };

  console.log("\nElectron self-test");
  console.log(`  electron ${process.versions.electron}, node ${process.versions.node}, abi ${process.versions.modules}\n`);

  check("native addon loads in the main process", observed.moduleLoaded);
  check("init() runs without throwing", observed.initCalled);

  const active = detection.isMeetingActive();
  observed.isMeetingActiveType = typeof active;
  check(
    "isMeetingActive() returns a boolean",
    observed.isMeetingActiveType === "boolean",
    `got ${observed.isMeetingActiveType} (${active})`
  );

  const details = detection.getLastDetectionDetails();
  observed.detailsShape = details ? Object.keys(details).sort().join(",") : null;
  check("getLastDetectionDetails() returns a result", details !== null);
  check(
    "details carry a reason and signals",
    Boolean(details && typeof details.reason === "string" && details.signals),
    details ? `reason=${details.reason}` : "no details"
  );

  // The main thread must not stall. Before the fix this averaged seconds.
  const start = process.hrtime.bigint();
  for (let i = 0; i < 200; i++) detection.isMeetingActive();
  const perCallMs = Number(process.hrtime.bigint() - start) / 1e6 / 200;
  check(
    "isMeetingActive() does not block the main thread",
    perCallMs < 5,
    `${perCallMs.toFixed(4)}ms per call`
  );

  check(
    "IPC delivers status updates to the renderer",
    observed.rendererAcks > 0,
    `${observed.rendererAcks} acks received`
  );

  console.log(
    `\n${failures.length === 0 ? "All checks passed." : `${failures.length} check(s) failed: ${failures.join(", ")}`}\n`
  );

  clearInterval(pollTimer);
  app.exit(failures.length === 0 ? 0 : 1);
}

app.whenReady().then(() => {
  createWindow();
  startDetection();

  if (SELFTEST) {
    // Give the poller time to complete a cycle and the renderer time to ack.
    setTimeout(runSelfTest, 6000);
  }
});

app.on("window-all-closed", () => {
  clearInterval(pollTimer);
  app.quit();
});
