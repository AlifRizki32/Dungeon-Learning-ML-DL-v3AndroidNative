//! Token tema v2 (dari `src/styles/pixel.css`, tidak diubah) + font.
//!
//! - Warna: latar abyss/dungeon/panel, border terang/gelap, teks krem/redup,
//!   aksen hijau/emas/merah/cyan/ungu (+ varian gelap).
//! - Font: Press Start 2P (judul piksel) + VT323 (teks RPG), dibundel lokal
//!   di `assets/fonts/` (OFL, sumber: github.com/google/fonts).
//!   egui 0.37 tidak menyertakan font bawaan (`default_fonts` opt-in),
//!   jadi kedua font ini WAJIB ada; bila aset rusak, `install_theme`
//!   jatuh ke font egui kosong dan UI tetap jalan dengan font sistem
//!   (dicatat di log via `eprintln!` — tidak ada logger di v3.0).
//! - Satuan: egui memakai point; di Android `pixels_per_point` =
//!   faktor densitas layar, jadi 1 point ≈ 1 dp. Target sentuh minimum
//!   48 dp = 48 point (lihat `TOUCH_MIN`).

use egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke, Ui,
};

/// Target sentuh minimum (spec Bagian 4): 48 dp.
pub const TOUCH_MIN: f32 = 48.0;
/// Lebar konten maksimum layar potret (360–430 dp).
pub const CONTENT_MAX_WIDTH: f32 = 430.0;
/// Inset aman atas (status bar) dan bawah (gesture bar). winit 0.30 tidak
/// menyediakan safe area Android, jadi dipakai perkiraan konservatif;
/// dicatat sebagai keterbatasan di `ui.md`.
pub const SAFE_TOP: f32 = 28.0;
pub const SAFE_BOTTOM: f32 = 20.0;

pub mod color {
    use egui::Color32;

    /// `#0d0a1a`
    pub const ABYSS: Color32 = Color32::from_rgb(0x0d, 0x0a, 0x1a);
    /// `#16112b`
    pub const DUNGEON: Color32 = Color32::from_rgb(0x16, 0x11, 0x2b);
    /// `#1f193d`
    pub const PANEL: Color32 = Color32::from_rgb(0x1f, 0x19, 0x3d);
    /// `#2b2352`
    pub const PANEL_INNER: Color32 = Color32::from_rgb(0x2b, 0x23, 0x52);
    /// `#120d26` (bayangan dalam pixel-box)
    pub const PANEL_DARK: Color32 = Color32::from_rgb(0x12, 0x0d, 0x26);
    /// `#3a306e` (sorot dalam pixel-box)
    pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0x3a, 0x30, 0x6e);
    /// `#5c4e99`
    pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0x5c, 0x4e, 0x99);
    /// `#080612`
    pub const BORDER_DARK: Color32 = Color32::from_rgb(0x08, 0x06, 0x12);
    /// `#f4f1ea`
    pub const CREAM: Color32 = Color32::from_rgb(0xf4, 0xf1, 0xea);
    /// `#b3aad1`
    pub const MUTED: Color32 = Color32::from_rgb(0xb3, 0xaa, 0xd1);
    /// `#38d864`
    pub const GREEN: Color32 = Color32::from_rgb(0x38, 0xd8, 0x64);
    /// `#1d8a3d`
    pub const GREEN_DARK: Color32 = Color32::from_rgb(0x1d, 0x8a, 0x3d);
    /// `#f5b935`
    pub const GOLD: Color32 = Color32::from_rgb(0xf5, 0xb9, 0x35);
    /// `#a87312`
    pub const GOLD_DARK: Color32 = Color32::from_rgb(0xa8, 0x73, 0x12);
    /// `#e83b47`
    pub const RED: Color32 = Color32::from_rgb(0xe8, 0x3b, 0x47);
    /// `#941923`
    pub const RED_DARK: Color32 = Color32::from_rgb(0x94, 0x19, 0x23);
    /// `#3ac5eb`
    pub const CYAN: Color32 = Color32::from_rgb(0x3a, 0xc5, 0xeb);
    /// `#9b5de5`
    pub const PURPLE: Color32 = Color32::from_rgb(0x9b, 0x5d, 0xe5);
    /// `#fb923c` (aksen oranye Lantai 5, dari DungeonMap v2)
    pub const ORANGE: Color32 = Color32::from_rgb(0xfb, 0x92, 0x3c);
    /// Varian latar pixel-box gold `#241b2f`
    pub const GOLD_BOX_BG: Color32 = Color32::from_rgb(0x24, 0x1b, 0x2f);
    /// Varian latar pixel-box red `#291324`
    pub const RED_BOX_BG: Color32 = Color32::from_rgb(0x29, 0x13, 0x24);
    /// Varian latar pixel-box green `#132924`
    pub const GREEN_BOX_BG: Color32 = Color32::from_rgb(0x13, 0x29, 0x24);
}

