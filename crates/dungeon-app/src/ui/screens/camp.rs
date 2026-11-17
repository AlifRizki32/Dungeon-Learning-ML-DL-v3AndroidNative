//! Camp Api Unggun (`CampPage.tsx` v2, 888 baris).
//!
//! Panggung api unggun (tahap menurut jam mingguan vs target),
//! target mingguan, form catat sesi belajar, pomodoro, daftar sesi,
//! proyeksi penyelesaian. Ekspor/impor save = Fase 4 (penyimpanan),
//! jadi di sini hanya ditampilkan sebagai catatan.

use std::time::{Duration, Instant};

use dungeon_core::{
    camp::{
        calculate_projected_completion, get_campfire_stage, get_last8_weeks_buckets,
        get_week_start_date_key, StudySessionSource,
    },
    reducer::Action,
};
use egui::Ui;

use crate::{
    app::{today_key, App},
    ui::theme::{self, color, PixelBox},
};

pub struct CampUi {
    pub duration_text: String,
    pub topic_id: String,
    pub note_text: String,
    pub show_form: bool,
    /// Pomodoro: sisa waktu + kapan terakhir dihitung.
    pub pomodoro_left: Option<Duration>,
    pub pomodoro_running: bool,
    pub pomodoro_last_tick: Option<Instant>,
}

impl Default for CampUi {
    fn default() -> Self {
        Self {
            duration_text: "25".to_string(),
            topic_id: String::new(),
            note_text: String::new(),
            show_form: false,
            pomodoro_left: None,
            pomodoro_running: false,
            pomodoro_last_tick: None,
        }
    }
}

