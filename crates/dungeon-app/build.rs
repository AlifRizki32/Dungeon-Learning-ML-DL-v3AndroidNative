//! Build script: decode font base64 ke OUT_DIR sebagai `.ttf`.
//!
//! Repo menyimpan font sebagai teks base64 (`.ttf.b64`) karena konektor
//! GitHub hanya mendorong berkas UTF-8 — berkas biner tidak bisa
//! di-push lewat `github push-files`. Isi font tetap sama persis
//! (Press Start 2P + VT323, OFL 1.1, sumber: github.com/google/fonts);
//! decode terjadi saat build, dan `theme.rs` memuat `.ttf` hasil
//! decode dari `OUT_DIR` lewat `include_bytes!`, jadi font tetap
//! dibundel di dalam `.so`/APK untuk pemakaian offline.

use std::path::Path;

use base64::Engine as _;

const FONTS: [(&str, &str); 2] = [
    (
        "assets/fonts/PressStart2P-Regular.ttf.b64",
        "PressStart2P-Regular.ttf",
    ),
    ("assets/fonts/VT323-Regular.ttf.b64", "VT323-Regular.ttf"),
];

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR tidak disetel cargo");
    for (b64_path, ttf_name) in FONTS {
        let encoded = std::fs::read_to_string(b64_path)
            .unwrap_or_else(|e| panic!("build.rs: gagal membaca {b64_path}: {e}"));
        let compact: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(compact.as_bytes())
            .unwrap_or_else(|e| panic!("build.rs: gagal decode base64 {b64_path}: {e}"));
        std::fs::write(Path::new(&out_dir).join(ttf_name), decoded)
            .unwrap_or_else(|e| panic!("build.rs: gagal menulis {ttf_name}: {e}"));
        println!("cargo:rerun-if-changed={b64_path}");
    }
}
