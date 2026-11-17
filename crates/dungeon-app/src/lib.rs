//! `dungeon-app` — lapis aplikasi Android (scaffold M1).
//!
//! Satu-satunya crate yang menyentuh Android. Entry point memakai
//! `android-activity` (NativeActivity, jalur resmi komunitas Rust).
//!
//! M1: placeholder — loop event minimal, belum ada UI, belum ada logika
//! game, belum ada audio, belum ada penyimpanan, belum ada WebView.
//! Toolkit UI (egui di atas wgpu, usulan SDD Bagian 5) BELUM diputuskan
//! Jamal, jadi TIDAK ada dependensi UI di sini. Jangan tambah sebelum
//! ada keputusan.

#[cfg(target_os = "android")]
use android_activity::{AndroidApp, MainEvent, PollEvent};

/// Entry point NativeActivity. Dipanggil sistem Android saat activity dibuat.
///
/// Hanya dikompilasi untuk target Android; di host, crate ini kosong
/// (supaya `cargo check` workspace tetap jalan tanpa Android).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    // M1: loop event minimal — tidak melakukan apa-apa selain menunggu
    // activity dihancurkan. UI dan game loop ditulis di fase berikutnya.
    // (android-activity 0.6: poll_events memakai callback, bukan return value.)
    let mut destroyed = false;
    while !destroyed {
        app.poll_events(Some(std::time::Duration::from_millis(100)), |event| {
            if matches!(event, PollEvent::Main(MainEvent::Destroy)) {
                destroyed = true;
            }
        });
    }
}
