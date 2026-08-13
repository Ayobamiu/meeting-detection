// Smoke test: verifies the published artifact actually loads and works.
//
// This runs in CI and as `npm test`. It must exit on its own — the interactive
// watcher lives in test.js (`npm run watch`).

const assert = require("node:assert");
const { execFileSync } = require("node:child_process");
const path = require("node:path");

const pkg = require("../package.json");
const detection = require("../index.js");

console.log("Running meeting-detection smoke test\n");

// 1. The binary that ships to users must carry both macOS architectures.
const universalBinary = path.join(
  __dirname,
  "..",
  "meeting-detection.darwin-universal.node"
);
const shippedBinary = pkg.files.find((file) => file.endsWith(".node"));
assert.strictEqual(
  shippedBinary,
  "meeting-detection.darwin-universal.node",
  "package.json must ship the universal binary"
);

const architectures = execFileSync("lipo", ["-archs", universalBinary], {
  encoding: "utf8",
}).trim();
for (const arch of ["x86_64", "arm64"]) {
  assert.ok(
    architectures.split(/\s+/).includes(arch),
    `universal binary is missing ${arch} (got: ${architectures})`
  );
}
console.log(`✓ universal binary covers ${architectures}`);

// 2. Nothing may run a Rust build on the user's machine at install time.
assert.strictEqual(
  pkg.scripts.install,
  undefined,
  "package.json must not define an install script — consumers have no Rust toolchain"
);
console.log("✓ no install script");

// 3. The public API is present.
for (const name of [
  "init",
  "isMeetingActive",
  "onMeetingStart",
  "onMeetingEnd",
  "getLastDetectionDetails",
]) {
  assert.strictEqual(
    typeof detection[name],
    "function",
    `missing export: ${name}`
  );
}
console.log("✓ exports: init, isMeetingActive, onMeetingStart, onMeetingEnd, getLastDetectionDetails");

// 4. A detection cycle runs and returns a well-formed result.
detection.init();

const active = detection.isMeetingActive();
assert.strictEqual(typeof active, "boolean", "isMeetingActive must return a boolean");
console.log(`✓ isMeetingActive() -> ${active}`);

const details = detection.getLastDetectionDetails();
assert.ok(details, "getLastDetectionDetails must return a result after init()");
assert.strictEqual(typeof details.active, "boolean");
assert.strictEqual(typeof details.reason, "string");
assert.ok(details.signals, "details.signals must be present");
console.log(`✓ getLastDetectionDetails() -> reason=${details.reason}`);

// 5. Cached reads must be cheap. A fresh detection shells out to lsof and
// AppleScript; serving that from the calling thread froze Electron's main
// process for seconds.
const start = process.hrtime.bigint();
for (let i = 0; i < 100; i++) {
  detection.isMeetingActive();
}
const perCallMs = Number(process.hrtime.bigint() - start) / 1e6 / 100;
assert.ok(
  perCallMs < 5,
  `isMeetingActive() must be served from cache, took ${perCallMs.toFixed(2)}ms per call`
);
console.log(`✓ isMeetingActive() averages ${perCallMs.toFixed(4)}ms per call`);

// 6. Callback registration must not throw.
detection.onMeetingStart(() => {});
detection.onMeetingEnd(() => {});
console.log("✓ callbacks registered");

console.log("\nAll smoke tests passed.");

// Registered threadsafe callbacks keep the event loop alive, so exit explicitly.
process.exit(0);
