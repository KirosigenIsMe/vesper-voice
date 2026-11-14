use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/vesper.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("vesper-resource.o");
        let result = Command::new("windres")
            .args(["-I", "assets", "-i", "assets/app.rc", "-o"])
            .arg(&out)
            .status()
            .expect("windres is required for the Windows icon");
        assert!(result.success(), "Unable to build icon resource");
        println!("cargo:rustc-link-arg={}", out.display());
    }
}
