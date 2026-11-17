//! Logika inti game Dungeon Learning v3 — murni, tanpa dependensi Android.
//!
//! Port dari kode v2 (`src/game/` dan `src/store/useGameStore.ts`).
//! Inti game tidak pernah membaca jam sistem: "hari ini" selalu dipasok
//! pemanggil sebagai kunci tanggal lokal `YYYY-MM-DD`.

pub mod camp;
pub mod dates;
pub mod formulas;
pub mod lesson_engine;
pub mod reducer;
pub mod rng;
pub mod spellbook;
pub mod state;

pub use dungeon_data as data;
