//! Pengaturan (`SettingsPage.tsx` v2, 503 baris).
//!
//! Suara & musik, kurangi gerakan, skala interval review, profil,
//! manajemen save, reset progres. Ekspor/impor save = Fase 4.

use dungeon_core::reducer::Action;
use egui::Ui;

use crate::{
    app::{App, Screen},
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

#[derive(Default)]
pub struct SettingsUi {
    pub confirm_reset: bool,
}

pub fn show_settings(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("PENGATURAN", 18.0, color::GREEN));
    ui.add_space(8.0);

    // Profil (baca saja di v3.0 — lihat catatan di bawah).
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("PROFIL", 11.0, color::GOLD));
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(56.0, 56.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, color::PANEL_DARK);
            sprites::draw_avatar(ui.painter(), rect.shrink(6.0), &app.state.avatar_id);
            ui.vertical(|ui| {
                ui.label(theme::vt_text(&app.state.player_name, 22.0, color::CREAM));
                ui.label(theme::vt_text(
                    &format!("Avatar: {}", app.state.avatar_id),
                    19.0,
                    color::MUTED,
                ));
            });
        });
        ui.label(theme::vt_text(
            "Ubah nama/avatar setelah petualangan dimulai belum tersedia di v3.0 (tidak ada aksi di dungeon-core — lihat ui.md).",
            18.0,
            color::MUTED,
        ));
    });
    ui.add_space(8.0);

    // Suara.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("SUARA", 11.0, color::GOLD));
        let mut sound = app.state.sound_enabled;
        if ui.checkbox(&mut sound, "Suara aktif").clicked() {
            app.apply(Action::ToggleSound);
        }
        let mut sfx = app.state.sfx_volume;
        let sfx_resp = ui.add(
            egui::Slider::new(&mut sfx, 0.0..=1.0)
                .text("Volume efek suara")
                .step_by(0.05),
        );
        if sfx_resp.changed() {
            app.apply(Action::SetSfxVolume { volume: sfx });
        }
        let mut music = app.state.music_enabled;
        if ui.checkbox(&mut music, "Musik aktif").clicked() {
            app.apply(Action::ToggleMusic);
        }
        let mut mvol = app.state.music_volume;
        let m_resp = ui.add(
            egui::Slider::new(&mut mvol, 0.0..=1.0)
                .text("Volume musik")
                .step_by(0.05),
        );
        if m_resp.changed() {
            app.apply(Action::SetMusicVolume { volume: mvol });
        }
        ui.label(theme::vt_text(
            "Catatan: audio (chiptune kira) hadir di fase terpisah — pengaturan di sini sudah tersimpan untuk nanti.",
            18.0,
            color::MUTED,
        ));
    });
    ui.add_space(8.0);

    // Gerakan & review.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("GERAKAN & ULASAN", 11.0, color::GOLD));
        let mut rm = app.state.reduce_motion;
        if ui.checkbox(&mut rm, "Kurangi gerakan (matikan scanline/animasi)").clicked() {
            app.apply(Action::ToggleReduceMotion);
        }
        let mut scale = app.state.settings.interval_scale;
        let s_resp = ui.add(
            egui::Slider::new(&mut scale, 0.5..=2.0)
                .text("Skala interval review")
                .step_by(0.1),
        );
        if s_resp.changed() {
            app.apply(Action::SetIntervalScale { scale });
        }
        ui.label(theme::vt_text(
            "Skala 0,5 = kartu kembali lebih cepat; 2,0 = lebih lambat.",
            18.0,
            color::MUTED,
        ));
    });
    ui.add_space(8.0);

    // Manajemen save.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("MANAJEMEN SAVE", 11.0, color::GOLD));
        ui.label(theme::vt_text(
            "Ekspor/impor save (JSON, validator maks 1 MB seperti v2) hadir di Fase 4 — penyimpanan Android.",
            19.0,
            color::MUTED,
        ));
        ui.add_space(4.0);
        if !app.settings.confirm_reset {
            if theme::pixel_button(ui, "🗑  Reset Progres", color::RED_DARK, color::CREAM)
                .clicked()
            {
                app.settings.confirm_reset = true;
            }
        } else {
            ui.label(theme::vt_text(
                "Yakin hapus SEMUA progres? Tindakan ini tidak bisa dibatalkan.",
                20.0,
                color::RED,
            ));
            ui.horizontal_wrapped(|ui| {
                if theme::pixel_button(ui, "Ya, Hapus Semua", color::RED_DARK, color::CREAM)
                    .clicked()
                {
                    app.apply(Action::ResetProgress);
                    app.settings.confirm_reset = false;
                    app.screen = Screen::Title;
                }
                if theme::pixel_button(ui, "Batal", color::DUNGEON, color::CREAM).clicked() {
                    app.settings.confirm_reset = false;
                }
            });
        }
    });
    ui.add_space(8.0);

    // Tentang.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("TENTANG", 11.0, color::GOLD));
        ui.label(theme::vt_text(
            "Dungeon Learning ML & DL — v3.0 (Rust native Android). 100% offline, tanpa AI Mentor.",
            19.0,
            color::CREAM,
        ));
        ui.label(theme::vt_text(
            "Tema & konten dari v2 (React/TypeScript). Paket: dungeonlearning.app.",
            18.0,
            color::MUTED,
        ));
    });
}
