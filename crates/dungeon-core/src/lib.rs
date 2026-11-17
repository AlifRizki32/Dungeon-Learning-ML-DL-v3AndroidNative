//! `dungeon-core` — logika murni permainan (scaffold M1).
//!
//! Lapis ini nantinya berisi rumus dan engine v2 yang diporting fungsi
//! per fungsi: formulas (XP `round(((min + max) / 2) * 50)`, kurva level
//! `60 * (L - 1)^2`), lesson engine, spellbook, camp tracker, state +
//! reducer (lihat SDD Bagian 4, 6, 10 dan spec Bagian 3).
//!
//! Aturan lapis ini: bisa diuji tanpa Android (`cargo test`), tidak pernah
//! memanggil API Android. M1: placeholder saja, belum ada logika game.

pub use dungeon_data as data;

/// Placeholder sementara agar crate bisa di-compile di M1.
/// Dihapus saat modul pertama (formulas) ditulis.
pub fn placeholder() {}
