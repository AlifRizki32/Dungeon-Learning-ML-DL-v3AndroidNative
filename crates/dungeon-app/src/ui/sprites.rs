//! Sprite piksel (pengganti `PixelSprites.tsx` v2).
//!
//! v2 menggambar sprite sebagai SVG piksel (1969 baris). Di egui,
//! sprite digambar sebagai bitmap 8x8 dari kotak-kotak berwarna —
//! gaya yang sama (pixel-art tajam, tanpa anti-alias), jauh lebih
//! ringan. Setiap sprite adalah 8 baris × 8 kolom karakter:
//! `.` transparan, `#` warna utama, `d` gelap, `l` terang,
//! `e` mata gelap, `g` emas, `r` merah, `w` krem.

use egui::{Color32, Painter, Rect, Vec2};

use crate::ui::theme::color;

/// Satu sprite 8x8.
pub struct Sprite {
    pub rows: [&'static str; 8],
}

fn draw_sprite(painter: &Painter, rect: Rect, sprite: &Sprite, main: Color32) {
    let cell = rect.width() / 8.0;
    let dark = main.gamma_multiply(0.55);
    let light = main.gamma_multiply(1.35);
    for (y, row) in sprite.rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let c = match ch {
                '#' => Some(main),
                'd' => Some(dark),
                'l' => Some(light),
                'e' => Some(color::BORDER_DARK),
                'g' => Some(color::GOLD),
                'r' => Some(color::RED),
                'w' => Some(color::CREAM),
                _ => None,
            };
            if let Some(c) = c {
                let min = rect.min + Vec2::new(x as f32 * cell, y as f32 * cell);
                painter.rect_filled(Rect::from_min_size(min, Vec2::splat(cell)), 0.0, c);
            }
        }
    }
}

/// Gambar sprite di `rect` (persegi). `main` = warna utama sprite.
pub fn draw(painter: &Painter, rect: Rect, sprite: &Sprite, main: Color32) {
    let size = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), Vec2::splat(size));
    draw_sprite(painter, square, sprite, main);
}

/// Alokasikan area persegi `size` di `ui` lalu gambar sprite di sana.
pub fn show(ui: &mut egui::Ui, sprite: &Sprite, main: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    draw(ui.painter(), rect, sprite, main);
}

/// Wajah avatar 8x8 (topi/helm berbeda per kelas lewat warna utama).
pub const AVATAR: Sprite = Sprite {
    rows: [
        ".######.",
        "#llllll#",
        "#l####l#",
        "#le##el#",
        "#l####l#",
        ".#d##d#.",
        "..####..",
        ".#d##d#.",
    ],
};

/// Slime Angka (monster Lantai 1).
pub const SLIME: Sprite = Sprite {
    rows: [
        "...##...",
        "..####..",
        ".######.",
        ".#e##e#.",
        "########",
        "#d####d#",
        "########",
        ".##..##.",
    ],
};

/// Golem Pohon (monster Lantai 2).
pub const GOLEM: Sprite = Sprite {
    rows: [
        ".######.",
        ".#l##l#.",
        ".######.",
        "..#ee#..",
        ".######.",
        "#d#dd#d#",
        ".######.",
        ".##..##.",
    ],
};

/// Hydra Neural (monster Lantai 3): tiga kepala.
pub const HYDRA: Sprite = Sprite {
    rows: [
        "#.#..#.#",
        "###..###",
        ".#e##e#.",
        "..####..",
        ".######.",
        "#d#dd#d#",
        ".######.",
        "..#..#..",
    ],
};

/// Mecha-Bot IoT (monster Lantai 4).
pub const MECHABOT: Sprite = Sprite {
    rows: [
        ".#l##l#.",
        ".######.",
        ".#e##e#.",
        ".#g##g#.",
        ".######.",
        "..#dd#..",
        ".#l##l#.",
        "...##...",
    ],
};

/// Raja Deadline (monster Lantai 5): jam + mahkota.
pub const DEADLINE: Sprite = Sprite {
    rows: [
        ".g.g.g..",
        ".######.",
        ".#e##e#.",
        ".######.",
        ".##rr##.",
        "..####..",
        ".#d##d#.",
        "..#..#..",
    ],
};

/// Pintu dungeon.
pub const DOOR: Sprite = Sprite {
    rows: [
        "..####..",
        ".#l##l#.",
        ".#l##l#.",
        ".#e##e#.",
        ".#l##l#.",
        ".#l##l#.",
        ".#l##l#.",
        ".######.",
    ],
};

