//! Peta Dungeon (`DungeonMap.tsx` v2, 756 baris).
//!
//! Pemilih 6 pintu lantai, node ruangan zigzag, ruang boss, dan tata
//! letak Guild Hall Lantai 5 (8 kartu quest + pintu boss tengah).
//! Lantai 6 mengarah ke layar Menara Master. Catatan bukti (evidence)
//! Lantai 5 bisa diisi per topik.

use dungeon_core::{
    formulas::{calculate_room_xp, is_boss_unlocked, is_floor_unlocked, is_room_unlocked},
    reducer::Action,
};
use egui::Ui;

use crate::{
    app::{App, Dialog, Screen},
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

pub fn show_dungeon(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("PETA DUNGEON", 18.0, color::GREEN));
    ui.label(theme::vt_text(
        "Pilih lantai, baca scroll, lalu kalahkan monster tiap ruangan.",
        20.0,
        color::MUTED,
    ));
    ui.add_space(8.0);

    // Pemilih pintu lantai (6 pintu).
    ui.horizontal_wrapped(|ui| {
        for floor in dungeon_data::roadmap::floors() {
            let label = if floor.id == 6 {
                "L6 Menara".to_string()
            } else {
                format!("L{}", floor.id)
            };
            let unlocked = is_floor_unlocked(floor.id, &app.state.defeated_bosses);
            let active = app.state.active_floor_id == floor.id;
            let text = if unlocked {
                label
            } else {
                format!("{label} 🔒")
            };
            if theme::pixel_chip(ui, &text, active).clicked() && unlocked {
                app.apply(Action::SelectFloor { floor_id: floor.id });
                if floor.id == 6 {
                    app.screen = Screen::Tower;
                }
            }
        }
    });
    ui.add_space(8.0);

    let floor_id = app.state.active_floor_id;
    if floor_id == 6 {
        // Lantai 6 = Menara Master (layar sendiri).
        theme::pixel_box(ui, PixelBox::Gold, |ui| {
            ui.label(theme::vt_text(
                "Lantai 6 adalah Menara Master — 7 quest Road to Master, Penjaga Puncak, dan Piagam Kelulusan.",
                21.0,
                color::CREAM,
            ));
            ui.add_space(6.0);
            if theme::pixel_button(ui, "Buka Menara Master", color::GOLD_DARK, color::CREAM)
                .clicked()
            {
                app.screen = Screen::Tower;
            }
        });
        return;
    }

    let Some(floor) = dungeon_data::roadmap::floor(floor_id) else {
        ui.label(theme::vt_text("Lantai tidak dikenal.", 21.0, color::RED));
        return;
    };
    let theme_color = sprites::parse_hex(&floor.theme_color);

    // Kepala lantai.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.horizontal(|ui| {
            sprites::show(ui, &sprites::DOOR, theme_color, 48.0);
            ui.vertical(|ui| {
                ui.label(theme::pixel_text(&floor.short_title, 13.0, theme_color));
                ui.label(theme::vt_text(&floor.phase_title, 20.0, color::CREAM));
                ui.label(theme::vt_text(
                    &format!(
                        "Monster: {} · {}–{} minggu",
                        floor.monster_family, floor.weeks.min, floor.weeks.max
                    ),
                    19.0,
                    color::MUTED,
                ));
            });
        });
        if let Some(tag) = &floor.parallel_tag {
            ui.label(theme::vt_text(tag, 19.0, color::GOLD));
        }
    });
    ui.add_space(8.0);

    // Node ruangan (zigzag: indentasi selang-seling).
    for (i, topic) in floor.topics.iter().enumerate() {
        let done = app
            .state
            .completed_rooms
            .iter()
            .any(|r| r == &topic.id);
        let unlocked = is_room_unlocked(
            topic,
            &floor.topics,
            &app.state.completed_rooms,
            &app.state.defeated_bosses,
        );
        let indent = if i % 2 == 0 { 0.0 } else { 24.0 };
        ui.horizontal(|ui| {
            ui.add_space(indent);
            theme::pixel_box(
                ui,
                if done { PixelBox::Green } else { PixelBox::Normal },
                |ui| {
                    ui.set_width(ui.available_width());
                    room_node(app, ui, floor_id, topic, done, unlocked, theme_color);
                },
            );
        });
        ui.add_space(4.0);
    }

    // Ruang boss.
    ui.add_space(8.0);
    let boss_unlocked = is_boss_unlocked(
        floor_id,
        &floor.topics,
        &app.state.completed_rooms,
        &app.state.defeated_bosses,
    );
    let boss_done = app
        .state
        .defeated_bosses
        .iter()
        .any(|b| b == &floor.boss.id);
    theme::pixel_box(ui, if boss_done { PixelBox::Gold } else { PixelBox::Red }, |ui| {
        ui.horizontal(|ui| {
            sprites::show(
                ui,
                sprites::monster_sprite(&floor.boss.sprite_type),
                color::RED,
                56.0,
            );
            ui.vertical(|ui| {
                ui.label(theme::pixel_text("RUANG BOSS", 11.0, color::RED));
                ui.label(theme::pixel_text(&floor.boss.name, 14.0, color::CREAM));
                ui.label(theme::vt_text(&floor.boss.title, 20.0, color::GOLD));
                ui.label(theme::vt_text(&floor.boss.challenge, 19.0, color::MUTED));
            });
        });
        ui.add_space(6.0);
        if boss_done {
            ui.label(theme::vt_text("✓ Boss sudah dikalahkan.", 21.0, color::GREEN));
        } else if boss_unlocked {
            if theme::pixel_button(ui, "⚔  Lawan Boss", color::RED_DARK, color::CREAM).clicked() {
                app.apply(Action::SetActiveNode {
                    floor_id,
                    node_id: format!("boss-{floor_id}"),
                });
                app.open_boss(floor_id);
            }
        } else {
            ui.label(theme::vt_text(
                "Terkunci — selesaikan semua ruangan lantai ini dulu.",
                20.0,
                color::MUTED,
            ));
        }
    });
}

