//! Modal pertarungan ruangan (`BattleModal.tsx` v2, 1053 baris).
//!
//! Alur v2: baca scroll → battle (deck berbobot, hati maks 3,
//! HP monster 100, −18 per jawaban benar) → serangan → hadiah.
//! Lantai 1 wajib lulus kuis (≥4/5) sebelum menyerang; lantai lain
//! cukup menyelesaikan kuis. Echo (soal gema) hanya Lantai 1 (+5 XP).

use dungeon_core::{
    reducer::Action,
    spellbook::{
        build_battle_deck, evaluate_floor1_battle, get_monster_dialogue, get_topic_content,
        PreparedBattleQuestion,
    },
};
use egui::{Context, Ui};

use crate::{
    app::App,
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

pub const MAX_HEARTS: i64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleStep {
    Scroll,
    Battle,
    Reward,
}

pub struct BattleUi {
    pub topic_id: String,
    pub floor_id: u32,
    pub deck: Vec<PreparedBattleQuestion>,
    step: BattleStep,
    pub scroll_checked: bool,
    pub index: usize,
    pub selected: Option<usize>,
    pub hearts: i64,
    pub monster_hp: i64,
    pub topic_correct: i64,
    pub echo_correct: bool,
    pub quiz_completed: bool,
    pub fainted: bool,
    pub retry_message: Option<String>,
}

impl BattleUi {
    pub fn open(app: &mut App, topic_id: &str) -> Self {
        let deck = build_battle_deck(
            topic_id,
            &app.state.completed_rooms,
            &app.state.question_stats,
            &mut app.rng,
        );
        let floor_id = get_topic_content(topic_id)
            .map(|t| t.floor_id)
            .unwrap_or(1);
        Self {
            topic_id: topic_id.to_string(),
            floor_id,
            deck,
            step: BattleStep::Scroll,
            scroll_checked: false,
            index: 0,
            selected: None,
            hearts: MAX_HEARTS,
            monster_hp: 100,
            topic_correct: 0,
            echo_correct: false,
            quiz_completed: false,
            fainted: false,
            retry_message: None,
        }
    }

    fn is_floor1(&self) -> bool {
        self.floor_id == 1
    }

    /// Ulangi kuis (deck diacak ulang, hati pulih) — seperti v2.
    fn retry(&mut self, app: &mut App) {
        if self.is_floor1() && self.topic_correct > 0 {
            let eval = evaluate_floor1_battle(self.topic_correct, 5);
            app.apply(Action::RecordQuizAttempt {
                topic_id: self.topic_id.clone(),
                score_percent: eval.score_percent,
            });
        }
        self.deck = build_battle_deck(
            &self.topic_id,
            &app.state.completed_rooms,
            &app.state.question_stats,
            &mut app.rng,
        );
        self.index = 0;
        self.selected = None;
        self.topic_correct = 0;
        self.echo_correct = false;
        self.hearts = MAX_HEARTS;
        self.monster_hp = 100;
        self.quiz_completed = false;
        self.fainted = false;
        self.retry_message = None;
    }
}

/// Gambar dialog battle. Mengembalikan `true` bila dialog masih terbuka.
pub fn show_battle(app: &mut App, ctx: &Context, battle: &mut BattleUi) -> bool {
    let mut open = true;
    let topic_name = dungeon_data::roadmap::topic_by_id(&battle.topic_id)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| battle.topic_id.clone());
    let title = format!("⚔  {topic_name}");
    let mut close_after = false;

    egui::Window::new(theme::pixel_text(&title, 13.0, color::CREAM))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            match battle.step {
                BattleStep::Scroll => show_scroll_step(app, ui, battle),
                BattleStep::Battle => show_battle_step(app, ui, battle),
                BattleStep::Reward => {
                    show_reward_step(app, ui, battle, &mut close_after);
                }
            }
        });

    if close_after {
        open = false;
    }
    open
}

