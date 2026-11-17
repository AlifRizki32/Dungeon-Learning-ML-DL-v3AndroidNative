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
pub mod ui;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

/// Entry point NativeActivity. Dipanggil sistem Android saat activity dibuat.
///
/// Hanya dikompilasi untuk target Android; di host, crate ini kosong
/// (supaya `cargo check` workspace tetap jalan tanpa Android).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    engine::run_android(app);
}
