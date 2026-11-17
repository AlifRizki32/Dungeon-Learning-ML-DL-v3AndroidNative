//! Modal pertarungan boss (`BossModal.tsx` v2, 1687 baris).
//!
//! Multi-fase (satu fase = satu varian soal A/B), hati maks 3,
//! ramuan 2 (lantai 3 & 5: 3 — dari core), pingsan lalu pulih
//! (fase terakhir yang sudah lolos ikut hilang — `recover_boss_after_faint`).
//! Semua fase lolos → kalahkan boss (+300 XP, lencana, relik).

use dungeon_core::{
    reducer::Action,
    state::create_default_boss_progress_for_floor,
};
use egui::{Context, Ui};

use crate::{
    app::App,
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

pub struct BossUi {
    pub floor_id: u32,
    pub hearts: i64,
    pub selected: Option<usize>,
    pub fainted: bool,
    pub defeated: bool,
}

impl BossUi {
    pub fn open(_app: &mut App, floor_id: u32) -> Self {
        Self {
            floor_id,
            hearts: 3,
            selected: None,
            fainted: false,
            defeated: false,
        }
    }
}

/// Gambar dialog boss. Mengembalikan `true` bila dialog masih terbuka.
pub fn show_boss(app: &mut App, ctx: &Context, boss: &mut BossUi) -> bool {
    let mut open = true;
    let floor = dungeon_data::roadmap::floor(boss.floor_id);
    let boss_name = floor
        .map(|f| f.boss.name.clone())
        .unwrap_or_else(|| "Boss".to_string());
    let title = format!("👑 {boss_name}");
    let mut close_after = false;

    egui::Window::new(theme::pixel_text(&title, 13.0, color::CREAM))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            if boss.defeated {
                show_boss_reward(app, ui, boss, &mut close_after);
            } else {
                show_boss_fight(app, ui, boss);
            }
        });

    if close_after {
        open = false;
    }
    open
}

