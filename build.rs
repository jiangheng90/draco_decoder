use std::path::Path;
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "wasm32" {
        println!("cargo:warning=Skipping build.rs on wasm32 target");
        return;
    }

    // js-build
    {
        let js_dir = "third_party/draco_decoder_js";
        let dist_dir = format!("{}/dist", js_dir);

        println!("cargo:warning=Building draco_decoder_js...");

        let status = Command::new("npm")
            .arg("i")
            .current_dir(js_dir)
            .status()
            .expect("Failed to run npm i for draco_decoder_js");

        assert!(status.success(), "npm i failed");

        let status = Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(js_dir)
            .status()
            .expect("Failed to run npm build for draco_decoder_js");
        assert!(status.success(), "npm build failed");

        let dst_js = Path::new("javascript/index.es.js");
        std::fs::create_dir_all(dst_js.parent().unwrap()).unwrap();
        std::fs::copy(format!("{}/index.es.js", dist_dir), &dst_js)
            .expect("Failed to copy index.es.js");

        let dst_wasm = Path::new("javascript/draco3d/draco_decoder.wasm");
        std::fs::create_dir_all(dst_wasm.parent().unwrap()).unwrap();
        std::fs::copy(
            format!("{}/draco3d/draco_decoder.wasm", dist_dir),
            &dst_wasm,
        )
        .expect("Failed to copy draco_decoder.wasm");
    }

    // Step 1: Build Draco with CMake
    let draco_dir = "third_party/draco";
    let draco_build = format!("{draco_dir}/build");
    let draco_install = format!("{draco_build}/install");

    if !Path::new(&draco_build).exists() {
        std::fs::create_dir_all(&draco_build).unwrap();
    }

    let status = Command::new("cmake")
        .args([
            "..",
            "-G",
            "Unix Makefiles",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DDRACO_TESTS=OFF",
            &format!("-DCMAKE_INSTALL_PREFIX={}", "install"),
        ])
        .current_dir(&draco_build)
        .status()
        .expect("Failed to run CMake");
    assert!(status.success(), "CMake configuration failed");

    let status = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&draco_build)
        .status()
        .expect("Failed to build Draco");
    assert!(status.success(), "Draco build failed");

    let status = Command::new("cmake")
        .args(["--install", "."])
        .current_dir(&draco_build)
        .status()
        .expect("Failed to install Draco");
    assert!(status.success(), "Draco install failed");

    cxx_build::bridge("src/ffi.rs")
        .file("cpp/decoder_api.cc")
        .include("include")
        .include("third_party/draco/src")
        .include("third_party/draco/build")
        .include(format!("{draco_install}/include"))
        .flag_if_supported("-std=c++17")
        .flag("-mmacosx-version-min=15.5")
        .compile("decoder_api");

    println!("cargo:rustc-link-search=native={draco_install}/lib");
    println!("cargo:rustc-link-lib=static=draco");

    println!("cargo:rerun-if-changed=cpp/decoder_api.cc");
    println!("cargo:rerun-if-changed=include/decoder_api.h");
}
