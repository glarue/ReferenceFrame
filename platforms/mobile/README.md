# Mobile Platform (iOS + Android)

**Status**: Planning Phase (not yet implemented)

This directory will contain the Flutter mobile application that shares the same Rust core logic as the web platform.

📋 **See [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md) for comprehensive implementation guide**

## Planned Architecture

```
mobile/
├── lib/                    # Flutter/Dart UI code
│   ├── main.dart
│   └── screens/
├── rust_bridge/            # flutter_rust_bridge FFI wrapper
│   ├── src/
│   │   └── api.rs          # Re-exports from core with FFI annotations
│   └── Cargo.toml
├── ios/                    # iOS-specific code
├── android/                # Android-specific code
└── pubspec.yaml           # Flutter dependencies
```

## Setup (when ready to implement)

1. Install Flutter SDK
2. Install flutter_rust_bridge_codegen:
   ```bash
   cargo install flutter_rust_bridge_codegen
   ```

3. Initialize Flutter project:
   ```bash
   flutter create --org com.referenceframe .
   ```

4. Generate FFI bindings:
   ```bash
   flutter_rust_bridge_codegen generate
   ```

## Shared Core

The mobile app will use the same `core/` Rust library as the web platform, ensuring consistent calculations and business logic across all platforms.