/// Pedang piksel.
pub const SWORD: Sprite = Sprite {
    rows: [
        "......#w",
        ".....#w#",
        "....#w#.",
        "...#w#..",
        "..#w#...",
        ".#d#....",
        "..##....",
        ".#d#....",
    ],
};

/// Gembok.
pub const LOCK: Sprite = Sprite {
    rows: [
        "...##...",
        "..#..#..",
        "..#..#..",
        ".######.",
        ".#e##e#.",
        ".#l##l#.",
        ".######.",
        "........",
    ],
};

/// Hati (diisi `filled=true`, kosong = garis luar saja).
pub const HEART: Sprite = Sprite {
    rows: [
        ".##..##.",
        "########",
        "########",
        ".######.",
        "..####..",
        "...##...",
        "........",
        "........",
    ],
};

/// Bintang (badge/relic).
pub const STAR: Sprite = Sprite {
    rows: [
        "...##...",
        "...##...",
        "########",
        ".######.",
        "..####..",
        ".#g##g#.",
        "........",
        "........",
    ],
};

/// Ramuan (potion).
pub const POTION: Sprite = Sprite {
    rows: [
        "...##...",
        "...##...",
        "..####..",
        ".#rrrr#.",
        ".#rrrr#.",
        ".#rrrr#.",
        "..####..",
        "........",
    ],
};

/// Relik (permata).
pub const RELIC: Sprite = Sprite {
    rows: [
        "...##...",
        "..#gg#..",
        ".#gggg#.",
        "#gggggg#",
        ".#gggg#.",
        "..#gg#..",
        "...##...",
        "........",
    ],
};

/// Gulungan (scroll).
pub const SCROLL: Sprite = Sprite {
    rows: [
        ".######.",
        ".#wwww#.",
        ".#w##w#.",
        ".#wwww#.",
        ".#w##w#.",
        ".#wwww#.",
        ".######.",
        "........",
    ],
};

/// Buku mantra (grimoire).
pub const BOOK: Sprite = Sprite {
    rows: [
        ".######.",
        ".#g##g#.",
        ".#g##g#.",
        ".######.",
        ".#w##w#.",
        ".#w##w#.",
        ".######.",
        "........",
    ],
};

/// Sprite monster berdasarkan `monster_type` roadmap
/// (`slime`/`golem`/`hydra`/`mechabot`/`deadline`, atau `boss-*`).
pub fn monster_sprite(monster_type: &str) -> &'static Sprite {
    match monster_type {
        "slime" | "boss-slime" => &SLIME,
        "golem" | "boss-golem" => &GOLEM,
        "hydra" | "boss-hydra" => &HYDRA,
        "mechabot" | "boss-mechabot" => &MECHABOT,
        "deadline" | "boss-deadline" => &DEADLINE,
        _ => &SLIME,
    }
}

/// Warna utama avatar berdasarkan id (`alchemist`/`knight`/`ranger`/`tinker`).
/// Diambil dari `AVATARS` roadmap; fallback ungu bila id tidak dikenal.
pub fn avatar_color(avatar_id: &str) -> Color32 {
    match avatar_id {
        "alchemist" => color::PURPLE,
        "knight" => color::GREEN,
        "ranger" => color::CYAN,
        "tinker" => color::GOLD,
        _ => color::PURPLE,
    }
}

/// Parse warna hex `#rrggbb` dari roadmap menjadi `Color32`.
/// Bila gagal, kembalikan krem.
pub fn parse_hex(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    color::CREAM
}

/// Gambar avatar (wajah) di `ui` dengan ukuran `size`.
pub fn show_avatar(ui: &mut egui::Ui, avatar_id: &str, size: f32) {
    show(ui, &AVATAR, avatar_color(avatar_id), size);
}

/// Gambar avatar di painter dalam `rect`.
pub fn draw_avatar(painter: &Painter, rect: Rect, avatar_id: &str) {
    draw(painter, rect, &AVATAR, avatar_color(avatar_id));
}

/// Gambar hati (terisi/kosong) di `ui`.
pub fn show_heart(ui: &mut egui::Ui, filled: bool, size: f32) {
    let c = if filled { color::RED } else { color::BORDER_LIGHT };
    show(ui, &HEART, c, size);
}

/// Posisi pemain di peta (panah/penanda): kotak hijau + teks "POSISI"
/// digambar pemanggil; sprite ini panah kecil.
pub const MARKER: Sprite = Sprite {
    rows: [
        "...##...",
        "..####..",
        ".######.",
        "########",
        "...##...",
        "...##...",
        "...##...",
        "........",
    ],
};
