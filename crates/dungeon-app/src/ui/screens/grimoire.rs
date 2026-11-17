//! Grimoire & Dojo (`GrimoirePage.tsx` v2, 583 baris).
//!
//! Kartu mantra (spell) + scroll per lantai, skor kuis terbaik per
//! topik, dan Dojo: latihan soal berbobot riwayat dari topik yang
//! sudah dikalahkan (`build_dojo_question_set`).

use dungeon_core::{
    reducer::Action,
    spellbook::{build_dojo_question_set, DojoQuestion},
};
use egui::Ui;

use crate::{
    app::{App, Dialog},
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

/// Satu sesi Dojo.
pub struct DojoSession {
    pub questions: Vec<DojoQuestion>,
    pub index: usize,
    pub selected: Option<usize>,
    pub correct_count: i64,
    pub xp_earned: i64,
    pub finished: bool,
}

#[derive(Default)]
pub struct GrimoireUi {
    pub floor_tab: u32,
    pub dojo: Option<DojoSession>,
}

impl GrimoireUi {
    fn floor_tab(&self) -> u32 {
        if self.floor_tab == 0 {
            1
        } else {
            self.floor_tab
        }
    }
}

pub fn show_grimoire(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("GRIMOIRE & DOJO", 18.0, color::GREEN));
    ui.label(theme::vt_text(
        "Kumpulan mantra (rumus) dan scroll (materi) tiap topik — plus Dojo untuk latihan soal.",
        20.0,
        color::MUTED,
    ));
    ui.add_space(8.0);

    // Dojo di atas (bila sesi berjalan). Sesi diambil keluar dulu supaya
    // `show_dojo` bisa memanggil `app.apply` tanpa pinjam ganda.
    let mut dojo_open = app.grimoire.dojo.is_some();
    if dojo_open {
        if let Some(mut session) = app.grimoire.dojo.take() {
            let mut close = false;
            show_dojo(app, ui, &mut session, &mut close);
            if !close {
                app.grimoire.dojo = Some(session);
            }
        }
        dojo_open = app.grimoire.dojo.is_some();
    }
    if !dojo_open {
        theme::pixel_box(ui, PixelBox::Gold, |ui| {
            ui.label(theme::pixel_text("DOJO", 12.0, color::GOLD));
            ui.label(theme::vt_text(
                &format!(
                    "Latihan soal dari topik yang sudah dikalahkan. XP Dojo hari ini: {} (batas harian 30 XP).",
                    app.state.dojo_daily_xp.xp
                ),
                20.0,
                color::CREAM,
            ));
            let can_start = !app.state.completed_rooms.is_empty();
            if !can_start {
                ui.label(theme::vt_text(
                    "Selesaikan satu ruangan dulu untuk membuka Dojo.",
                    19.0,
                    color::MUTED,
                ));
            }
            if theme::pixel_button(ui, "🥋  Mulai Latihan Dojo", color::GOLD_DARK, color::CREAM)
                .clicked()
                && can_start
            {
                let questions = build_dojo_question_set(
                    &app.state.completed_rooms,
                    &app.state.question_stats,
                    10,
                    &mut app.rng,
                );
                if questions.is_empty() {
                    app.toast = Some("Dojo belum punya soal — selesaikan ruangan dulu.".to_string());
                } else {
                    app.grimoire.dojo = Some(DojoSession {
                        questions,
                        index: 0,
                        selected: None,
                        correct_count: 0,
                        xp_earned: 0,
                        finished: false,
                    });
                }
            }
        });
        ui.add_space(8.0);
    }

    // Tab lantai.
    ui.horizontal_wrapped(|ui| {
        for floor_id in 1..=5u32 {
            let active = app.grimoire.floor_tab() == floor_id;
            if theme::pixel_chip(ui, &format!("Lantai {floor_id}"), active).clicked() {
                app.grimoire.floor_tab = floor_id;
            }
        }
    });
    ui.add_space(8.0);

    let floor_id = app.grimoire.floor_tab();
    let Some(floor) = dungeon_data::roadmap::floor(floor_id) else {
        return;
    };
    let Some(content) = dungeon_data::content::floor_content(floor_id) else {
        return;
    };
    let theme_color = sprites::parse_hex(&floor.theme_color);

    for topic in &floor.topics {
        let Some(tc) = content.topics.iter().find(|t| t.topic_id == topic.number) else {
            continue;
        };
        let unlocked = app.state.spells_unlocked.iter().any(|s| s == &tc.spell.id);
        let done = app
            .state
            .completed_rooms
            .iter()
            .any(|r| r == &topic.id);
        theme::pixel_box(ui, if unlocked { PixelBox::Normal } else { PixelBox::Red }, |ui| {
            ui.horizontal(|ui| {
                sprites::show(ui, &sprites::BOOK, theme_color, 40.0);
                ui.vertical(|ui| {
                    ui.label(theme::pixel_text(
                        &format!("No.{} — {}", topic.number, topic.name),
                        11.0,
                        if done { color::GREEN } else { color::CREAM },
                    ));
                    ui.label(theme::vt_text(
                        &format!("Mantra: {}", tc.spell.name),
                        21.0,
                        color::GOLD,
                    ));
                });
            });
            ui.add_space(4.0);
            if unlocked {
                for formula in &tc.spell.formulas {
                    ui.label(theme::vt_text(&format!("• {formula}"), 20.0, color::CREAM));
                }
                ui.label(theme::vt_text(
                    &format!("Scroll: {} paragraf", tc.scroll.paragraphs.len()),
                    19.0,
                    color::MUTED,
                ));
                if let Some(best) = app.state.best_score_by_topic.get(&topic.id) {
                    ui.label(theme::vt_text(
                        &format!("Skor kuis terbaik: {best}%"),
                        19.0,
                        color::GREEN,
                    ));
                }
                ui.horizontal_wrapped(|ui| {
                    if theme::pixel_button(ui, "📜 Baca Scroll", color::DUNGEON, color::CYAN)
                        .clicked()
                    {
                        app.dialog = Some(Dialog::Scroll {
                            topic_id: topic.id.clone(),
                        });
                    }
                    let chunks =
                        dungeon_core::lesson_engine::get_lesson_chunks_for_topic_number(
                            topic.number,
                        );
                    if let Some(chunk) = chunks.first() {
                        if theme::pixel_button(ui, "📖 Pelajaran", color::DUNGEON, color::GOLD)
                            .clicked()
                        {
                            app.open_lesson(&chunk.chunk_id);
                        }
                    }
                });
            } else {
                ui.label(theme::vt_text(
                    "Mantra terkunci — kalahkan monster ruangan ini untuk membuka.",
                    19.0,
                    color::MUTED,
                ));
            }
        });
        ui.add_space(4.0);
    }
}

