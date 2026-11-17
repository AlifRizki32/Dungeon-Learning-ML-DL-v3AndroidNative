//! Penampil scroll materi (`ScrollView.tsx` v2, 1023 baris).
//!
//! Rumus mantra, paragraf scroll, contoh pengerjaan, kesalahan umum,
//! referensi, dan 4 mini-lab interaktif v2: neuron, softmax, quant,
//! latency. Mini-lab bebas (tanpa XP, tidak menghalangi progres).

use dungeon_core::spellbook::get_topic_content;
use egui::{Context, Ui};

use crate::{
    app::App,
    ui::theme::{self, color, PixelBox},
};

/// Nilai slider mini-lab (bertahan selama aplikasi jalan).
pub struct ScrollUi {
    // Neuron: z = w1·x1 + w2·x2 + b.
    pub x1: f64,
    pub x2: f64,
    pub w1: f64,
    pub w2: f64,
    pub b: f64,
    // Softmax: 3 skor.
    pub s1: f64,
    pub s2: f64,
    pub s3: f64,
    // Quant: jumlah parameter (juta), dtype, arena aktivasi, kapasitas.
    pub quant_params_m: f64,
    pub quant_int8: bool,
    pub quant_arena_kb: f64,
    pub quant_flash_kb: f64,
    pub quant_ram_kb: f64,
    // Latency: 4 tahap pipa (ms) + batas total.
    pub lat_akuisisi: f64,
    pub lat_pra: f64,
    pub lat_inferensi: f64,
    pub lat_kirim: f64,
    pub lat_limit: f64,
}

impl Default for ScrollUi {
    fn default() -> Self {
        Self {
            x1: 1.0,
            x2: 0.5,
            w1: 0.8,
            w2: -0.6,
            b: 0.1,
            s1: 1.0,
            s2: 2.0,
            s3: 0.5,
            quant_params_m: 1.0,
            quant_int8: false,
            quant_arena_kb: 50.0,
            quant_flash_kb: 1024.0,
            quant_ram_kb: 512.0,
            lat_akuisisi: 20.0,
            lat_pra: 15.0,
            lat_inferensi: 40.0,
            lat_kirim: 25.0,
            lat_limit: 100.0,
        }
    }
}

/// Gambar dialog scroll. Mengembalikan `true` bila dialog masih terbuka.
pub fn show_scroll(app: &mut App, ctx: &Context, topic_id: &str) -> bool {
    let mut open = true;
    let topic_name = dungeon_data::roadmap::topic_by_id(topic_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| topic_id.to_string());
    let title = format!("📜 {topic_name}");
    let mut close_after = false;

    egui::Window::new(theme::pixel_text(&title, 13.0, color::CREAM))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ctx.viewport_rect().height() * 0.75)
                .show(ui, |ui| {
                    show_scroll_body(app, ui, topic_id);
                });
            ui.add_space(8.0);
            if theme::pixel_button(ui, "Tutup", color::GREEN_DARK, color::CREAM).clicked() {
                close_after = true;
            }
        });

    if close_after {
        open = false;
    }
    open
}

