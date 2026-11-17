//! `dungeon-app` — lapis aplikasi Android (Fase 3: UI egui di atas wgpu).
//!
//! Satu-satunya crate yang menyentuh Android. Entry point memakai
//! `android-activity` (NativeActivity) + winit 0.30 + egui 0.37 / wgpu 30.
//! State game berasal dari `dungeon-core` (reducer); UI di `ui/` hanya
//! mengirim `Action` dan menampilkan state + `Effect`.
//!
//! M3: 9 layar v2 + 5 dialog dipetakan ke Android potret; tanpa audio
//! (fase terpisah), tanpa penyimpanan Android (fase terpisah), tanpa
//! WebView 3D (fase terpisah), tanpa AI Mentor.

pub mod app;
pub mod engine;
pub mod logging;
pub mod ui;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

/// Entry point NativeActivity. Dipanggil sistem Android saat activity dibuat.
///
/// Hanya dikompilasi untuk target Android; di host, crate ini kosong
/// (supaya `cargo check` workspace tetap jalan tanpa Android).
///
/// M3-fix: logging + panic hook dipasang PALING AWAL (sebelum event
/// loop), dan seluruh `run_android` dibungkus `catch_unwind`. Tanpa
/// ini, panic di startup meng-unwind melewati batas FFI `android_main`
/// → proses di-abort → force close instan tanpa jejak di logcat.
/// Dengan ini panic tercatat ke logcat (tag `dungeon`) dan aplikasi
/// menutup bersih alih-alih abort.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    logging::init();
    log::info!("android_main mulai (dungeonlearning.app)");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine::run_android(app);
    }));
    match result {
        Ok(()) => log::info!("android_main selesai normal"),
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic tanpa pesan".to_string());
            log::error!("android_main menangkap panic (bukan abort): {msg}");
        }
    }
}