/// Panel Dojo: satu soal per langkah, pilihan jawaban, umpan balik,
/// XP per jawaban benar (dihitung reducer).
fn show_dojo(app: &mut App, ui: &mut Ui, session: &mut DojoSession, close: &mut bool) {
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.label(theme::pixel_text("DOJO — LATIHAN", 12.0, color::GOLD));
        if session.finished {
            ui.label(theme::vt_text(
                &format!(
                    "Selesai! Benar {}/{} · XP Dojo diperoleh: {}",
                    session.correct_count,
                    session.questions.len(),
                    session.xp_earned
                ),
                22.0,
                color::GREEN,
            ));
            if theme::pixel_button(ui, "Tutup Dojo", color::GREEN_DARK, color::CREAM).clicked() {
                *close = true;
            }
            return;
        }
        let Some(dq) = session.questions.get(session.index) else {
            session.finished = true;
            return;
        };
        let total = session.questions.len();
        ui.label(theme::vt_text(
            &format!(
                "Soal {}/{} · Topik: {} (bobot {})",
                session.index + 1,
                total,
                dq.question.spell_name,
                dq.weight
            ),
            19.0,
            color::MUTED,
        ));
        ui.label(theme::vt_text(&dq.question.question, 21.0, color::CREAM));
        ui.add_space(4.0);
        let selected = session.selected;
        for (i, opt) in dq.question.shuffled_options.iter().enumerate() {
            let chosen = selected == Some(i);
            let bg = if chosen {
                if opt.is_correct {
                    color::GREEN_DARK
                } else {
                    color::RED_DARK
                }
            } else {
                color::DUNGEON
            };
            let label = format!("{}. {}", (b'A' + i as u8) as char, opt.text);
            if theme::pixel_button(ui, &label, bg, color::CREAM).clicked() && selected.is_none()
            {
                session.selected = Some(i);
                let correct = opt.is_correct;
                if correct {
                    session.correct_count += 1;
                }
                // Catat jawaban + XP Dojo lewat reducer.
                app.apply(Action::RecordQuestionAnswer {
                    question_id: dq.question.id.clone(),
                    is_correct: correct,
                });
                let xp_before = app.state.xp;
                app.apply(Action::AwardDojoAnswerXp {
                    question_id: dq.question.id.clone(),
                    is_correct: correct,
                });
                let gained = (app.state.xp - xp_before).max(0);
                session.xp_earned += gained;
            }
        }
        if let Some(i) = selected {
            let opt = &dq.question.shuffled_options[i];
            ui.add_space(4.0);
            ui.label(theme::vt_text(
                if opt.is_correct {
                    "✓ Benar!"
                } else {
                    "✗ Salah — jawaban benar ditandai hijau."
                },
                21.0,
                if opt.is_correct { color::GREEN } else { color::RED },
            ));
            ui.label(theme::vt_text(&dq.question.explanation, 19.0, color::MUTED));
            ui.add_space(6.0);
            let last = session.index + 1 >= total;
            let label = if last { "Lihat Hasil" } else { "Soal Berikutnya" };
            if theme::pixel_button(ui, label, color::GREEN_DARK, color::CREAM).clicked() {
                if last {
                    session.finished = true;
                } else {
                    session.index += 1;
                    session.selected = None;
                }
            }
        }
    });
}
