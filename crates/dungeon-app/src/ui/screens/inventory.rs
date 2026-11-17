//! Inventori Relik (`Inventory.tsx` v2, 233 baris).
//!
//! Relik (dari boss + quest Guild Hall Lantai 5), lencana (badge),
//! dan mantra yang sudah dibuka.

use egui::Ui;

use crate::{
    app::App,
    ui::{
        sprites,
        theme::{self, color, PixelBox},
    },
};

pub fn show_inventory(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("INVENTORI RELIK", 18.0, color::GREEN));
    ui.label(theme::vt_text(
        "Harta yang kau kumpulkan sepanjang petualangan.",
        20.0,
        color::MUTED,
    ));
    ui.add_space(8.0);

    // Relik.
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.label(theme::pixel_text("RELIK", 12.0, color::GOLD));
        if app.state.relics.is_empty() {
            ui.label(theme::vt_text(
                "Belum ada relik — kalahkan boss atau selesaikan quest Guild Hall Lantai 5.",
                20.0,
                color::MUTED,
            ));
        }
        for relic_id in &app.state.relics {
            let info = find_relic(relic_id);
            ui.horizontal(|ui| {
                sprites::show(ui, &sprites::RELIC, color::GOLD, 32.0);
                ui.vertical(|ui| {
                    ui.label(theme::vt_text(
                        &info.clone().unwrap_or_else(|| relic_id.clone()),
                        21.0,
                        color::CREAM,
                    ));
                    if let Some(desc) = find_relic_desc(relic_id) {
                        ui.label(theme::vt_text(&desc, 19.0, color::MUTED));
                    }
                });
            });
        }
    });
    ui.add_space(8.0);

    // Lencana.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("LENCANA", 12.0, color::GOLD));
        if app.state.badges.is_empty() {
            ui.label(theme::vt_text(
                "Belum ada lencana — kalahkan boss tiap lantai.",
                20.0,
                color::MUTED,
            ));
        }
        for badge_id in &app.state.badges {
            let name = find_badge(badge_id).unwrap_or_else(|| badge_id.clone());
            ui.horizontal(|ui| {
                sprites::show(ui, &sprites::STAR, color::GOLD, 32.0);
                ui.label(theme::vt_text(&name, 21.0, color::CREAM));
            });
        }
    });
    ui.add_space(8.0);

    // Mantra yang dibuka.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text(
            &format!("MANTRA DIBUKA ({}/28)", app.state.spells_unlocked.len()),
            12.0,
            color::GOLD,
        ));
        if app.state.spells_unlocked.is_empty() {
            ui.label(theme::vt_text(
                "Belum ada mantra — kalahkan monster ruangan untuk membuka.",
                20.0,
                color::MUTED,
            ));
        }
        for spell_id in &app.state.spells_unlocked {
            let name = find_spell(spell_id).unwrap_or_else(|| spell_id.clone());
            ui.horizontal(|ui| {
                sprites::show(ui, &sprites::BOOK, color::PURPLE, 28.0);
                ui.label(theme::vt_text(&name, 20.0, color::CREAM));
            });
        }
    });
}

/// Cari nama + deskripsi relik berdasarkan id di roadmap (boss + topik).
fn find_relic(relic_id: &str) -> Option<String> {
    for floor in dungeon_data::roadmap::floors() {
        if floor.boss.relic.id == relic_id {
            return Some(format!("{} — {}", floor.boss.relic.name, floor.boss.relic.source));
        }
        for topic in &floor.topics {
            if let Some(r) = &topic.relic {
                if r.id == relic_id {
                    return Some(format!("{} — {}", r.name, r.source));
                }
            }
        }
    }
    None
}

fn find_relic_desc(relic_id: &str) -> Option<String> {
    for floor in dungeon_data::roadmap::floors() {
        if floor.boss.relic.id == relic_id {
            return Some(floor.boss.relic.description.clone());
        }
        for topic in &floor.topics {
            if let Some(r) = &topic.relic {
                if r.id == relic_id {
                    return Some(r.description.clone());
                }
            }
        }
    }
    None
}

fn find_badge(badge_id: &str) -> Option<String> {
    for floor in dungeon_data::roadmap::floors() {
        if floor.boss.badge.id == badge_id {
            return Some(floor.boss.badge.name.clone());
        }
    }
    None
}

fn find_spell(spell_id: &str) -> Option<String> {
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
