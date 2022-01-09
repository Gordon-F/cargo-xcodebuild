## cargo-xcodebuild

Helps cargo build and run apps for iOS. 📦 ⚙️ 🍏

### Setup

You need to install `Xcode` (*NOT* just Command Line Tools!), `xcodegen`, `cargo-xcodebuild`, and required rust targets.

#### 1. `Xcode`
Install via [App Store](https://apps.apple.com/app/xcode/id497799835).

#### 2. `xcodegen`
```shell
brew install xcodegen
```
Check `xcodegen` [installation guide](https://github.com/yonaskolb/XcodeGen#installing) for other options.

#### 2. `cargo-xcodebuild`:

Release version:
```shell
cargo install cargo-xcodebuild
```

Git version:
```shell
cargo install --git https://github.com/Gordon-F/cargo-xcodebuild cargo-xcodebuild
```

#### 4. Install required rust targets:
- `aarch64-apple-ios`: iOS devices
- `x86_64-apple-ios`: iOS simulator on x86 processors
- `aarch64-apple-ios-sim`: iOS simulator on Apple processors

```shell
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

### Commands
- `check`: Checks that the current package builds without creating xcodeproject
- `build`: Compiles the current package and create xcodeproject
- `run`: Run a project on device or simulator
- `generate` Generate xcodeproject without building it
- `open`: Open generated project with Xcode
- `devices`: List of booted simulator devices and connected devices
- `teams`: List of signing teams
- `boot`: Boot a simulator with specific device id

For example:
```shell
# Boot a simulator
cargo xcodebuild boot 4F57337E-1AF2-4D30-9726-87040063C016
# Run on avaliable device or simulator
cargo xcodebuild run
```

### Project setup

Make sure your `Cargo.toml` contains `staticlib` crate type and required build targets:
```toml
[package]
name = "cargo_xcodebuild_minimal_example"
version = "0.1.0"
edition = "2021"

[lib]
# Required
crate-type = ["staticlib"]

[dependencies]

[package.metadata.ios]
# Required
build_targets = ["aarch64-apple-ios"]
```

And `lib.rs` contains `main_rs` function:
```rust
#[no_mangle]
pub extern "C" fn main_rs() {
    // start game code here
}
```

Following instruction to run on a [simulator](https://github.com/Gordon-F/cargo-xcodebuild/wiki/Running-on-simulator) or a [device](https://github.com/Gordon-F/cargo-xcodebuild/wiki/Running-on-device).

### Manifest
Following configuration options are supported by `cargo xcodebuild` under `[package.metadata.ios]`:

```toml
[package.metadata.ios]
# Specifies the array of targets to build for.
build_targets = ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"]

# Specifies the array of dependencies sdk. Empty by default.
dependencies = ["OpenGLES.framework", "GLKit.framework", "Security.framework", "UIKit.framework"]

# Specifies deployment target. "12" by default.
deployment_target = "13.0"

# Specifies bundleIdPrefix. "com.rust" by default.
bundle_id_prefix = "com.rust.game"

# Specifies CODE_SIGN_IDENTITY.
code_sign_identity = "Apple Developer"

# Specifies DEVELOPMENT_TEAM.
development_team = "XXXXXX"

# Specifies device id and device type.
device_id = "XXXXXX"
device_type = "simulator" # or "device".

# Specifies an assets folder.
assets = ["assets/"]
```

Feel free to create an issue/PR if you need more!

### Examples
1. [`wgpu`](examples/wgpu)
2. [`bevy`](examples/bevy)
3. [`miniquad`](examples/miniquad)
4. [`macroquad`](examples/macroquad)


Inspired by a similar tool for Android - [`cargo apk`](https://github.com/rust-windowing/android-ndk-rs/tree/master/cargo-apk).
