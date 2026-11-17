//! Logging ke logcat Android + panic hook (M3-fix).
//!
//! Masalah yang diperbaiki: sebelum M3-fix tidak ada logger dan tidak
//! ada panic hook. Panic di jalur startup (mis. `expect()` di
//! `engine.rs`) langsung meng-unwind melewati batas FFI `android_main`
//! → proses di-abort → force close instan TANPA jejak apa pun di
//! logcat. Dengan modul ini:
//! - setiap panic dicatat ke logcat (tag `dungeon`) SEBELUM unwind,
//! - milestone startup dicatat (`android_main`, event loop, window,
//!   instance, surface, adapter, device, frame pertama),
//! - `android_main` dibungkus `catch_unwind` di `lib.rs` supaya panic
//!   tidak meng-abort proses (aplikasi menutup bersih, log tetap ada).
//!
//! Di host (bukan Android) modul ini no-op — `cargo check`/`cargo test`
//! tetap jalan tanpa logcat.

/// Pasang logger logcat + panic hook. Dipanggil PALING AWAL di
/// `android_main`, sebelum apa pun (termasuk pembuatan event loop).
pub fn init() {
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("dungeon")
                .with_max_level(log::LevelFilter::Debug),
        );
        // Panic hook: catat panic ke logcat, lalu panggil hook default
        // (yang menulis ke stderr — di Android tidak terlihat, tapi
        // tidak ada salahnya).
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // `PanicHookInfo` Display sudah memuat pesan + lokasi.
            log::error!("PANIC: {info}");
            default_hook(info);
        }));
        log::info!("logging aktif (tag logcat: dungeon)");
    }
    #[cfg(not(target_os = "android"))]
    {
        // Host: tidak ada logcat; sengaja diam.
    }
}
