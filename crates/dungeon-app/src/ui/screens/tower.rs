//! Menara Master (`TowerPage.tsx` v2, 1299 baris).
//!
//! 7 quest Road to Master (langkah + bukti), spesialisasi,
//! Penjaga Puncak (refleksi Cermin Puncak), dan Piagam Kelulusan.

use std::collections::BTreeMap;

use dungeon_core::{
    formulas::get_level_progress,
    reducer::Action,
    spellbook::{
        can_unlock_cermin_puncak, format_certificate_summary_text, CertificateSummaryInput,
    },
};
use egui::Ui;

use crate::{
    app::App,
    ui::theme::{self, color, PixelBox},
};

#[derive(Default)]
pub struct TowerUi {
    pub selected_quest: Option<String>,
    pub reflection_inputs: BTreeMap<String, String>,
    pub show_certificate: bool,
}

pub fn show_tower(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("MENARA MASTER", 18.0, color::GOLD));
    let f6 = dungeon_data::content::floor6_content();
    ui.label(theme::vt_text(&f6.intro, 20.0, color::MUTED));
    ui.add_space(8.0);

    let completed: Vec<String> = app.state.completed_tower_quests.clone();
    ui.label(theme::vt_text(
        &format!("Quest selesai: {}/7", completed.len()),
        21.0,
        color::GREEN,
    ));
    ui.add_space(8.0);

    // Daftar quest.
    for quest in &f6.quests {
        let done = completed.iter().any(|q| q == &quest.id);
        let expanded = app.tower.selected_quest.as_deref() == Some(quest.id.as_str());
        theme::pixel_box(ui, if done { PixelBox::Green } else { PixelBox::Normal }, |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::pixel_text(
                    &format!("{}{}", if done { "✓ " } else { "" }, quest.title),
                    12.0,
                    if done { color::GREEN } else { color::CREAM },
                ));
            });
            ui.label(theme::vt_text(&quest.summary, 20.0, color::MUTED));
            ui.label(theme::vt_text(
                &format!("XP: {} · Relik: {}", quest.xp, quest.relic),
                19.0,
                color::GOLD,
            ));
            if theme::pixel_button(
                ui,
                if expanded { "Tutup Detail" } else { "Lihat Detail" },
                color::DUNGEON,
                color::CYAN,
            )
            .clicked()
            {
                app.tower.selected_quest = if expanded {
                    None
                } else {
                    Some(quest.id.clone())
                };
            }
            if expanded {
                show_quest_detail(app, ui, quest);
            }
        });
        ui.add_space(4.0);
    }

    // Spesialisasi.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("SPESIALISASI", 11.0, color::GOLD));
        ui.label(theme::vt_text(&f6.specialization_note, 19.0, color::MUTED));
        ui.horizontal_wrapped(|ui| {
            for spec in &f6.specializations {
                let active = app.state.specialization.as_deref() == Some(spec.as_str());
                if theme::pixel_chip(ui, spec, active).clicked() {
                    app.apply(Action::SetSpecialization {
                        specialization: if active {
                            None
                        } else {
                            Some(spec.clone())
                        },
                    });
                }
            }
        });
    });
    ui.add_space(8.0);

    // Penjaga Puncak (Cermin Puncak).
    let mirror_unlocked = can_unlock_cermin_puncak(&app.state.tower_completed);
    theme::pixel_box(ui, if mirror_unlocked { PixelBox::Gold } else { PixelBox::Normal }, |ui| {
        ui.label(theme::pixel_text("CERMIN PUNCAK", 12.0, color::GOLD));
        ui.label(theme::vt_text(
            &format!("Penjaga: {}", f6.guardian.name),
            20.0,
            color::CREAM,
        ));
        ui.label(theme::vt_text(&f6.guardian.intro, 19.0, color::MUTED));
        if !mirror_unlocked {
            ui.label(theme::vt_text(
                "Terkunci — selesaikan semua quest Menara Master dulu.",
                19.0,
                color::MUTED,
            ));
        } else {
            for prompt in &f6.guardian.reflection_prompts {
                ui.label(theme::vt_text(&prompt.text, 20.0, color::CREAM));
                let mut answer = app
                    .tower
                    .reflection_inputs
                    .get(&prompt.id)
                    .cloned()
                    .unwrap_or_else(|| {
                        app.state
                            .reflections
                            .get(&prompt.id)
                            .cloned()
                            .unwrap_or_default()
                    });
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut answer)
                        .font(egui::FontId::new(
                            20.0,
                            egui::FontFamily::Name(theme::FONT_VT.into()),
                        ))
                        .desired_width(ui.available_width())
                        .desired_rows(2),
                );
                if resp.changed() {
                    app.tower
                        .reflection_inputs
                        .insert(prompt.id.clone(), answer.clone());
                }
                if resp.lost_focus() {
                    app.apply(Action::SetReflectionAnswer {
                        prompt_id: prompt.id.clone(),
                        answer: answer.clone(),
                    });
                }
            }
        }
    });
    ui.add_space(8.0);

    // Piagam kelulusan.
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.label(theme::pixel_text("PIAGAM KELULUSAN", 12.0, color::GOLD));
        if let Some(issued_at) = &app.state.certificate_issued_at {
            let level = get_level_progress(app.state.xp);
            let summary = format_certificate_summary_text(&CertificateSummaryInput {
                player_name: app.state.player_name.clone(),
                level: level.level,
                level_title: level.title.to_string(),
                rooms_defeated: app.state.completed_rooms.len(),
                total_rooms: 28,
                bosses_defeated: app.state.defeated_bosses.len(),
                total_bosses: 5,
                relics_count: app.state.relics.len(),
                total_relics: count_all_relics(),
                tower_quests_completed: app.state.completed_tower_quests.len(),
                total_tower_quests: f6.quests.len(),
                specialization: app.state.specialization.clone(),
                issued_date_formatted: issued_at.clone(),
            });
            ui.label(theme::vt_text(&f6.certificate.title, 22.0, color::GOLD));
            ui.label(theme::vt_text(&summary, 20.0, color::CREAM));
            ui.label(theme::vt_text(&f6.certificate.honest_note, 19.0, color::MUTED));
            ui.label(theme::vt_text(&f6.certificate.closing, 19.0, color::MUTED));
            ui.label(theme::vt_text(
                &format!("Diterbitkan: {issued_at}"),
                18.0,
                color::MUTED,
            ));
        } else if mirror_unlocked {
            ui.label(theme::vt_text(
                "Semua quest selesai dan Cermin Puncak terbuka — piagam siap diterbitkan.",
                20.0,
                color::CREAM,
            ));
            if theme::pixel_button(ui, "🎓 Terbitkan Piagam", color::GOLD_DARK, color::CREAM)
                .clicked()
            {
                app.apply(Action::IssueCertificate);
            }
        } else {
            ui.label(theme::vt_text(
                "Piagam diterbitkan setelah semua quest Menara Master selesai.",
                19.0,
                color::MUTED,
            ));
        }
    });
}