fn show_boss_fight(app: &mut App, ui: &mut Ui, boss: &mut BossUi) {
    let floor_id = boss.floor_id;
    let Some(floor) = dungeon_data::roadmap::floor(floor_id) else {
        ui.label(theme::vt_text("Lantai tidak dikenal.", 21.0, color::RED));
        return;
    };
    let Some(content) = dungeon_data::content::floor_content(floor_id) else {
        ui.label(theme::vt_text("Konten boss tidak ditemukan.", 21.0, color::RED));
        return;
    };
    let progress = app
        .state
        .boss_progress
        .get(&floor_id)
        .cloned()
        .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
    let total_phases = content.boss.phases.len();
    let cleared = progress.cleared_phases.len();

    // Kepala: sprite boss + hati + ramuan.
    ui.horizontal(|ui| {
        sprites::show(
            ui,
            sprites::monster_sprite(&floor.boss.sprite_type),
            color::RED,
            64.0,
        );
        ui.vertical(|ui| {
            ui.label(theme::pixel_text(&floor.boss.name, 13.0, color::CREAM));
            ui.label(theme::vt_text(&floor.boss.title, 20.0, color::GOLD));
            ui.horizontal(|ui| {
                for i in 0..3i64 {
                    sprites::show_heart(ui, i < boss.hearts, 20.0);
                }
                ui.label(theme::vt_text(
                    &format!("{}/3 HP", boss.hearts),
                    19.0,
                    color::RED,
                ));
            });
            ui.label(theme::vt_text(
                &format!("Fase: {cleared}/{total_phases} · Ramuan: {}", progress.potions),
                19.0,
                color::MUTED,
            ));
        });
    });
    ui.add_space(4.0);
    theme::xp_bar(ui, cleared as f32 / total_phases.max(1) as f32, &format!("{cleared}/{total_phases} fase"));
    ui.add_space(6.0);

    if boss.fainted {
        theme::pixel_box(ui, PixelBox::Red, |ui| {
            ui.label(theme::pixel_text("✕ PINGSAN!", 13.0, color::RED));
            ui.label(theme::vt_text(
                "Kamu pingsan di hadapan boss. Fase terakhir yang sudah lolos ikut hilang (aturan v2). Pulihkan dirimu dan coba lagi.",
                20.0,
                color::CREAM,
            ));
            if theme::pixel_button(ui, "💪 Pulih & Coba Lagi", color::RED_DARK, color::CREAM)
                .clicked()
            {
                app.apply(Action::FaintBossFight { floor_id });
                boss.hearts = 3;
                boss.fainted = false;
                boss.selected = None;
            }
        });
        return;
    }

    // Fase aktif = fase pertama yang belum lolos.
    let Some(phase) = content
        .boss
        .phases
        .iter()
        .find(|p| !progress.cleared_phases.contains(&p.phase))
    else {
        // Semua fase lolos → kalahkan boss.
        if theme::pixel_button(ui, "⚔  Kalahkan Boss!", color::RED_DARK, color::CREAM).clicked() {
            app.apply(Action::DefeatBoss { floor_id });
            boss.defeated = true;
        }
        return;
    };

    let variant_idx = progress
        .last_variant_by_phase
        .get(&phase.phase)
        .copied()
        .unwrap_or(0) as usize;
    let variant = phase.variants.get(variant_idx).or_else(|| phase.variants.first());
    let Some(variant) = variant else {
        ui.label(theme::vt_text("Varian fase tidak ditemukan.", 21.0, color::RED));
        return;
    };

    let phase_title = phase
        .title
        .clone()
        .unwrap_or_else(|| format!("Fase {}", phase.phase));
    ui.label(theme::pixel_text(
        &format!("FASE {} — {}", phase.phase, phase_title.to_uppercase()),
        11.0,
        color::GOLD,
    ));
    if let Some(spell_name) = find_spell_name(variant.spell_id.as_deref().unwrap_or(&phase.spell_id)) {
        ui.label(theme::vt_text(&format!("Mantra: {spell_name}"), 20.0, color::PURPLE));
    }
    ui.label(theme::vt_text(&variant.scenario, 21.0, color::CREAM));
    ui.add_space(4.0);

    let selected = boss.selected;
    for (i, opt) in variant.options.iter().enumerate() {
        let chosen = selected == Some(i);
        let bg = if chosen {
            if i == variant.correct_index {
                color::GREEN_DARK
            } else {
                color::RED_DARK
            }
        } else {
            color::DUNGEON
        };
        let label = format!("{}. {}", (b'A' + i as u8) as char, opt);
        if theme::pixel_button(ui, &label, bg, color::CREAM).clicked() && selected.is_none() {
            boss.selected = Some(i);
            if i == variant.correct_index {
                app.apply(Action::CompleteBossPhase {
                    floor_id,
                    phase: phase.phase,
                });
                boss.selected = None;
            } else {
                boss.hearts = (boss.hearts - 1).max(0);
                app.apply(Action::AdvanceBossPhaseVariant {
                    floor_id,
                    phase: phase.phase,
                });
                if boss.hearts == 0 {
                    boss.fainted = true;
                }
            }
        }
    }
    if let Some(i) = selected {
        let correct = i == variant.correct_index;
        ui.add_space(4.0);
        ui.label(theme::vt_text(
            if correct { "✓ Benar! Fase lolos." } else { "✗ Salah — varian soal berganti." },
            21.0,
            if correct { color::GREEN } else { color::RED },
        ));
        ui.label(theme::vt_text(&variant.explanation, 19.0, color::MUTED));
        if !correct && boss.hearts > 0 {
            ui.add_space(4.0);
            if theme::pixel_button(ui, "Soal Berikutnya (varian lain)", color::GOLD_DARK, color::CREAM)
                .clicked()
            {
                boss.selected = None;
            }
        }
    }

    // Ramuan: pulihkan 1 hati bila masih ada.
    ui.add_space(6.0);
    if progress.potions > 0 && boss.hearts < 3 {
        if theme::pixel_button(
            ui,
            &format!("🧪 Gunakan Ramuan (sisa {})", progress.potions),
            color::GREEN_DARK,
            color::CREAM,
        )
        .clicked()
        {
            app.apply(Action::UseBossPotion { floor_id });
            boss.hearts = (boss.hearts + 1).min(3);
        }
    } else if progress.potions == 0 {
        ui.label(theme::vt_text("Ramuan habis.", 18.0, color::MUTED));
    }
}

/// Layar hadiah boss (+300 XP, lencana, relik).
fn show_boss_reward(app: &mut App, ui: &mut Ui, _boss: &mut BossUi, close: &mut bool) {
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(theme::pixel_text("BOSS DIKALAHKAN!", 15.0, color::GOLD));
            if let Some(xp) = app.boss_reward {
                ui.label(theme::vt_text(&format!("+{xp} XP"), 26.0, color::GREEN));
            }
            ui.label(theme::vt_text(
                "Lencana dan relik boss ditambahkan ke Inventori.",
                20.0,
                color::CREAM,
            ));
        });
    });
    ui.add_space(8.0);
    if theme::pixel_button(ui, "Lanjutkan", color::GREEN_DARK, color::CREAM).clicked() {
        app.boss_reward = None;
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