/// Nama keluarga font kustom (dipakai sebagai `FontFamily::Name`).
pub const FONT_PIXEL: &str = "PressStart2P";
pub const FONT_VT: &str = "VT323";

/// Pasang font + gaya dasar egui sesuai tema v2. Dipanggil sekali
/// saat engine dibuat (sebelum frame pertama).
pub fn install_theme(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    // Font dibundel sebagai aset APK (`assets/fonts/`); di host (cargo
    // check) file yang sama dibaca dari direktori crate.
    // Font disimpan sebagai base64 di repo (lihat build.rs) dan didekode
    // ke OUT_DIR saat build; `include_bytes!` memuat hasil decode-nya
    // sehingga font tetap berada di dalam `.so`/APK (offline).
    let pixel = include_bytes!(concat!(env!("OUT_DIR"), "/PressStart2P-Regular.ttf"));
    let vt = include_bytes!(concat!(env!("OUT_DIR"), "/VT323-Regular.ttf"));
    fonts.font_data.insert(
        FONT_PIXEL.to_owned(),
        FontData::from_static(pixel).into(),
    );
    fonts.font_data.insert(FONT_VT.to_owned(), FontData::from_static(vt).into());
    // Judul piksel + teks RPG sebagai font utama; tanpa font bawaan egui
    // (default_fonts tidak diaktifkan) supaya APK tetap kecil dan offline.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_VT.to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, FONT_PIXEL.to_owned());

    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(color::CREAM),
        window_fill: color::ABYSS,
        panel_fill: color::PANEL,
        faint_bg_color: color::PANEL_INNER,
        extreme_bg_color: color::ABYSS,
        code_bg_color: color::PANEL_DARK,
        warn_fg_color: color::GOLD,
        error_fg_color: color::RED,
        window_stroke: Stroke::new(3.0, color::BORDER_LIGHT),
        window_corner_radius: egui::CornerRadius::ZERO,
        window_shadow: egui::Shadow::NONE,
        popup_shadow: egui::Shadow::NONE,
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: color::PANEL_INNER,
                weak_bg_fill: color::PANEL_INNER,
                bg_stroke: Stroke::new(2.0, color::BORDER_LIGHT),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: Stroke::new(1.0, color::CREAM),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: color::PANEL_INNER,
                weak_bg_fill: color::PANEL_INNER,
                bg_stroke: Stroke::new(2.0, color::BORDER_LIGHT),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: Stroke::new(1.0, color::CREAM),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: color::PANEL_LIGHT,
                weak_bg_fill: color::PANEL_LIGHT,
                bg_stroke: Stroke::new(2.0, color::GOLD),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: Stroke::new(1.0, color::CREAM),
                expansion: 0.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: color::GREEN_DARK,
                weak_bg_fill: color::GREEN_DARK,
                bg_stroke: Stroke::new(2.0, color::GREEN),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: Stroke::new(1.0, color::CREAM),
                expansion: 0.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: color::PANEL_LIGHT,
                weak_bg_fill: color::PANEL_LIGHT,
                bg_stroke: Stroke::new(2.0, color::GOLD),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: Stroke::new(1.0, color::CREAM),
                expansion: 0.0,
            },
        },
        selection: egui::style::Selection {
            bg_fill: color::GREEN,
            stroke: Stroke::new(1.0, color::ABYSS),
        },
        hyperlink_color: color::CYAN,
        ..Default::default()
    };
    // Teks: judul piksel kecil, isi VT323 besar (v2: font-vt 20-24px).
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(20.0, FontFamily::Name(FONT_PIXEL.into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(22.0, FontFamily::Name(FONT_VT.into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(20.0, FontFamily::Name(FONT_VT.into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(18.0, FontFamily::Name(FONT_VT.into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(14.0, FontFamily::Name(FONT_PIXEL.into())),
    );
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    ctx.set_style_of(egui::Theme::Dark, style);
}

/// Teks judul piksel (Press Start 2P), ukuran dalam point.
pub fn pixel_text(text: &str, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, FontFamily::Name(FONT_PIXEL.into())))
        .color(color)
}

/// Teks RPG (VT323), ukuran dalam point.
pub fn vt_text(text: &str, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, FontFamily::Name(FONT_VT.into())))
        .color(color)
}

/// Varian kotak piksel (`.pixel-box` + varian gold/red/green di pixel.css).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelBox {
    /// `.pixel-box`: panel `#1f193d`, border `#5c4e99`.
    Normal,
    /// `.pixel-box-gold`: bg `#241b2f`, border emas.
    Gold,
    /// `.pixel-box-red`: bg `#291324`, border merah.
    Red,
    /// `.pixel-box-green`: bg `#132924`, border hijau.
    Green,
}

impl PixelBox {
    fn bg(self) -> Color32 {
        match self {
            PixelBox::Normal => color::PANEL,
            PixelBox::Gold => color::GOLD_BOX_BG,
            PixelBox::Red => color::RED_BOX_BG,
            PixelBox::Green => color::GREEN_BOX_BG,
        }
    }

    fn border(self) -> Color32 {
        match self {
            PixelBox::Normal => color::BORDER_LIGHT,
            PixelBox::Gold => color::GOLD,
            PixelBox::Red => color::RED,
            PixelBox::Green => color::GREEN,
        }
    }
}

/// Gambar kotak piksel (border 3px + bayangan berundak khas v2) di
/// sekeliling `content`. Dipakai untuk semua panel/kartu.
/// Mengembalikan respons area konten.
pub fn pixel_box<R>(ui: &mut Ui, variant: PixelBox, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let outer_stroke = Stroke::new(3.0, variant.border());
    let shadow = 4i8;
    egui::Frame::default()
        .fill(variant.bg())
        .stroke(outer_stroke)
        .inner_margin(egui::Margin::same(10))
        .outer_margin(egui::Margin {
            left: shadow,
            right: shadow,
            top: shadow,
            bottom: shadow,
        })
        .show(ui, |ui| {
            // Sorot dalam (inset 3px #3a306e) + bayangan gelap bawah:
            // digambar sebagai frame dalam tipis di atas latar.
            egui::Frame::default()
                .fill(variant.bg())
                .stroke(Stroke::new(1.0, color::PANEL_LIGHT))
                .inner_margin(egui::Margin::same(2))
                .show(ui, add_contents)
                .inner
        })
        .inner
}

/// Tombol piksel (`.pixel-btn`): border 2px krem + bayangan 4px gelap,
/// tinggi minimum 48 dp (target sentuh). `bg` = warna latar tombol,
/// `fg` = warna teks.
pub fn pixel_button(ui: &mut Ui, label: &str, bg: Color32, fg: Color32) -> egui::Response {
    let min_h = TOUCH_MIN;
    let text = vt_text(label, 22.0, fg);
    let button = egui::Button::new(text)
        .fill(bg)
        .stroke(Stroke::new(2.0, color::CREAM))
        .min_size(egui::vec2(0.0, min_h));
    let resp = ui.add(button);
    // Bayangan 4px khas pixel-btn: garis gelap di bawah tombol.
    let rect = resp.rect;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, rect.bottom()),
            egui::pos2(rect.right() + 2.0, rect.bottom() + 4.0),
        ),
        0.0,
        color::BORDER_DARK,
    );
    resp
}

