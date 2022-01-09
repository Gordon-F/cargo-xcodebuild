#[cfg(not(target_os = "macos"))]
fn main() {
    compile_error!("Unfortunately, only MacOS is supported. If you need supports for another OS better take a look at https://libimobiledevice.org.");
}

#[cfg(target_os = "macos")]
fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let frameworks_dir = std::path::PathBuf::from(&out_dir).join("Frameworks");
    std::fs::create_dir_all(&frameworks_dir).unwrap();

    let new_framework_location = std::path::PathBuf::from(
        "/Library/Apple/System/Library/PrivateFrameworks/MobileDevice.framework",
    );
    let old_framework_location =
        std::path::PathBuf::from("/System/Library/PrivateFrameworks/MobileDevice.framework");

    let selected_framework_location = {
        if new_framework_location.exists() && new_framework_location.is_dir() {
            new_framework_location
        } else if old_framework_location.exists() && old_framework_location.is_dir() {
            old_framework_location
        } else {
            panic!(
                "Can't find MobileDevice.framework:\n{}\n{}",
                new_framework_location.into_os_string().to_string_lossy(),
                old_framework_location.into_os_string().to_string_lossy()
            )
        }
    };

    // Unfortunately, there is no way to link directly to a private framework
    // But we can workaround this by symlinking.
    std::process::Command::new("ln")
        .current_dir(&frameworks_dir)
        .arg("-sf")
        .arg(selected_framework_location)
        .output()
        .unwrap();

    println!(
        "cargo:rustc-link-search=framework={}",
        frameworks_dir.into_os_string().to_string_lossy()
    );
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=MobileDevice");
}