pub fn show_camp(app: &mut App, ui: &mut Ui) {
    ui.label(theme::pixel_text("CAMP API UNGGUN", 18.0, color::GREEN));
    ui.label(theme::vt_text(
        "Tempat istirahat petualang — catat sesi belajarmu, jaga api tetap menyala.",
        20.0,
        color::MUTED,
    ));
    ui.add_space(8.0);

    let today = today_key();
    let week_start = get_week_start_date_key(&today, &today);
    let week_minutes: f64 = app
        .state
        .study_sessions
        .iter()
        .filter(|s| s.date.as_str() >= week_start.as_str())
        .map(|s| s.duration_minutes)
        .sum();
    let week_hours = week_minutes / 60.0;
    let stage = get_campfire_stage(week_hours, app.state.weekly_target_hours);

    // Panggung api unggun.
    theme::pixel_box(ui, PixelBox::Gold, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(theme::pixel_text(
                &format!("🔥 {}", stage.name),
                14.0,
                color::GOLD,
            ));
            ui.label(theme::vt_text(stage.description, 20.0, color::CREAM));
            ui.label(theme::vt_text(
                &format!(
                    "Minggu ini: {:.1} jam / target {:.1} jam ({}%)",
                    week_hours, app.state.weekly_target_hours, stage.percent
                ),
                20.0,
                color::GREEN,
            ));
            theme::xp_bar(ui, stage.percent as f32 / 100.0, &format!("{}%", stage.percent));
        });
    });
    ui.add_space(8.0);

    // Target mingguan.
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("TARGET MINGGUAN", 11.0, color::GOLD));
        ui.horizontal_wrapped(|ui| {
            for hours in [3.0, 6.0, 9.0, 12.0] {
                let active = (app.state.weekly_target_hours - hours).abs() < 0.01;
                if theme::pixel_chip(ui, &format!("{hours} jam"), active).clicked() {
                    app.apply(Action::SetWeeklyTargetHours { hours });
                }
            }
        });
        ui.label(theme::vt_text(
            "Bonus mingguan 50 XP otomatis diklaim saat target tercapai.",
            19.0,
            color::MUTED,
        ));
    });
    ui.add_space(8.0);

    // Pomodoro.
    show_pomodoro(app, ui);
    ui.add_space(8.0);

    // Form catat sesi.
    if !app.camp.show_form {
        if theme::pixel_button(ui, "＋ Catat Sesi Belajar", color::GREEN_DARK, color::CREAM)
            .clicked()
        {
            app.camp.show_form = true;
        }
    } else {
        theme::pixel_box(ui, PixelBox::Normal, |ui| {
            ui.label(theme::pixel_text("CATAT SESI", 11.0, color::GOLD));
            ui.label(theme::vt_text("Durasi (menit):", 20.0, color::CREAM));
            ui.add(
                egui::TextEdit::singleline(&mut app.camp.duration_text)
                    .font(egui::FontId::new(
                        20.0,
                        egui::FontFamily::Name(theme::FONT_VT.into()),
                    ))
                    .desired_width(80.0),
            );
            ui.label(theme::vt_text("Topik:", 20.0, color::CREAM));
            egui::ComboBox::new("camp_topic", "Topik")
                .selected_text(if app.camp.topic_id.is_empty() {
                    "Pilih topik…".to_string()
                } else {
                    dungeon_data::roadmap::topic_by_id(&app.camp.topic_id)
                        .map(|t| format!("No.{} — {}", t.number, t.name))
                        .unwrap_or_default()
                })
                .show_ui(ui, |ui| {
                    for topic in dungeon_data::roadmap::all_topics() {
                        ui.selectable_value(
                            &mut app.camp.topic_id,
                            topic.id.clone(),
                            format!("No.{} — {}", topic.number, topic.name),
                        );
                    }
                });
            ui.label(theme::vt_text("Catatan (opsional):", 20.0, color::CREAM));
            ui.add(
                egui::TextEdit::singleline(&mut app.camp.note_text)
                    .font(egui::FontId::new(
                        20.0,
                        egui::FontFamily::Name(theme::FONT_VT.into()),
                    ))
                    .desired_width(ui.available_width()),
            );
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if theme::pixel_button(ui, "Simpan Sesi", color::GREEN_DARK, color::CREAM)
                    .clicked()
                {
                    let minutes: i64 = app.camp.duration_text.trim().parse().unwrap_or(0);
                    if minutes <= 0 {
                        app.toast = Some("Durasi harus lebih dari 0 menit.".to_string());
                    } else if app.camp.topic_id.is_empty() {
                        app.toast = Some("Pilih topik dulu.".to_string());
                    } else {
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as i64)
                            .unwrap_or(0);
                        let note = if app.camp.note_text.trim().is_empty() {
                            None
                        } else {
                            Some(app.camp.note_text.trim().to_string())
                        };
                        app.apply(Action::LogStudySession {
                            session_id: format!("s-{now_ms}"),
                            created_at_ms: now_ms,
                            topic_id: app.camp.topic_id.clone(),
                            duration_minutes: minutes,
                            date: today.clone(),
                            note,
                            source: StudySessionSource::Manual,
                        });
                        app.camp.show_form = false;
                        app.camp.note_text.clear();
                        app.toast = Some("Sesi belajar tercatat.".to_string());
                    }
                }
                if theme::pixel_button(ui, "Batal", color::DUNGEON, color::CREAM).clicked() {
                    app.camp.show_form = false;
                }
            });
        });
        ui.add_space(8.0);
    }

    // 8 minggu terakhir.
    let buckets = get_last8_weeks_buckets(&app.state.study_sessions, &today);
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("8 MINGGU TERAKHIR", 11.0, color::GOLD));
        for b in &buckets {
            ui.label(theme::vt_text(
                &format!(
                    "{}: {:.1} jam",
                    b.date_range_label, b.total_hours
                ),
                19.0,
                color::CREAM,
            ));
        }
    });
    ui.add_space(8.0);

    // Proyeksi penyelesaian.
    let proj = calculate_projected_completion(
        &app.state.completed_rooms,
        &app.state.study_sessions,
        app.state.weekly_target_hours,
        &today,
    );
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("PROYEKSI", 11.0, color::GOLD));
        ui.label(theme::vt_text(
            &format!(
                "Sisa roadmap: {:.0} minggu (~{:.0} jam). Perkiraan selesai dalam {} minggu ({}) dengan rata-rata {:.1} jam/minggu{}.",
                proj.remaining_roadmap_weeks,
                proj.remaining_estimated_hours,
                proj.estimated_weeks_to_finish,
                proj.projected_month_year,
                proj.effective_hours_per_week,
                if proj.using_actual_average {
                    " — memakai rata-rata aktualmu"
                } else {
                    " — memakai target mingguan"
                }
            ),
            20.0,
            color::CREAM,
        ));
    });
    ui.add_space(8.0);

    // Daftar sesi (terbaru dulu, maks 20).
    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("RIWAYAT SESI", 11.0, color::GOLD));
        if app.state.study_sessions.is_empty() {
            ui.label(theme::vt_text("Belum ada sesi tercatat.", 20.0, color::MUTED));
        }
        let mut sessions: Vec<_> = app.state.study_sessions.iter().cloned().collect();
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        for s in sessions.iter().take(20) {
            ui.horizontal(|ui| {
                let topic_name = dungeon_data::roadmap::topic_by_id(&s.topic_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| s.topic_id.clone());
                ui.label(theme::vt_text(
                    &format!(
                        "{} · {:.0} mnt · {} · {}",
                        s.date,
                        s.duration_minutes,
                        topic_name,
                        match s.source {
                            StudySessionSource::Manual => "manual",
                            StudySessionSource::Pomodoro => "pomodoro",
                        }
                    ),
                    19.0,
                    color::CREAM,
                ));
                let id = s.id.clone();
                if ui
                    .small_button("Hapus")
                    .clicked()
                {
                    app.apply(Action::DeleteStudySession { session_id: id });
                }
            });
            if let Some(note) = &s.note {
                ui.label(theme::vt_text(&format!("  “{note}”"), 18.0, color::MUTED));
            }
        }
    });
    ui.add_space(8.0);
    ui.label(theme::vt_text(
        "Ekspor/impor save (JSON) hadir di Fase 4 — penyimpanan Android.",
        18.0,
        color::MUTED,
    ));
}

