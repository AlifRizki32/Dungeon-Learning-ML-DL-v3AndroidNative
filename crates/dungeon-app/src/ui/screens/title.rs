//! Layar judul (`TitleScreen.tsx` v2).
//!
//! Logo crest + judul "DUNGEON BELAJAR ML & DL", banner progres
//! tersimpan, input nama (maks 28 karakter, bawaan "Petualang Tensor"),
//! 4 avatar dari `AVATARS`, tombol mulai, catatan roadmap 66–88 minggu.
//! Tidak ada menu AI Mentor (dilarang di v3.0).

use dungeon_core::reducer::Action;
use egui::Ui;

use crate::{
    app::{App, Screen},
    ui::{
        sprites,
        theme::{self, color},
    },
};

pub fn show_title(app: &mut App, ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(theme::SAFE_TOP + 24.0);
        // Logo crest: pedang + pintu di atas panel emas.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, color::PANEL);
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(3.0, color::GOLD),
            egui::StrokeKind::Outside,
        );
        sprites::draw(ui.painter(), rect.shrink(14.0), &sprites::SWORD, color::CREAM);

        ui.add_space(16.0);
        ui.label(theme::pixel_text("DUNGEON", 34.0, color::GREEN));
        ui.label(theme::pixel_text("BELAJAR", 22.0, color::CREAM));
        ui.label(theme::pixel_text("ML & DL", 26.0, color::GOLD));
        ui.add_space(8.0);
        ui.label(theme::vt_text(
            "RPG belajar Machine Learning & Deep Learning — 28 topik, 6 lantai, 5 boss.",
            22.0,
            color::MUTED,
        ));
        ui.add_space(16.0);

        if app.state.has_started {
            // Banner progres tersimpan (v2: saved-progress banner).
            theme::pixel_box(ui, theme::PixelBox::Gold, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(theme::vt_text(
                        &format!(
                            "Petualangan tersimpan: {} · Lv.{} · {} XP · {} ruangan selesai",
                            app.state.player_name,
                            dungeon_core::formulas::get_level_progress(app.state.xp).level,
                            app.state.xp,
                            app.state.completed_rooms.len()
                        ),
                        21.0,
                        color::CREAM,
                    ));
                    ui.add_space(8.0);
                    if theme::pixel_button(
                        ui,
                        "▶  Lanjutkan Petualangan",
                        color::GREEN_DARK,
                        color::CREAM,
                    )
                    .clicked()
                    {
                        app.apply(Action::ContinueAdventure);
                        app.screen = Screen::Dungeon;
                    }
                    ui.add_space(4.0);
                    if theme::pixel_button(
                        ui,
                        "Petualangan Baru (hapus progres lama)",
                        color::DUNGEON,
                        color::RED,
                    )
                    .clicked()
                    {
                        app.apply(Action::ResetProgress);
                    }
                });
            });
        } else {
            new_adventure_form(app, ui);
        }

        ui.add_space(16.0);
        ui.label(theme::vt_text(
            "Peta roadmap: 66–88 minggu efektif (2–4 jam/minggu).",
            19.0,
            color::MUTED,
        ));
        ui.label(theme::vt_text(
            "100% offline — tanpa akun, tanpa server.",
            19.0,
            color::MUTED,
        ));
        ui.add_space(theme::SAFE_BOTTOM);
    });
}

/// Formulir petualangan baru: nama + pilih avatar + mulai.
fn new_adventure_form(app: &mut App, ui: &mut Ui) {
    theme::pixel_box(ui, theme::PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("NAMA PETUALANG", 11.0, color::GOLD));
        let mut name = app.title_name.clone();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut name)
                .font(egui::FontId::new(
                    22.0,
                    egui::FontFamily::Name(theme::FONT_VT.into()),
                ))
                .desired_width(ui.available_width())
                .hint_text("Petualang Tensor"),
        );
        if resp.changed() {
            // v2: maks 28 karakter.
            name.truncate(28);
            app.title_name = name;
        }
        ui.add_space(8.0);
        ui.label(theme::pixel_text("PILIH AVATAR", 11.0, color::GOLD));
        ui.add_space(4.0);
        let avatars = &dungeon_data::roadmap::roadmap().avatars;
        ui.horizontal_wrapped(|ui| {
            for av in avatars {
                let selected = app.title_avatar == av.id;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(64.0, 64.0),
                    egui::Sense::click(),
                );
                let bg = if selected { color::GOLD_BOX_BG } else { color::PANEL_INNER };
                ui.painter().rect_filled(rect, 0.0, bg);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(
                        2.0,
                        if selected { color::GOLD } else { color::BORDER_LIGHT },
                    ),
                    egui::StrokeKind::Outside,
                );
                sprites::draw_avatar(ui.painter(), rect.shrink(8.0), &av.id);
                if resp.clicked() {
                    app.title_avatar = av.id.clone();
                }
            }
        });
        // Nama + peran avatar terpilih.
        if let Some(av) = avatars.iter().find(|a| a.id == app.title_avatar) {
            ui.label(theme::vt_text(
                &format!("{} — {}", av.name, av.role),
                20.0,
                color::GOLD,
            ));
        }
        ui.add_space(10.0);
        let name = if app.title_name.trim().is_empty() {
            "Petualang Tensor".to_string()
        } else {
            app.title_name.trim().to_string()
        };
        if theme::pixel_button(ui, "▶  MULAI PETUALANGAN", color::GREEN_DARK, color::CREAM)
            .clicked()
        {
            app.apply(Action::StartAdventure {
                name,
                avatar_id: app.title_avatar.clone(),
            });
            app.screen = Screen::Dungeon;
        }
    });
}