fn room_node(
    app: &mut App,
    ui: &mut Ui,
    floor_id: u32,
    topic: &dungeon_data::roadmap::TopicItem,
    done: bool,
    unlocked: bool,
    theme_color: egui::Color32,
) {
    ui.horizontal(|ui| {
        sprites::show(ui, sprites::monster_sprite(&topic_monster_type(topic, floor_id)), theme_color, 40.0);
        ui.vertical(|ui| {
            let status = if done {
                " ✓ SELESAI"
            } else if unlocked {
                ""
            } else {
                " 🔒"
            };
            ui.label(theme::pixel_text(
                &format!("No.{} — {}{}", topic.number, topic.name, status),
                11.0,
                if done { color::GREEN } else { color::CREAM },
            ));
            ui.label(theme::vt_text(
                &format!(
                    "{} · {}–{} minggu · ~{} XP",
                    topic.scope,
                    topic.weeks.min,
                    topic.weeks.max,
                    calculate_room_xp(topic.weeks.min, topic.weeks.max)
                ),
                19.0,
                color::MUTED,
            ));
            if let Some(note) = &topic.parallel_note {
                ui.label(theme::vt_text(note, 18.0, color::GOLD));
            }
            if !topic.flavor.is_empty() {
                ui.label(theme::vt_text(&topic.flavor, 18.0, color::MUTED));
            }
        });
    });
    ui.add_space(4.0);
    if !unlocked {
        ui.label(theme::vt_text(
            "Terkunci — selesaikan ruangan sebelumnya dulu.",
            19.0,
            color::MUTED,
        ));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        if theme::pixel_button(ui, "📜 Baca Scroll", color::DUNGEON, color::CYAN).clicked() {
            app.dialog = Some(Dialog::Scroll {
                topic_id: topic.id.clone(),
            });
        }
        if theme::pixel_button(ui, "📖 Pelajaran", color::DUNGEON, color::GOLD).clicked() {
            // Buka chunk pertama topik ini (bila ada).
            let chunks =
                dungeon_core::lesson_engine::get_lesson_chunks_for_topic_number(topic.number);
            if let Some(chunk) = chunks.first() {
                app.open_lesson(&chunk.chunk_id);
            } else {
                app.dialog = Some(Dialog::Notice {
                    title: "Pelajaran".to_string(),
                    body: format!("Topik \"{}\" belum punya chunk pelajaran.", topic.name),
                });
            }
        }
        if done {
            let best = app
                .state
                .best_score_by_topic
                .get(&topic.id)
                .copied()
                .unwrap_or(0);
            ui.label(theme::vt_text(
                &format!("Skor kuis terbaik: {best}%"),
                19.0,
                color::GREEN,
            ));
            if theme::pixel_button(ui, "⚔  Ulang Battle", color::DUNGEON, color::CREAM).clicked()
            {
                app.apply(Action::SetActiveNode {
                    floor_id,
                    node_id: topic.id.clone(),
                });
                app.open_battle(&topic.id);
            }
        } else if theme::pixel_button(ui, "⚔  Masuk Ruangan", color::GREEN_DARK, color::CREAM)
            .clicked()
        {
            app.apply(Action::SetActiveNode {
                floor_id,
                node_id: topic.id.clone(),
            });
            app.open_battle(&topic.id);
        }
    });

    // Guild Hall Lantai 5: catatan bukti + nama relik.
    if floor_id == 5 {
        if let Some(content) = dungeon_data::content::floor_content(5)
            .and_then(|fc| fc.topics.iter().find(|t| t.topic_id == topic.number))
        {
            ui.add_space(6.0);
            if let Some(relic) = &content.relic {
                ui.label(theme::vt_text(&format!("Relik: {relic}"), 19.0, color::GOLD));
            }
            if let Some(hint) = &content.evidence_hint {
                ui.label(theme::vt_text(&format!("Bukti: {hint}"), 19.0, color::MUTED));
            }
            let mut note = app
                .state
                .evidence_notes
                .get(&topic.id)
                .cloned()
                .unwrap_or_default();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut note)
                    .font(egui::FontId::new(
                        20.0,
                        egui::FontFamily::Name(theme::FONT_VT.into()),
                    ))
                    .desired_width(ui.available_width())
                    .hint_text("Catatan bukti (mis. tautan repo / file)…"),
            );
            if resp.lost_focus() || resp.changed() {
                app.apply(Action::SetEvidenceNote {
                    topic_id: topic.id.clone(),
                    note: note.clone(),
                });
            }
        }
    }
}

/// Tipe monster ruangan: dari `monsterType` lantai (v2 memakai satu
/// keluarga monster per lantai).
fn topic_monster_type(topic: &dungeon_data::roadmap::TopicItem, _floor_id: u32) -> String {
    dungeon_data::roadmap::floor(topic.floor_id)
        .map(|f| f.monster_type.clone())
        .unwrap_or_else(|| "slime".to_string())
}