/// Detail satu quest: langkah (checkbox), petunjuk bukti, referensi,
/// tombol selesaikan.
fn show_quest_detail(
    app: &mut App,
    ui: &mut Ui,
    quest: &dungeon_data::content::TowerQuestContent,
) {
    ui.add_space(4.0);
    ui.label(theme::pixel_text("LANGKAH", 10.0, color::GOLD));
    let steps_state = app
        .state
        .tower_steps
        .get(&quest.id)
        .cloned()
        .unwrap_or_default();
    for step in &quest.steps {
        let checked = steps_state.get(&step.id).copied().unwrap_or(false);
        let mut check = checked;
        ui.horizontal(|ui| {
            if ui.checkbox(&mut check, "").clicked() && check != checked {
                app.apply(Action::ToggleTowerStep {
                    quest_id: quest.id.clone(),
                    step_id: step.id.clone(),
                });
            }
            ui.label(theme::vt_text(&step.text, 20.0, color::CREAM));
        });
    }
    ui.label(theme::vt_text(
        &format!("Petunjuk bukti: {}", quest.evidence_hint),
        19.0,
        color::MUTED,
    ));
    if !quest.references.is_empty() {
        ui.label(theme::pixel_text("REFERENSI", 10.0, color::GOLD));
        for r in &quest.references {
            let text = match &r.url {
                Some(url) => format!("• {} ({url})", r.label),
                None => format!("• {}", r.label),
            };
            ui.label(theme::vt_text(&text, 18.0, color::CYAN));
        }
    }
    ui.add_space(6.0);
    let done = app
        .state
        .completed_tower_quests
        .iter()
        .any(|q| q == &quest.id);
    if done {
        ui.label(theme::vt_text("✓ Quest selesai.", 21.0, color::GREEN));
    } else if theme::pixel_button(ui, "Tandai Quest Selesai", color::GREEN_DARK, color::CREAM)
        .clicked()
    {
        app.apply(Action::CompleteTowerQuest {
            quest_id: quest.id.clone(),
        });
    }
}

/// Jumlah seluruh relik yang ada di roadmap (untuk ringkasan piagam).
fn count_all_relics() -> usize {
    let mut n = 0;
    for floor in dungeon_data::roadmap::floors() {
        n += 1; // relik boss
        for topic in &floor.topics {
            if topic.relic.is_some() {
                n += 1;
            }
        }
    }
    n
}
