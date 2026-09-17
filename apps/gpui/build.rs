fn main() {
    #[cfg(target_os = "macos")]
    macos::build();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::process::Command;

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
        let fetch = manifest.join("../../script/fetch_sparkle.sh");
        let status = Command::new("bash")
            .arg(&fetch)
            .status()
            .expect("run script/fetch_sparkle.sh");
        assert!(
            status.success(),
            "script/fetch_sparkle.sh failed; Sparkle is required for macOS Native builds"
        );

        let vendor = manifest.join("vendor/Sparkle");
        let framework = vendor.join("Sparkle.framework");
        assert!(
            framework.is_dir(),
            "Sparkle.framework missing at {}",
            framework.display()
        );

        println!("cargo:rerun-if-changed=native/sparkle_bridge.m");
        println!("cargo:rerun-if-changed=native/sparkle_bridge.h");
        println!("cargo:rerun-if-changed=../../script/fetch_sparkle.sh");

        cc::Build::new()
            .file("native/sparkle_bridge.m")
            .flag("-fobjc-arc")
            .flag("-fmodules")
            .flag(format!("-F{}", vendor.display()))
            .flag("-mmacosx-version-min=14.0")
            .compile("mdow_sparkle_bridge");

        println!("cargo:rustc-link-search=framework={}", vendor.display());
        println!("cargo:rustc-link-lib=framework=Sparkle");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        // Development binaries need the vendored framework. Release binaries must
        // resolve only the bundled copy so packaging checks cannot pass by accident.
        if std::env::var("PROFILE").as_deref() != Ok("release") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", vendor.display());
        }
    }
}