fn show_scroll_body(app: &mut App, ui: &mut Ui, topic_id: &str) {
    let Some(tc) = get_topic_content(topic_id) else {
        ui.label(theme::vt_text("Konten topik tidak ditemukan.", 21.0, color::RED));
        return;
    };
    ui.label(theme::pixel_text(
        &format!("MANTRA: {}", tc.topic.spell.name),
        11.0,
        color::GOLD,
    ));
    for formula in &tc.topic.spell.formulas {
        ui.label(theme::vt_text(&format!("• {formula}"), 20.0, color::CREAM));
    }
    ui.add_space(6.0);
    for para in &tc.topic.scroll.paragraphs {
        ui.label(theme::vt_text(para, 21.0, color::CREAM));
        ui.add_space(4.0);
    }
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("CONTOH PENGERJAAN", 10.0, color::CYAN));
        ui.label(theme::vt_text(&tc.topic.scroll.worked_example, 20.0, color::CREAM));
    });
    ui.add_space(4.0);
    theme::pixel_box(ui, PixelBox::Red, |ui| {
        ui.label(theme::pixel_text("KESALAHAN UMUM", 10.0, color::RED));
        ui.label(theme::vt_text(&tc.topic.scroll.common_mistake, 20.0, color::CREAM));
    });
    ui.add_space(4.0);
    if !tc.topic.references.is_empty() {
        ui.label(theme::pixel_text("REFERENSI", 10.0, color::GOLD));
        for r in &tc.topic.references {
            let text = match &r.url {
                Some(url) => format!("• {} ({url})", r.label),
                None => format!("• {}", r.label),
            };
            ui.label(theme::vt_text(&text, 18.0, color::CYAN));
        }
        ui.add_space(4.0);
    }
    // Mini-lab (bila ada di topik ini).
    match tc.topic.mini_lab.as_deref() {
        Some("neuron") => show_lab_neuron(app, ui),
        Some("softmax") => show_lab_softmax(app, ui),
        Some("quant") => show_lab_quant(app, ui),
        Some("latency") => show_lab_latency(app, ui),
        _ => {}
    }
}

fn lab_header(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(theme::pixel_text(title, 11.0, color::GOLD));
    ui.label(theme::vt_text(
        "Eksperimen bebas (Tanpa XP · Tidak menghalangi progres)",
        18.0,
        color::MUTED,
    ));
}

fn show_lab_neuron(app: &mut App, ui: &mut Ui) {
    lab_header(ui, "MINI-LAB: NEURON");
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        for (label, val, range) in [
            ("Input x1", &mut app.scroll.x1, -2.0..=2.0),
            ("Input x2", &mut app.scroll.x2, -2.0..=2.0),
            ("Bobot w1", &mut app.scroll.w1, -2.0..=2.0),
            ("Bobot w2", &mut app.scroll.w2, -2.0..=2.0),
            ("Bias b", &mut app.scroll.b, -2.0..=2.0),
        ] {
            ui.add(egui::Slider::new(val, range).text(label).step_by(0.1));
        }
        let z = app.scroll.w1 * app.scroll.x1 + app.scroll.w2 * app.scroll.x2 + app.scroll.b;
        let relu = z.max(0.0);
        let sigmoid = 1.0 / (1.0 + (-z).exp());
        ui.label(theme::vt_text(&format!("z = w1·x1 + w2·x2 + b = {z:.2}"), 21.0, color::GOLD));
        ui.label(theme::vt_text(&format!("ReLU(z) = {relu:.2}"), 20.0, color::GREEN));
        ui.label(theme::vt_text(&format!("sigmoid(z) = {sigmoid:.3}"), 20.0, color::CYAN));
    });
}

fn show_lab_softmax(app: &mut App, ui: &mut Ui) {
    lab_header(ui, "MINI-LAB: SOFTMAX");
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        for (label, val) in [
            ("Skor s1", &mut app.scroll.s1),
            ("Skor s2", &mut app.scroll.s2),
            ("Skor s3", &mut app.scroll.s3),
        ] {
            ui.add(egui::Slider::new(val, -5.0..=5.0).text(label).step_by(0.5));
        }
        let (s1, s2, s3) = (app.scroll.s1, app.scroll.s2, app.scroll.s3);
        let m = s1.max(s2).max(s3);
        let (e1, e2, e3) = ((s1 - m).exp(), (s2 - m).exp(), (s3 - m).exp());
        let sum = e1 + e2 + e3;
        ui.label(theme::vt_text(
            "softmax(s_i) = e^s_i / Σ e^s_j",
            20.0,
            color::GOLD,
        ));
        for (i, p) in [e1 / sum, e2 / sum, e3 / sum].iter().enumerate() {
            ui.label(theme::vt_text(
                &format!("p{} = {:.3} ({:.1}%)", i + 1, p, p * 100.0),
                20.0,
                color::CREAM,
            ));
            theme::xp_bar(ui, *p as f32, "");
        }
        ui.label(theme::vt_text("Jumlah bobot = 1", 19.0, color::GREEN));
    });
}

