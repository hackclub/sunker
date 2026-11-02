use std::path::Path;

const IMAGES: &str = "frontend/src/lib/assets/images";

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
}