/// Tombol piksel kecil (mis. chip navigasi): tetap ≥48 dp tingginya.
pub fn pixel_chip(ui: &mut Ui, label: &str, active: bool) -> egui::Response {
    let (bg, fg) = if active {
        (color::GREEN, color::ABYSS)
    } else {
        (color::DUNGEON, color::CREAM)
    };
    let text = vt_text(label, 20.0, fg);
    ui.add(
        egui::Button::new(text)
            .fill(bg)
            .stroke(Stroke::new(2.0, if active { color::CREAM } else { color::BORDER_LIGHT }))
            .min_size(egui::vec2(0.0, TOUCH_MIN)),
    )
}

/// Bilah progres gaya XP v2: latar gelap + isi hijau bergaris.
pub fn xp_bar(ui: &mut Ui, fraction: f32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, color::ABYSS);
    painter.rect_stroke(rect, 0.0, Stroke::new(2.0, color::BORDER_LIGHT), egui::StrokeKind::Outside);
    let fill_w = (rect.width() * fraction.clamp(0.0, 1.0)).max(0.0);
    if fill_w > 0.0 {
        let fill = egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 2.0), egui::vec2(fill_w - 4.0, rect.height() - 4.0));
        painter.rect_filled(fill, 0.0, color::GREEN);
        // Garis terang shimmer (statis; animasi mengikuti reduce-motion di v2).
        painter.rect_filled(
            egui::Rect::from_min_size(fill.min, egui::vec2((fill_w * 0.25).min(24.0), fill.height())),
            0.0,
            Color32::from_rgb(0x6e, 0xf2, 0x91),
        );
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(13.0, FontFamily::Name(FONT_PIXEL.into())),
        color::CREAM,
    );
}