fn show_lab_quant(app: &mut App, ui: &mut Ui) {
    lab_header(ui, "MINI-LAB: KUANTISASI");
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.add(
            egui::Slider::new(&mut app.scroll.quant_params_m, 0.1..=10.0)
                .text("Parameter (juta)")
                .step_by(0.1),
        );
        ui.horizontal_wrapped(|ui| {
            let f32_on = !app.scroll.quant_int8;
            if theme::pixel_chip(ui, "float32 (4B)", f32_on).clicked() {
                app.scroll.quant_int8 = false;
            }
            if theme::pixel_chip(ui, "int8 (1B)", app.scroll.quant_int8).clicked() {
                app.scroll.quant_int8 = true;
            }
        });
        ui.add(
            egui::Slider::new(&mut app.scroll.quant_arena_kb, 0.0..=200.0)
                .text("Arena aktivasi RAM (KB)")
                .step_by(5.0),
        );
        ui.add(
            egui::Slider::new(&mut app.scroll.quant_flash_kb, 128.0..=4096.0)
                .text("Kapasitas Flash (KB)")
                .step_by(128.0),
        );
        ui.add(
            egui::Slider::new(&mut app.scroll.quant_ram_kb, 64.0..=1024.0)
                .text("Kapasitas RAM (KB)")
                .step_by(64.0),
        );
        let bytes = if app.scroll.quant_int8 { 1.0 } else { 4.0 };
        let size_kb = app.scroll.quant_params_m * 1_000_000.0 * bytes / 1024.0;
        let flash_ok = size_kb <= app.scroll.quant_flash_kb;
        let ram_ok = app.scroll.quant_arena_kb <= app.scroll.quant_ram_kb;
        ui.label(theme::vt_text(
            &format!("Ukuran model: {size_kb:.0} KB"),
            21.0,
            color::CYAN,
        ));
        ui.label(theme::vt_text(
            &format!(
                "FLASH: {size_kb:.0} KB / {:.0} KB — {}",
                app.scroll.quant_flash_kb,
                if flash_ok { "✓ MUAT" } else { "✗ TIDAK MUAT" }
            ),
            20.0,
            if flash_ok { color::GREEN } else { color::RED },
        ));
        ui.label(theme::vt_text(
            &format!(
                "RAM: {:.0} KB / {:.0} KB — {}",
                app.scroll.quant_arena_kb,
                app.scroll.quant_ram_kb,
                if ram_ok { "✓ MUAT" } else { "✗ TIDAK MUAT" }
            ),
            20.0,
            if ram_ok { color::GREEN } else { color::RED },
        ));
    });
}

fn show_lab_latency(app: &mut App, ui: &mut Ui) {
    lab_header(ui, "MINI-LAB: ANGGARAN LATENSI");
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        for (label, val) in [
            ("Akuisisi (ms)", &mut app.scroll.lat_akuisisi),
            ("Pra-pemrosesan (ms)", &mut app.scroll.lat_pra),
            ("Inferensi (ms)", &mut app.scroll.lat_inferensi),
            ("Pengiriman (ms)", &mut app.scroll.lat_kirim),
        ] {
            ui.add(egui::Slider::new(val, 0.0..=200.0).text(label).step_by(5.0));
        }
        ui.add(
            egui::Slider::new(&mut app.scroll.lat_limit, 50.0..=500.0)
                .text("Batas total (ms)")
                .step_by(10.0),
        );
        let total = app.scroll.lat_akuisisi
            + app.scroll.lat_pra
            + app.scroll.lat_inferensi
            + app.scroll.lat_kirim;
        let ratio = if app.scroll.lat_limit > 0.0 {
            (total / app.scroll.lat_limit).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        ui.label(theme::vt_text(
            &format!("Total: {total:.0} ms / batas {:.0} ms", app.scroll.lat_limit),
            21.0,
            if total <= app.scroll.lat_limit {
                color::GREEN
            } else {
                color::RED
            },
        ));
        theme::xp_bar(ui, ratio, &format!("{:.0}%", ratio * 100.0));
    });
}
