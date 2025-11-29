# Quick Start Guide

## For JavaScript Developers New to Rust

This guide will help you understand and work with this Rust + Node.js project.

## Project Structure

```
meeting-detection/
├── Cargo.toml          # Rust dependencies (like package.json)
├── package.json        # Node.js package config
├── build.rs            # Build script for napi-rs
├── src/                # Rust source code
│   ├── lib.rs          # Main entry, JavaScript bindings
│   ├── detector.rs     # Core detection logic
│   ├── config.rs       # Meeting app configurations
│   ├── error.rs        # Error types
│   └── platform/       # Platform-specific code
│       ├── mod.rs      # Platform abstraction
│       ├── macos.rs    # macOS implementation
│       ├── windows.rs  # Windows implementation
│       └── linux.rs    # Linux implementation
├── test.js             # Test file
└── README.md           # Full documentation
```

## Key Rust Concepts Used

### 1. **Traits** (like interfaces in TypeScript)
```rust
// Similar to: interface PlatformDetector { ... }
pub trait PlatformDetector {
    fn is_microphone_active(&self) -> Result<bool, DetectionError>;
    // ...
}
```

### 2. **Result<T, E>** (like try/catch)
```rust
// Similar to: try { ... } catch (e) { ... }
fn detect() -> Result<bool, DetectionError> {
    // Returns Ok(value) on success, Err(error) on failure
}
```

### 3. **Arc** (Atomic Reference Counting)
```rust
// Similar to: shared references in JS (but thread-safe)
let shared = Arc::new(data);
let clone = Arc::clone(&shared); // Both point to same data
```

### 4. **Mutex** (mutual exclusion)
```rust
// Similar to: locks in JS (but for thread safety)
let data = Mutex::new(vec![]);
let mut guard = data.lock().unwrap(); // Lock, modify, auto-unlock
```

## Building the Project

### First Time Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Node.js dependencies**:
   ```bash
   npm install
   ```

3. **Build the Rust code**:
   ```bash
   npm run build
   ```

   This will:
   - Compile Rust code to a `.node` file (native addon)
   - Generate TypeScript definitions
   - Create `index.js` and `index.d.ts`

### Development Workflow

1. **Make changes to Rust code** in `src/`
2. **Rebuild**:
   ```bash
   npm run build
   ```
3. **Test**:
   ```bash
   node test.js
   ```

## Common Tasks

### Adding a New Meeting App

Edit `src/config.rs`:
```rust
pub fn get_meeting_app_processes() -> Vec<&'static str> {
    vec![
        // ... existing apps
        "new-app",  // Add here
    ]
}
```

### Adjusting Detection Threshold

Edit `src/detector.rs`:
```rust
const THRESHOLD: i32 = 3; // Change this value
```

### Adding Platform-Specific Code

1. Edit the appropriate file in `src/platform/` (macos.rs, windows.rs, or linux.rs)
2. Implement the trait method
3. Rebuild: `npm run build`

## Debugging

### Enable Logging

Set environment variable:
```bash
RUST_LOG=debug node test.js
```

### Check Compilation Errors

```bash
cargo check  # Just check, don't build
cargo build  # Full build with errors
```

### Common Issues

1. **"Permission denied" errors**: 
   - macOS: Grant permissions in System Preferences
   - Windows: Check Privacy settings
   - Linux: May need to install `xdotool` or `wmctrl`

2. **Build fails**:
   - Make sure Rust is installed: `rustc --version`
   - Make sure you're on the right platform
   - Check `Cargo.toml` for correct dependencies

3. **"Module not found"**:
   - Run `npm install` first
   - Then `npm run build`

## Testing

Run the test file:
```bash
node test.js
```

This will:
- Initialize the engine
- Show current meeting status
- Listen for meeting start/end events
- Poll status every 5 seconds

## Next Steps

1. Read `README.md` for full API documentation
2. Check `src/detector.rs` to understand the detection logic
3. Modify `test.js` to test your use case
4. Integrate into your Electron/Node.js app

## Getting Help

- **Rust Book**: https://doc.rust-lang.org/book/
- **napi-rs Docs**: https://napi.rs/
- **Project Issues**: Open an issue on GitHub

