use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=src/locales.tsv");
    let catalog = std::fs::read_to_string("src/locales.tsv").expect("Missing translations");
    let mut rows = std::collections::BTreeMap::new();
    for (line, row) in catalog.lines().enumerate() {
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let cells: Vec<_> = row.split('\t').collect();
        assert_eq!(
            cells.len(),
            10,
            "Expected 10 languages at line {}",
            line + 1
        );
        assert!(
            cells.iter().all(|cell| !cell.trim().is_empty()),
            "Empty translation at line {}",
            line + 1
        );
        assert!(
            rows.insert(cells[0], cells[1..].to_vec()).is_none(),
            "Duplicate translation: {}",
            cells[0]
        );
    }
    let mut output = String::from("const MESSAGES: &[(&str, [&str; 9])] = &[\n");
    for (key, translations) in rows {
        output.push_str(&format!("({key:?}, {translations:?}),\n"));
    }
    output.push_str("];\n");
    std::fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("translations.rs"),
        output,
    )
    .unwrap();
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
