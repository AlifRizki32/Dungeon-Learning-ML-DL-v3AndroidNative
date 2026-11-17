//! Camp tracker — port `campTracker.ts` v2.
//!
//! Meliputi: ember minggu (8 minggu kalender), tahap api unggun, proyeksi
//! penyelesaian roadmap, validasi node lantai, dan validator save JSON
//! (maks 1 MB, 41 field disanitasi per-field).

use crate::dates;
use dungeon_data::roadmap::{all_topics, floor};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

// ============================================================================
// KONSTANTA
// ============================================================================

/// Ukuran maksimal file save: 1 MB.
pub const MAX_SAVE_FILE_BYTES: usize = 1024 * 1024;
/// Bonus XP mingguan (sekali per minggu bila target tercapai).
pub const WEEKLY_BONUS_XP: i64 = 50;
/// Target jam belajar mingguan bawaan.
pub const DEFAULT_WEEKLY_TARGET_HOURS: f64 = 9.0;
/// Jam per minggu roadmap (titik tengah asumsi 8-10 jam/minggu).
pub const BASELINE_HOURS_PER_ROADMAP_WEEK: f64 = 9.0;

// ============================================================================
// TIPE
// ============================================================================

/// Sumber sesi belajar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudySessionSource {
    Manual,
    Pomodoro,
}

/// Satu sesi belajar tercatat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudySession {
    pub id: String,
    #[serde(rename = "topicId")]
    pub topic_id: String,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: f64,
    /// Tanggal lokal `YYYY-MM-DD`.
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub source: StudySessionSource,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

/// Satu ember minggu kalender (Senin–Minggu).
#[derive(Debug, Clone, PartialEq)]
pub struct WeekBucket {
    /// Kunci Senin minggu ini (`YYYY-MM-DD`).
    pub week_key: String,
    pub short_label: String,
    pub date_range_label: String,
    pub total_minutes: i64,
    pub total_hours: f64,
    pub session_count: usize,
}

/// Tahap visual api unggun.
#[derive(Debug, Clone, PartialEq)]
pub struct CampfireStageInfo {
    pub stage: u8,
    pub percent: i64,
    pub name: &'static str,
    pub description: &'static str,
}

/// Proyeksi penyelesaian roadmap.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedCompletionInfo {
    pub remaining_roadmap_weeks: f64,
    pub remaining_estimated_hours: f64,
    pub actual_avg_hours_per_week: f64,
    pub effective_hours_per_week: f64,
    pub using_actual_average: bool,
    pub estimated_weeks_to_finish: i64,
    pub projected_month_year: String,
}

// ============================================================================
// TANGGAL & EMBER MINGGU
// ============================================================================

/// Kunci Senin (`YYYY-MM-DD`) minggu kalender yang memuat tanggal ini.
/// Kunci tanggal tidak valid jatuh kembali ke `today_key` (mirip v2).
pub fn get_week_start_date_key(date_key: &str, today_key: &str) -> String {
    dates::monday_of_week(date_key).unwrap_or_else(|| today_key.to_string())
}

fn format_week_date_range(monday_key: &str) -> String {
    let Some((_, m, d)) = dates::parse_date_key(monday_key) else {
        return monday_key.to_string();
    };
    let sunday_key = dates::add_days(monday_key, 6, monday_key);
    let Some((_, sm, sd)) = dates::parse_date_key(&sunday_key) else {
        return monday_key.to_string();
    };
    let m_month = dates::MONTHS_SHORT_ID[(m - 1) as usize];
    let s_month = dates::MONTHS_SHORT_ID[(sm - 1) as usize];
    if m == sm {
        format!("{d}-{sd} {m_month}")
    } else {
        format!("{d} {m_month}-{sd} {s_month}")
    }
}

