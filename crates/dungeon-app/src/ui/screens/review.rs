//! Ulasan Harian (`ReviewPage.tsx` v2, 393 baris).
//!
//! Antrean kartu yang jatuh tempo (Leitner box 1/3/7/14/30 hari),
//! tombol penilaian Lupa / Ragu / Ingat, distribusi kotak.

use dungeon_core::{
    lesson_engine::{get_due_review_items, ReviewRating},
    reducer::Action,
};
use egui::Ui;

use crate::{
    app::{today_key, App},
    ui::theme::{self, color, PixelBox},
};

#[derive(Default)]
pub struct ReviewUi {
    pub started: bool,
    pub index: usize,
    pub revealed: bool,
    pub graded_count: i64,
}

pub fn show_review(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("ULASAN HARIAN", 18.0, color::GREEN));
    ui.label(theme::vt_text(
        "Kartu review terjadwal — nilai kejujuranmu: Lupa / Ragu / Ingat.",
        20.0,
        color::MUTED,
    ));
    ui.add_space(8.0);

    let today = today_key();
    let due = get_due_review_items(&app.state.review_queue, &today, 10);

    // Distribusi kotak (Leitner).
    let mut boxes = [0usize; 6];
    for entry in app.state.review_queue.values() {
        let b = entry.box_ as usize;
        if (1..=5).contains(&b) {
            boxes[b] += 1;
        }
    }
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("DISTRIBUSI KOTAK", 11.0, color::GOLD));
        for b in 1..=5u8 {
            let interval = match b {
                1 => "1 hari",
                2 => "3 hari",
                3 => "7 hari",
                4 => "14 hari",
                _ => "30 hari",
            };
            ui.label(theme::vt_text(
                &format!("Kotak {b} ({interval}): {} kartu", boxes[b as usize]),
                20.0,
                color::CREAM,
            ));
        }
        ui.label(theme::vt_text(
            &format!("Total kartu dalam antrean: {}", app.state.review_queue.len()),
            19.0,
            color::MUTED,
        ));
    });
    ui.add_space(8.0);

    if !app.review.started {
        theme::pixel_box(ui, PixelBox::Gold, |ui| {
            ui.label(theme::vt_text(
                &format!(
                    "Kartu jatuh tempo hari ini: {} (maks 10 per sesi).",
                    due.total_due_count
                ),
                22.0,
                color::CREAM,
            ));
            if due.session_items.is_empty() {
                ui.label(theme::vt_text(
                    "Tidak ada kartu jatuh tempo. Selesaikan chunk pelajaran untuk menambah kartu review.",
                    20.0,
                    color::MUTED,
                ));
            } else if theme::pixel_button(ui, "▶  Mulai Sesi Ulasan", color::GREEN_DARK, color::CREAM)
                .clicked()
            {
                app.review.started = true;
                app.review.index = 0;
                app.review.revealed = false;
                app.review.graded_count = 0;
            }
        });
        return;
    }

    let session = due.session_items;
    if app.review.index >= session.len() {
        theme::pixel_box(ui, PixelBox::Green, |ui| {
            ui.label(theme::vt_text(
                &format!(
                    "Sesi selesai! {} kartu dinilai hari ini.",
                    app.review.graded_count
                ),
                22.0,
                color::GREEN,
            ));
            if theme::pixel_button(ui, "Selesai", color::GREEN_DARK, color::CREAM).clicked() {
                app.review.started = false;
            }
        });
        return;
    }

    let item = &session[app.review.index];
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text(
            &format!(
                "KARTU {}/{} · Kotak {} · {}",
                app.review.index + 1,
                session.len(),
                item.box_,
                item.chunk_title
            ),
            10.0,
            color::GOLD,
        ));
        ui.add_space(4.0);
        ui.label(theme::vt_text(item.front, 24.0, color::CREAM));
        if app.review.revealed {
            ui.add_space(6.0);
            ui.label(theme::vt_text(item.back, 22.0, color::GREEN));
            ui.add_space(8.0);
            ui.label(theme::pixel_text("SEBERAPA INGAT?", 10.0, color::GOLD));
            ui.horizontal_wrapped(|ui| {
                let ratings = [
                    ("😕 Lupa", ReviewRating::Lupa, color::RED_DARK),
                    ("🤔 Ragu", ReviewRating::Ragu, color::GOLD_DARK),
                    ("😊 Ingat", ReviewRating::Ingat, color::GREEN_DARK),
                ];
                for (label, rating, bg) in ratings {
                    if theme::pixel_button(ui, label, bg, color::CREAM).clicked() {
                        app.apply(Action::GradeReviewItem {
                            item_id: item.id.to_string(),
                            rating,
                        });
                        app.review.graded_count += 1;
                        app.review.index += 1;
                        app.review.revealed = false;
                    }
                }
            });
        } else if theme::pixel_button(ui, "👁  Lihat Jawaban", color::DUNGEON, color::CYAN)
            .clicked()
        {
            app.review.revealed = true;
        }
    });
}
