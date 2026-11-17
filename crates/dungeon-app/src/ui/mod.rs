//! Lapisan UI egui (Fase 3).
//!
//! - `theme`: token warna/font v2 + kotak piksel + tombol.
//! - `sprites`: sprite piksel 8x8 (pengganti `PixelSprites.tsx` v2).
//! - `screens`: 9 view v2 (Title, Dungeon, Grimoire, Review, Camp,
//!   Dashboard, Inventory, Tower, Settings).
//! - `dialogs`: 5 dialog v2 (Battle, Boss, Scroll, Lesson, LevelUp).
//!
//! Tidak ada layar/menu AI Mentor di mana pun (dilarang di v3.0).

pub mod dialogs;
pub mod screens;
pub mod sprites;
pub mod theme;