/// 8 ember minggu kalender yang berakhir di minggu `reference_date_key`
/// (terlama dulu, terbaru terakhir).
pub fn get_last8_weeks_buckets(
    sessions: &[StudySession],
    reference_date_key: &str,
) -> Vec<WeekBucket> {
    let current_monday = get_week_start_date_key(reference_date_key, reference_date_key);
    let mut minutes_by_week: BTreeMap<String, f64> = BTreeMap::new();
    let mut count_by_week: BTreeMap<String, usize> = BTreeMap::new();

    for s in sessions {
        if !(s.duration_minutes > 0.0) || !s.duration_minutes.is_finite() {
            continue;
        }
        let wk = get_week_start_date_key(&s.date, &current_monday);
        *minutes_by_week.entry(wk.clone()).or_insert(0.0) += s.duration_minutes;
        *count_by_week.entry(wk).or_insert(0) += 1;
    }

    let mut buckets: Vec<WeekBucket> = Vec::new();
    for offset in (0..=7).rev() {
        let wk = dates::add_days(&current_monday, -(offset * 7), &current_monday);
        let total_minutes = minutes_by_week.get(&wk).copied().unwrap_or(0.0).round() as i64;
        let total_hours = ((total_minutes as f64 / 60.0) * 10.0).round() / 10.0;
        buckets.push(WeekBucket {
            week_key: wk.clone(),
            short_label: if offset == 0 {
                "Minggu Ini".to_string()
            } else {
                format!("M-{offset}")
            },
            date_range_label: format_week_date_range(&wk),
            total_minutes,
            total_hours,
            session_count: count_by_week.get(&wk).copied().unwrap_or(0),
        });
    }
    buckets
}

// ============================================================================
// API UNGGUN & PROYEKSI
// ============================================================================