/// Langkah 1: baca scroll, centang "sudah dibaca", mulai pertarungan.
fn show_scroll_step(_app: &mut App, ui: &mut Ui, battle: &mut BattleUi) {
    let Some(tc) = get_topic_content(&battle.topic_id) else {
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
    for para in tc.topic.scroll.paragraphs.iter().take(3) {
        ui.label(theme::vt_text(para, 20.0, color::CREAM));
        ui.add_space(2.0);
    }
    ui.label(theme::vt_text(
        &format!("Contoh: {}", tc.topic.scroll.worked_example),
        19.0,
        color::CYAN,
    ));
    ui.label(theme::vt_text(
        &format!("Kesalahan umum: {}", tc.topic.scroll.common_mistake),
        19.0,
        color::RED,
    ));
    ui.add_space(8.0);
    ui.checkbox(&mut battle.scroll_checked, "Saya sudah membaca scroll ini");
    ui.add_space(6.0);
    if theme::pixel_button(ui, "⚔  Mulai Pertarungan", color::GREEN_DARK, color::CREAM)
        .clicked()
    {
        if battle.scroll_checked {
            battle.step = BattleStep::Battle;
        } else {
            battle.retry_message =
                Some("Centang dulu bahwa kamu sudah membaca scroll.".to_string());
        }
    }
    if let Some(msg) = &battle.retry_message {
        ui.label(theme::vt_text(msg, 19.0, color::RED));
    }
}

/// Langkah 2: pertarungan — soal, hati, HP monster.
fn show_battle_step(app: &mut App, ui: &mut Ui, battle: &mut BattleUi) {
    let topic = dungeon_data::roadmap::topic_by_id(&battle.topic_id);
    let floor = topic
        .and_then(|t| dungeon_data::roadmap::floor(t.floor_id));
    let monster_type = floor
        .map(|f| f.monster_type.clone())
        .unwrap_or_else(|| "slime".to_string());
    let monster_name = topic
        .map(|t| t.monster_name.clone())
        .unwrap_or_default();
    let theme_color = floor
        .map(|f| sprites::parse_hex(&f.theme_color))
        .unwrap_or(color::GREEN);

    // Kepala: monster + HP + hati pemain.
    ui.horizontal(|ui| {
        sprites::show(ui, sprites::monster_sprite(&monster_type), theme_color, 56.0);
        ui.vertical(|ui| {
            ui.label(theme::pixel_text(&monster_name, 12.0, color::CREAM));
            theme::xp_bar(ui, battle.monster_hp as f32 / 100.0, &format!("HP {}", battle.monster_hp));
            ui.horizontal(|ui| {
                for i in 0..MAX_HEARTS {
                    sprites::show_heart(ui, i < battle.hearts, 20.0);
                }
                ui.label(theme::vt_text(
                    &format!("{}/{} HP", battle.hearts, MAX_HEARTS),
                    19.0,
                    color::RED,
                ));
            });
        });
    });
    ui.label(theme::vt_text(
        &get_monster_dialogue(&battle.topic_id, ""),
        19.0,
        color::MUTED,
    ));
    ui.add_space(6.0);

    if battle.fainted {
        theme::pixel_box(ui, PixelBox::Red, |ui| {
            ui.label(theme::pixel_text("✕ PINGSAN — HATI HABIS!", 13.0, color::RED));
            ui.label(theme::vt_text(
                "Monster terlalu kuat kali ini. Pelajari lagi scroll-nya, lalu coba lagi.",
                20.0,
                color::CREAM,
            ));
            if theme::pixel_button(ui, "🔄 Coba Lagi", color::RED_DARK, color::CREAM).clicked() {
                battle.retry(app);
            }
        });
        return;
    }

    if battle.quiz_completed {
        // Kuis selesai — tampilkan hasil + tombol serang / belum lulus.
        let eval = evaluate_floor1_battle(battle.topic_correct, 5);
        let passed = if battle.is_floor1() {
            eval.passed_quiz
        } else {
            true
        };
        theme::pixel_box(ui, if passed { PixelBox::Green } else { PixelBox::Red }, |ui| {
            if battle.is_floor1() {
                ui.label(theme::vt_text(
                    &format!(
                        "{} UJIAN MANTRA {}! (SKOR TOPIK: {}/5 — {}%, MINIMAL 4/5)",
                        if passed { "✓" } else { "✗" },
                        if passed { "LULUS" } else { "BELUM LULUS" },
                        battle.topic_correct,
                        eval.score_percent
                    ),
                    21.0,
                    if passed { color::GREEN } else { color::RED },
                ));
                if battle.echo_correct {
                    ui.label(theme::vt_text(
                        "Gema Masa Lalu dijawab benar (+5 XP saat menang).",
                        19.0,
                        color::GOLD,
                    ));
                }
            } else {
                ui.label(theme::vt_text(
                    &format!(
                        "Kuis selesai — benar {}/{} soal topik.",
                        battle.topic_correct,
                        battle.deck.iter().filter(|q| !q.is_echo).count()
                    ),
                    21.0,
                    color::GREEN,
                ));
            }
            ui.add_space(6.0);
            if passed {
                if theme::pixel_button(ui, "⚔  SERANG MONSTER!", color::RED_DARK, color::CREAM)
                    .clicked()
                {
                    battle.monster_hp = 0;
                    app.apply(Action::DefeatRoom {
                        topic_id: battle.topic_id.clone(),
                        correct_quiz_answers: if battle.is_floor1() {
                            battle.topic_correct
                        } else {
                            0
                        },
                        quiz_score_percent: if battle.is_floor1() {
                            Some(eval.score_percent)
                        } else {
                            None
                        },
                        echo_correct: if battle.is_floor1() {
                            battle.echo_correct
                        } else {
                            false
                        },
                    });
                    battle.step = BattleStep::Reward;
                }
            } else if theme::pixel_button(ui, "🔄 Ulangi Kuis", color::GOLD_DARK, color::CREAM)
                .clicked()
            {
                battle.retry(app);
            }
        });
        return;
    }

    // Soal aktif.
    let Some(q) = battle.deck.get(battle.index).cloned() else {
        battle.quiz_completed = true;
        return;
    };
    let total = battle.deck.len();
    ui.label(theme::pixel_text(
        &format!("SOAL {}/{} · {}", battle.index + 1, total, q.spell_name),
        10.0,
        color::GOLD,
    ));
    if q.is_echo {
        ui.label(theme::vt_text("✦ Gema Masa Lalu (soal bonus +5 XP)", 19.0, color::GOLD));
    }
    ui.label(theme::vt_text(&q.question, 21.0, color::CREAM));
    ui.add_space(4.0);
    let selected = battle.selected;
    for (i, opt) in q.shuffled_options.iter().enumerate() {
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
        if theme::pixel_button(ui, &label, bg, color::CREAM).clicked() && selected.is_none() {
            battle.selected = Some(i);
            app.apply(Action::RecordQuestionAnswer {
                question_id: q.id.clone(),
                is_correct: opt.is_correct,
            });
            if opt.is_correct {
                if q.is_echo {
                    battle.echo_correct = true;
                } else {
                    battle.topic_correct += 1;
                }
                battle.monster_hp = (battle.monster_hp - 18).max(15);
            } else {
                battle.hearts = (battle.hearts - 1).max(0);
                if battle.hearts == 0 {
                    battle.fainted = true;
                }
            }
        }
    }
    if let Some(i) = selected {
        let opt = &q.shuffled_options[i];
        ui.add_space(4.0);
        ui.label(theme::vt_text(
            if opt.is_correct { "✓ Benar!" } else { "✗ Salah." },
            21.0,
            if opt.is_correct { color::GREEN } else { color::RED },
        ));
        ui.label(theme::vt_text(&q.explanation, 19.0, color::MUTED));
        ui.add_space(6.0);
        let last = battle.index + 1 >= total;
        let label = if last { "Selesai Kuis" } else { "Soal Berikutnya" };
        if theme::pixel_button(ui, label, color::GREEN_DARK, color::CREAM).clicked() {
            if last {
                battle.quiz_completed = true;
                let eval = evaluate_floor1_battle(battle.topic_correct, 5);
                app.apply(Action::RecordQuizAttempt {
                    topic_id: battle.topic_id.clone(),
                    score_percent: eval.score_percent,
                });
            } else {
                battle.index += 1;
                battle.selected = None;
            }
        }
    }
    if let Some(msg) = &battle.retry_message {
        ui.label(theme::vt_text(msg, 19.0, color::RED));
    }
}

/// Langkah 3: hadiah kemenangan (dari `Effect::RoomDefeated`).
fn show_reward_step(app: &mut App, ui: &mut Ui, battle: &mut BattleUi, close: &mut bool) {
    let _ = battle;
    if let Some(reward) = app.room_reward.clone() {
        theme::pixel_box(ui, PixelBox::Gold, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(theme::pixel_text("MONSTER DIKALAHKAN!", 15.0, color::GOLD));
                ui.label(theme::vt_text(
                    &format!("+{} XP", reward.total_xp),
                    26.0,
                    color::GREEN,
                ));
                ui.label(theme::vt_text(
                    &format!("(Basis: +{} XP", reward.base_xp),
                    19.0,
                    color::CREAM,
                ));
                if reward.bonus_xp > 0 {
                    ui.label(theme::vt_text(
                        &format!("Bonus Kritikal +10%: +{} XP", reward.bonus_xp),
                        19.0,
                        color::GOLD,
                    ));
                }
                if reward.echo_xp > 0 {
                    ui.label(theme::vt_text(
                        &format!("Bonus Gema Masa Lalu: +{} XP", reward.echo_xp),
                        19.0,
                        color::GOLD,
                    ));
                }
                if let Some(spell_id) = &reward.unlocked_spell_id {
                    let name = find_spell_name(spell_id).unwrap_or_else(|| spell_id.clone());
                    ui.label(theme::vt_text(
                        &format!("Mantra baru dibuka: {name}"),
                        20.0,
                        color::PURPLE,
                    ));
                }
            });
        });
    } else {
        ui.label(theme::vt_text("Ruangan selesai.", 21.0, color::GREEN));
    }
    ui.add_space(8.0);
    if theme::pixel_button(ui, "Lanjutkan", color::GREEN_DARK, color::CREAM).clicked() {
        app.room_reward = None;
        *close = true;
    }
}

fn find_spell_name(spell_id: &str) -> Option<String> {
    for floor_id in 1..=5u32 {
        if let Some(fc) = dungeon_data::content::floor_content(floor_id) {
            for topic in &fc.topics {
                if topic.spell.id == spell_id {
                    return Some(topic.spell.name.clone());
                }
            }
        }
    }
    None
}
