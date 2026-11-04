use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

const IMAGES: &str = "frontend/src/lib/assets/images";
const MODEL_URL: &str = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/silueta.onnx";
const MODEL_SHA256: &str = "75da6c8d2f8096ec743d071951be73b4a8bc7b3e51d9a6625d63644f90ffeedb";

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed={IMAGES}");
    let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(IMAGES);
    let mut originals: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let kind = match path.extension()?.to_str()? {
                "png" => "image/png",
                "webp" => "image/webp",
                "gif" => "image/gif",
                _ => return None,
            };
            let slug = path.file_stem()?.to_str()?.to_string();
            Some(format!("({slug:?}, {kind:?}, include_bytes!({:?})),", path.display().to_string()))
        })
        .collect();
    originals.sort();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("originals.rs");
    std::fs::write(out, format!("pub const ORIGINALS: &[(&str, &str, &[u8])] = &[\n{}\n];\n", originals.join("\n"))).unwrap();

    let model = Path::new(&std::env::var("OUT_DIR").unwrap()).join("silueta.onnx");
    if !matches(&model) {
        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&model)
            .arg(MODEL_URL)
            .status()
            .expect("failed to run curl; is it installed?");
        assert!(status.success(), "downloading the background removal model failed");
        assert!(matches(&model), "the background removal model has the wrong checksum");
    }
}

fn matches(path: &Path) -> bool {
    fs::read(path).is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == MODEL_SHA256)
}