/// Tahap api unggun dari jam mingguan vs target jam.
pub fn get_campfire_stage(weekly_hours: f64, weekly_target_hours: f64) -> CampfireStageInfo {
    // v2: `Math.max(1, weeklyTargetHours || DEFAULT)` — 0/NaN memakai bawaan.
    let safe_target = if weekly_target_hours == 0.0 || !weekly_target_hours.is_finite() {
        DEFAULT_WEEKLY_TARGET_HOURS
    } else {
        weekly_target_hours.max(1.0)
    };
    let raw_percent = ((weekly_hours.max(0.0) / safe_target) * 100.0).round() as i64;
    let percent = raw_percent.min(100);

    if weekly_hours <= 0.0 {
        return CampfireStageInfo {
            stage: 0,
            percent: 0,
            name: "Bara Kayu Padam",
            description:
                "Belum ada sesi belajar minggu ini. Catat sesi atau mulai Pomodoro untuk menyalakan api!",
        };
    }
    if raw_percent < 40 {
        return CampfireStageInfo {
            stage: 1,
            percent,
            name: "Percikan Api Kecil",
            description: "Api mulai menyala hangat. Teruskan ritme belajarmu minggu ini!",
        };
    }
    if raw_percent < 75 {
        return CampfireStageInfo {
            stage: 2,
            percent,
            name: "Api Unggun Hangat",
            description:
                "Separuh perjalanan target mingguan telah ditempuh. Cahaya perkemahan makin terang!",
        };
    }
    if raw_percent < 100 {
        return CampfireStageInfo {
            stage: 3,
            percent,
            name: "Api Berkobar Terang",
            description:
                "Sedikit lagi menuju target mingguan! Bonus XP mingguan sudah di depan mata.",
        };
    }
    CampfireStageInfo {
        stage: 4,
        percent: 100,
        name: "Api Unggun Legendaris",
        description: "Target jam belajar minggu ini tercapai! Api berkobar penuh dengan berkah XP.",
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Proyeksi kasar bulan selesai berdasarkan topik tersisa dan rata-rata
/// jam/minggu aktual (hanya dari ember yang aktif).
pub fn calculate_projected_completion(
    completed_room_ids: &[String],
    sessions: &[StudySession],
    weekly_target_hours: f64,
    reference_date_key: &str,
) -> ProjectedCompletionInfo {
    let mut remaining_roadmap_weeks = 0.0f64;
    for topic in all_topics() {
        if !completed_room_ids.iter().any(|c| c == &topic.id) {
            remaining_roadmap_weeks += (topic.weeks.min as f64 + topic.weeks.max as f64) / 2.0;
        }
    }
    let remaining_roadmap_weeks = round1(remaining_roadmap_weeks);
    let remaining_estimated_hours =
        round1(remaining_roadmap_weeks * BASELINE_HOURS_PER_ROADMAP_WEEK);

    let buckets = get_last8_weeks_buckets(sessions, reference_date_key);
    let active_buckets: Vec<&WeekBucket> =
        buckets.iter().filter(|b| b.total_minutes > 0).collect();

    let mut actual_avg_hours_per_week = 0.0f64;
    if !active_buckets.is_empty() {
        let sum_hours: f64 = active_buckets
            .iter()
            .map(|b| b.total_minutes as f64 / 60.0)
            .sum();
        actual_avg_hours_per_week = round1(sum_hours / active_buckets.len() as f64);
    }

    let safe_target = if weekly_target_hours == 0.0 || !weekly_target_hours.is_finite() {
        DEFAULT_WEEKLY_TARGET_HOURS
    } else {
        weekly_target_hours.max(1.0)
    };
    let using_actual_average = actual_avg_hours_per_week > 0.0;
    let effective_hours_per_week = if using_actual_average {
        actual_avg_hours_per_week
    } else {
        safe_target
    };

    let (ref_year, ref_month, _) = dates::parse_date_key(reference_date_key)
        .unwrap_or((1970, 1, 1));
    if remaining_estimated_hours <= 0.0 {
        let month_name = dates::MONTHS_FULL_ID[(ref_month - 1) as usize];
        return ProjectedCompletionInfo {
            remaining_roadmap_weeks: 0.0,
            remaining_estimated_hours: 0.0,
            actual_avg_hours_per_week,
            effective_hours_per_week,
            using_actual_average,
            estimated_weeks_to_finish: 0,
            projected_month_year: format!("Selesai ({month_name} {ref_year})"),
        };
    }

    let estimated_weeks_to_finish = (1i64).max(
        (remaining_estimated_hours / effective_hours_per_week.max(0.5)).ceil() as i64,
    );
    let projected_key =
        dates::add_days(reference_date_key, estimated_weeks_to_finish * 7, reference_date_key);
    let (proj_year, proj_month, _) =
        dates::parse_date_key(&projected_key).unwrap_or((ref_year, ref_month, 1));
    let projected_month_year = format!(
        "{} {}",
        dates::MONTHS_FULL_ID[(proj_month - 1) as usize],
        proj_year
    );

    ProjectedCompletionInfo {
        remaining_roadmap_weeks,
        remaining_estimated_hours,
        actual_avg_hours_per_week,
        effective_hours_per_week,
        using_actual_average,
        estimated_weeks_to_finish,
        projected_month_year,
    }
}

// ============================================================================
// VALIDASI NODE LANTAI
// ============================================================================

/// `true` bila `node_id` adalah id topik atau id boss yang valid di lantai
/// yang diberikan (lantai 1-5).
pub fn is_valid_floor_node_id(floor_id: u32, node_id: &str) -> bool {
    if node_id.trim().is_empty() {
        return false;
    }
    let Some(f) = floor(floor_id) else {
        return false;
    };
    f.boss.id == node_id || f.topics.iter().any(|t| t.id == node_id)
}

// ============================================================================
// VALIDATOR SAVE JSON
// ============================================================================

pub fn is_valid_date_key(s: &str) -> bool {
    let t = s.trim();
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn as_finite_number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

fn floor_i64(x: f64) -> i64 {
    x.floor() as i64
}

/// Memvalidasi & menormalisasi string JSON save impor.
///
/// Mengembalikan peta field yang sudah disanitasi (hanya field yang dikenali
/// dan lolos batas), atau pesan galat berbahasa Indonesia. Mendukung
/// pembungkus Zustand `{ state, version }` maupun objek state langsung.
/// Entri yang tidak valid dibuang per-field tanpa menggagalkan impor —
/// sama seperti v2.
pub fn validate_and_parse_save_json(raw_json: &str) -> Result<Map<String, Value>, String> {
    if raw_json.trim().is_empty() {
        return Err("Teks atau file JSON kosong.".to_string());
    }
    if raw_json.len() > MAX_SAVE_FILE_BYTES {
        return Err(
            "Ukuran file terlalu besar (maksimal 1 MB). File simpanan ditolak.".to_string(),
        );
    }
    let parsed: Value = serde_json::from_str(raw_json).map_err(|_| {
        "Gagal membaca JSON. Pastikan format tanda kurung dan tanda kutip sudah benar."
            .to_string()
    })?;
    let Some(obj) = parsed.as_object() else {
        return Err("Format JSON tidak valid: harus berupa objek data.".to_string());
    };
    let candidate: &Map<String, Value> = match obj.get("state") {
        Some(Value::Object(inner)) => inner,
        _ => obj,
    };

    let has_known_keys = [
        "xp",
        "playerName",
        "completedRooms",
        "defeatedBosses",
        "towerCompleted",
        "completedTowerQuests",
        "studySessions",
        "lessonProgress",
        "reviewQueue",
    ]
    .iter()
    .any(|k| candidate.contains_key(*k));
    if !has_known_keys {
        return Err(
            "JSON tidak dikenali sebagai data simpanan Dungeon Belajar ML & DL.".to_string(),
        );
    }

    let mut sanitized: Map<String, Value> = Map::new();
    let get = |k: &str| candidate.get(k);

    // 1. version
    if let Some(v) = get("version").and_then(as_finite_number) {
        if v >= 1.0 {
            sanitized.insert(
                "version".to_string(),
                Value::from(floor_i64(v)),
            );
        }
    }
    // 2. hasStarted
    if let Some(b) = get("hasStarted").and_then(Value::as_bool) {
        sanitized.insert("hasStarted".to_string(), Value::Bool(b));
    }
    // 3. playerName (<= 28 karakter)
    if let Some(s) = get("playerName").and_then(Value::as_str) {
        let trimmed: String = s.trim().chars().take(28).collect();
        let name = if trimmed.is_empty() {
            "Petualang Tensor".to_string()
        } else {
            trimmed
        };
        sanitized.insert("playerName".to_string(), Value::String(name));
    }
    // 4. avatarId
    if let Some(s) = get("avatarId").and_then(Value::as_str) {
        if !s.trim().is_empty() {
            sanitized.insert("avatarId".to_string(), Value::String(s.trim().to_string()));
        }
    }
    // 5. xp
    if let Some(v) = get("xp").and_then(as_finite_number) {
        if v >= 0.0 {
            sanitized.insert("xp".to_string(), Value::from(floor_i64(v)));
        }
    }
    // 6. streak
    if let Some(v) = get("streak").and_then(as_finite_number) {
        if v >= 0.0 {
            sanitized.insert("streak".to_string(), Value::from(floor_i64(v)));
        }
    }
    // 7. lastActiveDate (null atau YYYY-MM-DD)
    match get("lastActiveDate") {
        Some(Value::Null) => {
            sanitized.insert("lastActiveDate".to_string(), Value::Null);
        }
        Some(Value::String(s)) if is_valid_date_key(s) => {
            sanitized.insert(
                "lastActiveDate".to_string(),
                Value::String(s.trim().to_string()),
            );
        }
        _ => {}
    }
    // 8-10, 12, 16, 17, 20-22, 29: array string.
    for key in [
        "completedRooms",
        "defeatedBosses",
        "completedTowerQuests",
        "towerCompleted",
        "relics",
        "badges",
        "quizBonusClaimedRooms",
        "echoBonusClaimedRooms",
        "spellsUnlocked",
        "claimedWeeklyBonusWeeks",
    ] {
        if let Some(arr) = get(key).and_then(Value::as_array) {
            let clean: Vec<Value> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| Value::String(s.to_string())))
                .collect();
            sanitized.insert(key.to_string(), Value::Array(clean));
        }
    }
    // 11. towerSteps (hanya boolean true yang disimpan)
    if let Some(Value::Object(steps)) = get("towerSteps") {
        let mut clean: Map<String, Value> = Map::new();
        for (q_id, step_map) in steps {
            if let Value::Object(inner) = step_map {
                let mut clean_inner: Map<String, Value> = Map::new();
                for (s_id, val) in inner {
                    if val.as_bool() == Some(true) {
                        clean_inner.insert(s_id.clone(), Value::Bool(true));
                    }
                }
                clean.insert(q_id.clone(), Value::Object(clean_inner));
            }
        }
        sanitized.insert("towerSteps".to_string(), Value::Object(clean));
    }
    // 13. reflections (<= 500 karakter)
    if let Some(Value::Object(m)) = get("reflections") {
        let mut clean: Map<String, Value> = Map::new();
        for (k, v) in m {
            if let Some(s) = v.as_str() {
                let clipped: String = s.chars().take(500).collect();
                clean.insert(k.clone(), Value::String(clipped));
            }
        }
        sanitized.insert("reflections".to_string(), Value::Object(clean));
    }
    // 14. specialization
    match get("specialization") {
        Some(Value::Null) => {
            sanitized.insert("specialization".to_string(), Value::Null);
        }
        Some(Value::String(s)) => {
            let t = s.trim();
            sanitized.insert(
                "specialization".to_string(),
                if t.is_empty() {
                    Value::Null
                } else {
                    Value::String(t.to_string())
                },
            );
        }
        _ => {}
    }
    // 15. certificateIssuedAt (string atau null, apa adanya)
    if let Some(v @ (Value::String(_) | Value::Null)) = get("certificateIssuedAt") {
        sanitized.insert("certificateIssuedAt".to_string(), v.clone());
    }
    // 18-19. quizBestScores / bestScoreByTopic (angka 0-100)
    for key in ["quizBestScores", "bestScoreByTopic"] {
        if let Some(Value::Object(m)) = get(key) {
            let mut clean: Map<String, Value> = Map::new();
            for (k, v) in m {
                if let Some(n) = as_finite_number(v) {
                    if (0.0..=100.0).contains(&n) {
                        clean.insert(k.clone(), Value::from(n));
                    }
                }
            }
            sanitized.insert(key.to_string(), Value::Object(clean));
        }
    }
    // 23. questionStats
    if let Some(Value::Object(m)) = get("questionStats") {
        let mut clean: Map<String, Value> = Map::new();
        for (q_id, raw) in m {
            if let Value::Object(st) = raw {
                let seen = st.get("seen").and_then(as_finite_number);
                let wrong = st.get("wrong").and_then(as_finite_number);
                let streak = st.get("correctStreak").and_then(as_finite_number);
                if let (Some(seen), Some(wrong), Some(streak)) = (seen, wrong, streak) {
                    if seen >= 0.0 && wrong >= 0.0 && streak >= 0.0 {
                        let mut entry: Map<String, Value> = Map::new();
                        entry.insert("seen".to_string(), Value::from(floor_i64(seen)));
                        entry.insert("wrong".to_string(), Value::from(floor_i64(wrong)));
                        entry.insert(
                            "correctStreak".to_string(),
                            Value::from(floor_i64(streak)),
                        );
                        clean.insert(q_id.clone(), Value::Object(entry));
                    }
                }
            }
        }
        sanitized.insert("questionStats".to_string(), Value::Object(clean));
    }
    // 24. bossProgress (diteruskan apa adanya; migrasi v2→v3 ada di state.rs)
    if let Some(Value::Object(_)) = get("bossProgress") {
        sanitized.insert(
            "bossProgress".to_string(),
            get("bossProgress").unwrap().clone(),
        );
    }
    // 25. dojoDailyXP
    if let Some(Value::Object(dj)) = get("dojoDailyXP") {
        let date_ok = dj.get("date").and_then(Value::as_str).is_some();
        let xp = dj.get("xp").and_then(as_finite_number);
        if date_ok {
            if let Some(xp) = xp {
                if xp >= 0.0 {
                    let mut entry: Map<String, Value> = Map::new();
                    entry.insert(
                        "date".to_string(),
                        Value::String(dj["date"].as_str().unwrap().to_string()),
                    );
                    entry.insert("xp".to_string(), Value::from(floor_i64(xp)));
                    sanitized.insert("dojoDailyXP".to_string(), Value::Object(entry));
                }
            }
        }
    }
    // 26. evidenceNotes (<= 300 karakter)
    if let Some(Value::Object(m)) = get("evidenceNotes") {
        let mut clean: Map<String, Value> = Map::new();
        for (k, v) in m {
            if let Some(s) = v.as_str() {
                let clipped: String = s.chars().take(300).collect();
                clean.insert(k.clone(), Value::String(clipped));
            }
        }
        sanitized.insert("evidenceNotes".to_string(), Value::Object(clean));
    }
    // 27. studySessions (entri valid saja, utuh)
    if let Some(arr) = get("studySessions").and_then(Value::as_array) {
        let clean: Vec<Value> = arr
            .iter()
            .filter(|s| {
                let Some(o) = s.as_object() else {
                    return false;
                };
                let id_ok = o.get("id").and_then(Value::as_str).is_some();
                let topic_ok = o.get("topicId").and_then(Value::as_str).is_some();
                let dur_ok = o
                    .get("durationMinutes")
                    .and_then(as_finite_number)
                    .map(|d| d > 0.0)
                    .unwrap_or(false);
                let date_ok = o
                    .get("date")
                    .and_then(Value::as_str)
                    .map(is_valid_date_key)
                    .unwrap_or(false);
                id_ok && topic_ok && dur_ok && date_ok
            })
            .cloned()
            .collect();
        sanitized.insert("studySessions".to_string(), Value::Array(clean));
    }
    // 28. weeklyTargetHours (1-80)
    if let Some(v) = get("weeklyTargetHours").and_then(as_finite_number) {
        if v >= 1.0 {
            sanitized.insert(
                "weeklyTargetHours".to_string(),
                Value::from(v.clamp(1.0, 80.0)),
            );
        }
    }
    // 30. activeFloorId (1-6, bilangan bulat)
    if let Some(v) = get("activeFloorId").and_then(as_finite_number) {
        if v.fract() == 0.0 && (1.0..=6.0).contains(&v) {
            sanitized.insert("activeFloorId".to_string(), Value::from(v as i64));
        }
    }
    // 31. activeNodeByFloor (lantai 1-5 -> node valid di lantai itu)
    if let Some(Value::Object(m)) = get("activeNodeByFloor") {
        let mut clean: Map<String, Value> = Map::new();
        for (f_key, node_val) in m {
            if let Ok(floor_num) = f_key.parse::<u32>() {
                if (1..=5).contains(&floor_num) {
                    if let Some(node) = node_val.as_str() {
                        if is_valid_floor_node_id(floor_num, node) {
                            clean.insert(f_key.clone(), Value::String(node.to_string()));
                        }
                    }
                }
            }
        }
        sanitized.insert("activeNodeByFloor".to_string(), Value::Object(clean));
    }
    // 32/34/36. boolean
    for key in ["soundEnabled", "musicEnabled", "reduceMotion"] {
        if let Some(b) = get(key).and_then(Value::as_bool) {
            sanitized.insert(key.to_string(), Value::Bool(b));
        }
    }
    // 33/35. volume (0-1, dibulatkan 2 desimal)
    for key in ["sfxVolume", "musicVolume"] {
        if let Some(v) = get(key).and_then(as_finite_number) {
            let rounded = (v * 100.0).round() / 100.0;
            sanitized.insert(key.to_string(), Value::from(rounded.clamp(0.0, 1.0)));
        }
    }
    // 37. lessonProgress
    if let Some(Value::Object(m)) = get("lessonProgress") {
        let mut clean: Map<String, Value> = Map::new();
        for (c_id, val) in m {
            let Some(o) = val.as_object() else {
                continue;
            };
            if let Some(done) = o.get("done") {
                if done.as_bool().is_none() {
                    continue;
                }
            }
            if let Some(att) = o.get("attempts") {
                match as_finite_number(att) {
                    Some(n) if n >= 0.0 => {}
                    _ => continue,
                }
            }
            if let Some(act) = o.get("attemptedActivities") {
                if !act.is_array() {
                    continue;
                }
            }
            let done = o.get("done").and_then(Value::as_bool).unwrap_or(false);
            let attempts = o
                .get("attempts")
                .and_then(as_finite_number)
                .map(floor_i64)
                .unwrap_or(0);
            let activities: Vec<Value> = o
                .get("attemptedActivities")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| Value::String(s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let mut entry: Map<String, Value> = Map::new();
            entry.insert("done".to_string(), Value::Bool(done));
            entry.insert("attempts".to_string(), Value::from(attempts));
            entry.insert("attemptedActivities".to_string(), Value::Array(activities));
            clean.insert(c_id.clone(), Value::Object(entry));
        }
        sanitized.insert("lessonProgress".to_string(), Value::Object(clean));
    }
    // 38. summaries (<= 400 karakter)
    if let Some(Value::Object(m)) = get("summaries") {
        let mut clean: Map<String, Value> = Map::new();
        for (k, v) in m {
            if let Some(s) = v.as_str() {
                let clipped: String = s.chars().take(400).collect();
                clean.insert(k.clone(), Value::String(clipped));
            }
        }
        sanitized.insert("summaries".to_string(), Value::Object(clean));
    }
    // 39. taskDone (boolean saja)
    if let Some(Value::Object(m)) = get("taskDone") {
        let mut clean: Map<String, Value> = Map::new();
        for (k, v) in m {
            if let Some(b) = v.as_bool() {
                clean.insert(k.clone(), Value::Bool(b));
            }
        }
        sanitized.insert("taskDone".to_string(), Value::Object(clean));
    }
    // 40. reviewQueue (box 0-4, due YYYY-MM-DD)
    if let Some(Value::Object(m)) = get("reviewQueue") {
        let mut clean: Map<String, Value> = Map::new();
        for (item_id, val) in m {
            let Some(o) = val.as_object() else {
                continue;
            };
            let Some(box_n) = o.get("box").and_then(as_finite_number) else {
                continue;
            };
            let Some(due) = o.get("due").and_then(Value::as_str) else {
                continue;
            };
            if !is_valid_date_key(due) {
                continue;
            }
            let mut entry: Map<String, Value> = Map::new();
            entry.insert(
                "box".to_string(),
                Value::from((floor_i64(box_n)).clamp(0, 4)),
            );
            entry.insert("due".to_string(), Value::String(due.trim().to_string()));
            clean.insert(item_id.clone(), Value::Object(entry));
        }
        sanitized.insert("reviewQueue".to_string(), Value::Object(clean));
    }
    // 41. settings (intervalScale 0,5-2, dibulatkan 1 desimal)
    if let Some(Value::Object(raw)) = get("settings") {
        if let Some(scale) = raw.get("intervalScale").and_then(as_finite_number) {
            let clamped = ((scale * 10.0).round() / 10.0).clamp(0.5, 2.0);
            let mut entry: Map<String, Value> = Map::new();
            entry.insert("intervalScale".to_string(), Value::from(clamped));
            sanitized.insert("settings".to_string(), Value::Object(entry));
        }
    }

    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, topic: &str, minutes: f64, date: &str, created: i64) -> StudySession {
        StudySession {
            id: id.to_string(),
            topic_id: topic.to_string(),
            duration_minutes: minutes,
            date: date.to_string(),
            note: None,
            source: StudySessionSource::Manual,
            created_at: created,
        }
    }

    #[test]
    fn week_start_is_monday_for_any_weekday() {
        assert_eq!(
            get_week_start_date_key("2026-10-30", "2026-10-30"),
            "2026-10-26"
        ); // Jumat -> Senin 26 Okt
        assert_eq!(
            get_week_start_date_key("2026-11-01", "2026-11-01"),
            "2026-10-26"
        ); // Minggu -> Senin 26 Okt
        assert_eq!(
            get_week_start_date_key("2026-10-26", "2026-10-26"),
            "2026-10-26"
        ); // Senin tetap
    }

    #[test]
    fn last8_weeks_buckets_aggregate_hours() {
        let sessions = vec![
            session("s1", "topic-1", 120.0, "2026-10-30", 1),
            session("s2", "topic-2", 90.0, "2026-10-27", 2),
            session("s3", "topic-1", 180.0, "2026-10-20", 3),
        ];
        let buckets = get_last8_weeks_buckets(&sessions, "2026-10-30");
        assert_eq!(buckets.len(), 8);
        assert_eq!(buckets[7].short_label, "Minggu Ini");
        assert_eq!(buckets[7].total_hours, 3.5);
        assert_eq!(buckets[7].session_count, 2);
        assert_eq!(buckets[6].short_label, "M-1");
        assert_eq!(buckets[6].total_hours, 3.0);
        assert_eq!(buckets[6].session_count, 1);
    }

    #[test]
    fn campfire_stage_scales_0_to_4() {
        assert_eq!(get_campfire_stage(0.0, 9.0).stage, 0);
        assert_eq!(get_campfire_stage(2.0, 9.0).stage, 1); // 22%
        assert_eq!(get_campfire_stage(4.5, 9.0).stage, 2); // 50%
        assert_eq!(get_campfire_stage(7.5, 9.0).stage, 3); // 83%
        assert_eq!(get_campfire_stage(9.0, 9.0).stage, 4); // 100%
        assert_eq!(get_campfire_stage(12.0, 9.0).stage, 4); // >100%
    }

    #[test]
    fn projected_completion_uses_actual_average() {
        let sessions = vec![session("s1", "topic-1", 600.0, "2026-10-30", 1)];
        let proj = calculate_projected_completion(&[], &sessions, 9.0, "2026-10-30");
        assert!(proj.using_actual_average);
        assert_eq!(proj.actual_avg_hours_per_week, 10.0);
        assert!(proj.estimated_weeks_to_finish > 0);
        assert!(proj.projected_month_year.len() > 4);
    }

    #[test]
    fn validate_save_accepts_valid_and_rejects_broken() {
        let good = serde_json::json!({
            "state": {
                "playerName": "Arsitek Neural",
                "xp": 450,
                "completedRooms": ["topic-1", "topic-6"],
                "weeklyTargetHours": 12
            },
            "version": 1
        })
        .to_string();
        let res = validate_and_parse_save_json(&good).unwrap();
        assert_eq!(res["playerName"], "Arsitek Neural");
        assert_eq!(res["xp"], 450);
        assert_eq!(res["weeklyTargetHours"].as_f64(), Some(12.0));

        assert!(validate_and_parse_save_json("{ broken json ").is_err());
        assert!(validate_and_parse_save_json("").is_err());
        assert!(validate_and_parse_save_json("{}").is_err());
    }

    #[test]
    fn validate_save_sanitizes_dirty_payload_per_field() {
        // Port bagian P-06 campTracker.test.ts (payload kotor).
        let dirty = serde_json::json!({
            "version": 3,
            "state": {
                "playerName": "A".repeat(50),
                "summaries": { "t1c1": "S".repeat(500), "badType": 12345 },
                "evidenceNotes": { "21": "E".repeat(400), "badNote": null },
                "reflections": { "q1": "R".repeat(600), "q2": "Valid reflection", "badRefl": 999 },
                "reviewQueue": {
                    "validCard": { "box": 4, "due": "2026-11-15" },
                    "clampedBox": { "box": 99, "due": "2026-11-16" },
                    "badDate": { "box": 2, "due": "not-a-date" }
                },
                "settings": { "intervalScale": 9.9 },
                "taskDone": { "t1c1": true, "t1c2": false, "t1c3": "not-a-bool" },
                "lessonProgress": {
                    "t1c1": { "done": true, "attempts": 3, "attemptedActivities": ["guess", 999, "p0"] },
                    "badEntry": "invalid"
                },
                "activeFloorId": 99,
                "activeNodeByFloor": {
                    "1": "topic-2",
                    "2": "topic-999",
                    "3": "topic-1",
                    "4": "boss-4"
                }
            }
        })
        .to_string();
        let res = validate_and_parse_save_json(&dirty).unwrap();
        assert_eq!(res["playerName"].as_str().unwrap().chars().count(), 28);
        assert_eq!(res["summaries"]["t1c1"].as_str().unwrap().chars().count(), 400);
        assert!(res["summaries"].get("badType").is_none());
        assert_eq!(
            res["evidenceNotes"]["21"].as_str().unwrap().chars().count(),
            300
        );
        assert!(res["evidenceNotes"].get("badNote").is_none());
        assert_eq!(res["reflections"]["q1"].as_str().unwrap().chars().count(), 500);
        assert_eq!(res["reflections"]["q2"], "Valid reflection");
        assert!(res["reflections"].get("badRefl").is_none());
        assert_eq!(res["reviewQueue"]["validCard"]["box"], 4);
        assert_eq!(res["reviewQueue"]["clampedBox"]["box"], 4);
        assert!(res["reviewQueue"].get("badDate").is_none());
        assert_eq!(res["settings"]["intervalScale"], 2.0);
        assert_eq!(res["taskDone"]["t1c1"], true);
        assert_eq!(res["taskDone"]["t1c2"], false);
        assert!(res["taskDone"].get("t1c3").is_none());
        assert_eq!(
            res["lessonProgress"]["t1c1"]["attemptedActivities"],
            serde_json::json!(["guess", "p0"])
        );
        assert!(res["lessonProgress"].get("badEntry").is_none());
        assert!(res.get("activeFloorId").is_none());
        assert_eq!(res["activeNodeByFloor"]["1"], "topic-2");
        assert!(res["activeNodeByFloor"].get("2").is_none());
        assert!(res["activeNodeByFloor"].get("3").is_none());
        assert_eq!(res["activeNodeByFloor"]["4"], "boss-4");

        // Oversized ditolak.
        let oversized = "x".repeat(1024 * 1024 + 10);
        let err = validate_and_parse_save_json(&oversized).unwrap_err();
        assert!(err.contains("1 MB"));
    }
}
