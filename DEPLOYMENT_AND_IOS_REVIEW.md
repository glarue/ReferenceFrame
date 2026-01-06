# Deployment & iOS Implementation Review

## 1. GitHub Pages Hosting (WASM Web Version)

### Current Status
- ✅ **PyScript version** is already deployed at https://glarue.github.io/ReferenceFrame
- ✅ **GitHub Actions workflow** exists (`.github/workflows/deploy.yml`)
- ❌ **WASM version** is NOT currently deployed

### Can we deploy the WASM version? **YES - Very Easy**

The WASM web version (`platforms/web/`) is **perfectly suited** for GitHub Pages because:
- ✅ All static files (HTML, CSS, JS, WASM)
- ✅ No server-side processing needed
- ✅ No build process required at deploy time (pre-built WASM binaries in `pkg/`)
- ✅ Smaller payload than PyScript (~220 KB vs ~30 MB)

### Deployment Options

#### Option A: Replace PyScript Version
**Simple but loses the working PyScript app**

Modify `.github/workflows/deploy.yml`:
```yaml
- name: Prepare deployment files
  run: |
    mkdir -p deploy
    cp platforms/web/index.html platforms/web/styles.css platforms/web/storage.js deploy/
    cp -r platforms/web/pkg deploy/
```

**Trade-offs:**
- ✅ Single URL, simpler
- ❌ Loses proven PyScript implementation
- ❌ Goes against project strategy of keeping both

#### Option B: Deploy as Subdirectory (RECOMMENDED)
**Both versions coexist at different URLs**

```yaml
- name: Prepare deployment files
  run: |
    mkdir -p deploy
    # PyScript version at root
    cp index.html manifest.json sw.js .nojekyll styles.css app.js deploy/
    cp -r src deploy/
    # WASM version at /wasm
    mkdir -p deploy/wasm
    cp platforms/web/index.html platforms/web/styles.css platforms/web/storage.js deploy/wasm/
    cp -r platforms/web/pkg deploy/wasm/
```

**URLs:**
- PyScript: https://glarue.github.io/ReferenceFrame (existing)
- WASM: https://glarue.github.io/ReferenceFrame/wasm (new)

**Trade-offs:**
- ✅ Keeps both implementations available
- ✅ Aligns with project strategy
- ✅ Users can choose based on needs (mature PyScript vs faster WASM)
- ✅ A/B testing possible
- ⚠️ Slightly more complex deployment

#### Option C: Separate Repository
**New repo for WASM version only**

Create `ReferenceFrame-WASM` repository with dedicated Pages deployment.

**Trade-offs:**
- ✅ Clean separation
- ✅ Independent versioning
- ❌ Splits project maintenance
- ❌ More repos to manage

### Recommendation: **Option B (Subdirectory)**
- Maintains both implementations as per project strategy
- Minimal changes to existing deployment
- Allows gradual migration/testing

---

## 2. iOS Implementation Requirements

### Current State
- ✅ **Rust core** complete (2,400 LOC, 108 tests passing)
- ✅ **SVG visualization** complete (56 tests passing)
- ⏸️ **Mobile platform** planned but not started
- ⏸️ **Flutter mockup** exists but Flutter SDK not installed

### Two Possible Approaches

#### Approach A: Flutter (Cross-Platform) - CURRENT PLAN
**Status:** Documented but not implemented

**What's needed:**
1. ✅ Rust core (already complete)
2. ❌ Flutter SDK installation
3. ❌ `flutter_rust_bridge` FFI bindings generation
4. ❌ Flutter UI implementation (~400-600 LOC Dart)
5. ❌ iOS-specific packaging/signing
6. ❌ App Store submission

**Pros:**
- iOS + Android from single UI codebase
- ~79% code sharing (Rust core + Flutter UI)
- Documented approach (`platforms/mobile/README.md`)

**Cons:**
- Requires Flutter SDK (~1.5 GB download)
- Learning curve if not familiar with Flutter/Dart
- FFI layer adds complexity
- Bridge maintenance (flutter_rust_bridge updates)

**Estimated effort:** 2-3 weeks for working iOS + Android apps

#### Approach B: Native Swift (iOS-Only) - ALTERNATIVE
**Status:** Not currently planned

**What's needed:**
1. ✅ Rust core (already complete)
2. ❌ Swift FFI bindings (via `uniffi-rs` or manual)
3. ❌ SwiftUI implementation (~400-600 LOC)
4. ❌ iOS packaging/signing
5. ❌ App Store submission

**Pros:**
- Pure Swift - native iOS experience
- No Flutter dependency
- Smaller app size
- Better iOS integration (SwiftUI, iOS-specific features)

