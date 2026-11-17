//! Karakter & Statistik (`DashboardPage.tsx` v2, 670 baris).
//!
//! Kartu statistik, tabel minggu yang ditaklukkan, progres per lantai,
//! topik terlemah (untuk diulang), tombol pelajaran per topik.

use dungeon_core::{
    formulas::{calculate_weeks_conquered, get_display_streak, get_level_progress},
    spellbook::get_weakest_topics,
};
use egui::Ui;

use crate::{
    app::{today_key, App, Screen},
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

pub fn show_dashboard(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("KARAKTER & STATISTIK", 18.0, color::GREEN));
    ui.add_space(8.0);

    let level = get_level_progress(app.state.xp);
    let streak = get_display_streak(app.state.streak, app.state.last_active_date.as_deref(), &today_key());
    let weeks = calculate_weeks_conquered(&app.state.completed_rooms);
    let total_minutes: f64 = app
        .state
        .study_sessions
        .iter()
        .map(|s| s.duration_minutes)
        .sum();

    // Kartu identitas.
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, color::PANEL_DARK);
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, color::GOLD),
                egui::StrokeKind::Outside,
            );
            sprites::draw_avatar(ui.painter(), rect.shrink(6.0), &app.state.avatar_id);
            ui.vertical(|ui| {
                ui.label(theme::pixel_text(&app.state.player_name, 14.0, color::CREAM));
                ui.label(theme::vt_text(
                    &format!("Level {} — {}", level.level, level.title),
                    21.0,
                    color::GOLD,
                ));
                ui.label(theme::vt_text(
                    &format!("Streak: {streak} hari · Total XP: {}", app.state.xp),
                    20.0,
                    color::GREEN,
                ));
            });
        });
    });
    ui.add_space(8.0);

    // Kartu statistik (grid 2 kolom).
    let stats: [(&str, String); 6] = [
        ("Ruangan selesai", format!("{}/28", app.state.completed_rooms.len())),
        ("Boss dikalahkan", format!("{}/5", app.state.defeated_bosses.len())),
        ("Quest Menara", format!("{}/7", app.state.completed_tower_quests.len())),
        ("Relik", format!("{}", app.state.relics.len())),
        ("Lencana", format!("{}", app.state.badges.len())),
        (
            "Total jam belajar",
            format!("{:.1} jam", total_minutes / 60.0),
        ),
    ];
    ui.horizontal_wrapped(|ui| {
        for (label, value) in stats {
            theme::pixel_box(ui, PixelBox::Normal, |ui| {
                ui.set_min_width(150.0);
                ui.label(theme::pixel_text(label.to_uppercase().as_str(), 9.0, color::GOLD));
                ui.label(theme::pixel_text(&value, 16.0, color::CREAM));
            });
        }
    });
    ui.add_space(8.0);

    // Minggu yang ditaklukkan.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("MINGGU DITAKLUKKAN", 11.0, color::GOLD));
        ui.label(theme::vt_text(
            &format!(
                "Efektif: {}–{} dari {}–{} minggu roadmap (mentah: {}–{} dari {}–{}).",
                weeks.effective_conquered_min,
                weeks.effective_conquered_max,
                weeks.effective_total_min,
                weeks.effective_total_max,
                weeks.conquered_min,
                weeks.conquered_max,
                weeks.raw_total_min,
                weeks.raw_total_max
            ),
            20.0,
            color::CREAM,
        ));
    });
    ui.add_space(8.0);

    // Topik terlemah (skor kuis terendah).
    let weakest = get_weakest_topics(&app.state.quiz_best_scores, 5);
    if !weakest.is_empty() {
        theme::pixel_box(ui, PixelBox::Red, |ui| {
            ui.label(theme::pixel_text("TOPIK TERLEMAH — ULANGI", 11.0, color::RED));
            for w in &weakest {
                ui.horizontal(|ui| {
                    ui.label(theme::vt_text(
                        &format!("No.{} — {} (skor terbaik {}%)", w.number, w.name, w.best_score_percent),
                        20.0,
                        color::CREAM,
                    ));
                    if theme::pixel_button(ui, "Latihan", color::DUNGEON, color::GOLD).clicked() {
                        app.open_battle(&w.topic_id);
                    }
                });
            }
        });
        ui.add_space(8.0);
    }

    // Progres per lantai.
    for floor in dungeon_data::roadmap::floors() {
        if floor.id == 6 {
            continue;
        }
        let done = floor
            .topics
            .iter()
            .filter(|t| app.state.completed_rooms.iter().any(|r| r == &t.id))
            .count();
        let total = floor.topics.len();
        let boss_done = app
            .state
            .defeated_bosses
            .iter()
            .any(|b| b == &floor.boss.id);
        let theme_color = sprites::parse_hex(&floor.theme_color);
        theme::pixel_box(ui, PixelBox::Normal, |ui| {
            ui.horizontal(|ui| {
                sprites::show(ui, &sprites::DOOR, theme_color, 36.0);
                ui.vertical(|ui| {
                    ui.label(theme::pixel_text(&floor.short_title, 11.0, theme_color));
                    ui.label(theme::vt_text(
                        &format!(
                            "Ruangan: {done}/{total} · Boss: {}",
                            if boss_done { "✓ dikalahkan" } else { "belum" }
                        ),
                        20.0,
                        color::CREAM,
                    ));
                });
            });
            ui.add_space(4.0);
            theme::xp_bar(ui, done as f32 / total.max(1) as f32, &format!("{done}/{total}"));
            ui.add_space(4.0);
            if theme::pixel_button(ui, "Buka Peta Lantai Ini", color::DUNGEON, color::CYAN)
                .clicked()
            {
                app.apply(dungeon_core::reducer::Action::SelectFloor {
                    floor_id: floor.id,
                });
                app.screen = Screen::Dungeon;
            }
        });
        ui.add_space(4.0);
    }
}