/// Timer pomodoro sederhana (25 menit bawaan): hitung mundur per frame.
fn show_pomodoro(app: &mut App, ui: &mut Ui) {
    // Majukan timer bila berjalan.
    if app.camp.pomodoro_running {
        if let Some(last) = app.camp.pomodoro_last_tick {
            let elapsed = last.elapsed();
            if let Some(left) = app.camp.pomodoro_left {
                if elapsed >= left {
                    app.camp.pomodoro_left = Some(Duration::ZERO);
                    app.camp.pomodoro_running = false;
                    app.toast = Some("Pomodoro selesai! Catat sesinya di atas.".to_string());
                } else {
                    app.camp.pomodoro_left = Some(left - elapsed);
                }
            }
        }
        app.camp.pomodoro_last_tick = Some(Instant::now());
        ui.ctx().request_repaint();
    }

    theme::pixel_box(ui, PixelBox::Normal, |ui| {
        ui.label(theme::pixel_text("POMODORO", 11.0, color::GOLD));
        let label = match app.camp.pomodoro_left {
            Some(left) => {
                let secs = left.as_secs();
                format!("{:02}:{:02}", secs / 60, secs % 60)
            }
            None => "25:00".to_string(),
        };
        ui.label(theme::pixel_text(&label, 22.0, color::CREAM));
        ui.horizontal_wrapped(|ui| {
            if !app.camp.pomodoro_running {
                if theme::pixel_button(ui, "▶  Mulai 25 menit", color::GREEN_DARK, color::CREAM)
                    .clicked()
                {
                    app.camp.pomodoro_left = Some(Duration::from_secs(25 * 60));
                    app.camp.pomodoro_running = true;
                    app.camp.pomodoro_last_tick = Some(Instant::now());
                }
            } else {
                if theme::pixel_button(ui, "⏸  Jeda", color::GOLD_DARK, color::CREAM).clicked() {
                    app.camp.pomodoro_running = false;
                }
                if theme::pixel_button(ui, "⏹  Berhenti", color::DUNGEON, color::RED).clicked() {
                    app.camp.pomodoro_running = false;
                    app.camp.pomodoro_left = None;
                }
            }
        });
    });
}