**Cons:**
- iOS only (Android would need separate Kotlin implementation)
- ~21% code duplication if Android needed later
- No existing plan/mockup

**Estimated effort:** 1-2 weeks for iOS-only app

### Key Implementation Details (Either Approach)

#### Shared Components (Already Complete ✅)
- Frame design calculations
- Unit conversion (inches ↔ mm)
- Input parsing (fractions: "12 3/4" → 12.75)
- Aspect ratio management
- Shareable URL encoding/decoding
- SVG visualization generation
- Validation rules

#### Platform-Specific Work Needed (Either Approach ❌)
1. **UI Layer** (~400-600 LOC):
   - Input forms (artwork size, mat width, frame dimensions, etc.)
   - Results display
   - Visualization rendering (SVG → native view)
   - Settings/preferences

2. **State Management**:
   - Form state
   - Saved configurations (iOS: UserDefaults vs Flutter: SharedPreferences)
   - Custom sizes storage

3. **PDF Export** (if desired):
   - iOS: PDFKit integration
   - Flutter: pdf package + integration

4. **Sharing**:
   - iOS: UIActivityViewController
   - Flutter: share_plus package

5. **Deep Linking** (for shareable URLs):
   - iOS: Universal Links
   - Flutter: uni_links package

#### Flutter-Specific Steps (Approach A)

**Setup (one-time):**
```bash
# 1. Install Flutter SDK (~1.5 GB)
git clone https://github.com/flutter/flutter.git -b stable --depth 1
export PATH="$PATH:`pwd`/flutter/bin"

# 2. Install flutter_rust_bridge
cargo install flutter_rust_bridge_codegen

# 3. Initialize Flutter project
cd platforms/mobile
flutter create --org com.referenceframe .

# 4. Add dependencies to pubspec.yaml
flutter pub add flutter_rust_bridge
flutter pub add ffi

# 5. Generate FFI bindings
cd rust_bridge
flutter_rust_bridge_codegen generate
```

**Development workflow:**
```bash
# 1. Modify Rust core if needed
cd core
cargo test

# 2. Regenerate bindings
cd ../platforms/mobile/rust_bridge
flutter_rust_bridge_codegen generate

# 3. Run Flutter app
cd ..
flutter run -d <ios-simulator-id>
```

#### Swift-Specific Steps (Approach B)

**Setup (one-time):**
```bash
# 1. Create Xcode project (via Xcode UI)

# 2. Add Rust as static library
cd core
cargo build --target aarch64-apple-ios --release
cargo build --target x86_64-apple-ios --release  # simulator

# 3. Create fat binary (or use XCFramework)
lipo -create \
  target/aarch64-apple-ios/release/libreferenceframe_core.a \
  target/x86_64-apple-ios/release/libreferenceframe_core.a \
  -output libreferenceframe.a

# 4. Link in Xcode project + generate Swift bindings
```

**Development workflow:**
```bash
# 1. Modify Rust core if needed
cd core
cargo test

# 2. Rebuild for iOS
./build_ios.sh  # script to rebuild fat binary

# 3. Build/run in Xcode
open ios/ReferenceFrame.xcodeproj
```

### Recommendation: **Flutter (Approach A)** - Aligns with current plan
- Project already documents this approach
- Gets both iOS + Android
- Mockup exists (`platforms/mobile/flutter_mockup/`)
- Only blocker is Flutter SDK installation

**If iOS-only is acceptable:** Swift would be faster and simpler

---

## 3. Summary

### GitHub Pages Deployment: **Ready Now**
- WASM web version can deploy immediately
- Recommended: Deploy at `/wasm` subdirectory alongside PyScript
- 1-2 hour effort to update GitHub Actions workflow

### iOS Implementation: **2-3 weeks with Flutter, 1-2 weeks with Swift**
- Rust core is 100% complete
- Only UI layer and platform integration needed
- Flutter = iOS + Android (~400-600 LOC Dart)
- Swift = iOS only (~400-600 LOC Swift)
- Both approaches share 79%+ code via Rust core

### Next Steps (If Pursuing Both)

**Short-term (this week):**
1. Update `.github/workflows/deploy.yml` to deploy WASM at `/wasm`
2. Test deployment at `https://glarue.github.io/ReferenceFrame/wasm`

**Medium-term (2-3 weeks):**
1. Choose Flutter vs Swift approach
2. Install required SDK
3. Implement UI layer
4. Test on device/simulator
5. Prepare for App Store submission

---

**Last Updated:** 2026-01-06
**Status:** WASM ready to deploy, iOS implementation well-positioned to start
