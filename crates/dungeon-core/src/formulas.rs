//! Rumus dan aturan angka permainan — port `src/game/formulas.ts` v2.
//!
//! Semua angka di sini adalah spesifikasi (spec Bagian 3): bila hasil porting
//! berbeda dari angka ini, portingnya yang salah.

use crate::dates;
use dungeon_data::roadmap::{self, TopicItem};

/// XP mengalahkan boss lantai.
pub const BOSS_XP: i64 = 300;
/// XP menyelesaikan satu quest Menara Master.
pub const TOWER_QUEST_XP: i64 = 150;

/// XP ruangan = round(((min + max) / 2) * 50).
pub fn calculate_room_xp(min_weeks: u32, max_weeks: u32) -> i64 {
    (((min_weeks as f64 + max_weeks as f64) / 2.0) * 50.0).round() as i64
}

/// XP yang dibutuhkan untuk mencapai level L = 60 * (L - 1)^2.
/// L1 = 0, L2 = 60, L3 = 240, L10 = 4860.
pub fn xp_required_for_level(level: i64) -> i64 {
    if level <= 1 {
        0
    } else {
        60 * (level - 1).pow(2)
    }
}

/// Level saat ini dari total XP (tanpa batas level).
pub fn calculate_level_from_xp(xp: i64) -> i64 {
    if xp <= 0 {
        1
    } else {
        ((xp as f64 / 60.0).sqrt().floor() as i64) + 1
    }
}

/// Gelar per level:
/// Lv1-2 "Novice Data", Lv3-4 "Data Apprentice", Lv5-6 "Neural Knight",
/// Lv7-9 "Edge Ranger", Lv10+ "ML Archmage".
pub fn get_title_for_level(level: i64) -> &'static str {
    if level <= 2 {
        "Novice Data"
    } else if level <= 4 {
        "Data Apprentice"
    } else if level <= 6 {
        "Neural Knight"
    } else if level <= 9 {
        "Edge Ranger"
    } else {
        "ML Archmage"
    }
}

/// Kemajuan level pemain.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelProgress {
    pub level: i64,
    pub title: &'static str,
    pub current_level_xp: i64,
    pub next_level_xp: i64,
    pub xp_in_current_level: i64,
    pub xp_needed_for_next_level: i64,
    pub progress_percent: i64,
}

pub fn get_level_progress(xp: i64) -> LevelProgress {
    let safe_xp = xp.max(0);
    let level = calculate_level_from_xp(safe_xp);
    let current_level_xp = xp_required_for_level(level);
    let next_level_xp = xp_required_for_level(level + 1);
    let xp_in_current_level = safe_xp - current_level_xp;
    let xp_needed_for_next_level = next_level_xp - current_level_xp;
    let progress_percent = if xp_needed_for_next_level > 0 {
        (((xp_in_current_level as f64 / xp_needed_for_next_level as f64) * 100.0).round()
            as i64)
            .clamp(0, 100)
    } else {
        100
    };
    LevelProgress {
        level,
        title: get_title_for_level(level),
        current_level_xp,
        next_level_xp,
        xp_in_current_level,
        xp_needed_for_next_level,
        progress_percent,
    }
}

/// Buka-kunci lantai:
/// - Lantai 1 dan 5 (Guild Hall) terbuka sejak awal.
/// - Lantai 2-4 terbuka setelah boss lantai sebelumnya kalah.
/// - Lantai 6 (Menara Master) terbuka setelah kelima boss kalah.
pub fn is_floor_unlocked(floor_id: u32, defeated_bosses: &[String]) -> bool {
    match floor_id {
        1 | 5 => true,
        2 => defeated_bosses.iter().any(|b| b == "boss-1"),
        3 => defeated_bosses.iter().any(|b| b == "boss-2"),
        4 => defeated_bosses.iter().any(|b| b == "boss-3"),
        6 => ["boss-1", "boss-2", "boss-3", "boss-4", "boss-5"]
            .iter()
            .all(|id| defeated_bosses.iter().any(|b| b == id)),
        _ => false,
    }
}

/// Buka-kunci ruangan:
/// - Lantai harus terbuka.
/// - Topik `parallel` (Topik 6 di Lantai 1, Topik 21-28 di Lantai 5)
///   terbuka sejak awal lantainya.
/// - Topik sekuensial terbuka bila topik non-paralel sebelumnya selesai.
pub fn is_room_unlocked(
    topic: &TopicItem,
    floor_topics: &[TopicItem],
    completed_rooms: &[String],
    defeated_bosses: &[String],
) -> bool {
    if !is_floor_unlocked(topic.floor_id, defeated_bosses) {
        return false;
    }
    if topic.parallel {
        return true;
    }
    let sequential: Vec<&TopicItem> = floor_topics.iter().filter(|t| !t.parallel).collect();
    let idx = match sequential.iter().position(|t| t.id == topic.id) {
        Some(i) => i,
        None => return false,
    };
    if idx == 0 {
        return true;
    }
    let prev = sequential[idx - 1];
    completed_rooms.iter().any(|r| r == &prev.id)
}

/// Boss terbuka bila semua ruangan lantainya selesai.
pub fn is_boss_unlocked(
    floor_id: u32,
    floor_topics: &[TopicItem],
    completed_rooms: &[String],
    defeated_bosses: &[String],
) -> bool {
    if !is_floor_unlocked(floor_id, defeated_bosses) {
        return false;
    }
    floor_topics
        .iter()
        .all(|t| completed_rooms.iter().any(|r| r == &t.id))
}

/// Hasil perhitungan streak: (streak_baru, tanggal_aktif_terakhir).
///
/// Streak: +1 per hari kalender lokal dengan aktivitas (ruangan, boss, atau
/// quest selesai); reset ke 1 bila ada hari yang terlewat. `today_key`
/// dipasok pemanggil (core tidak membaca jam sistem).
pub fn compute_next_streak(
    current_streak: i64,
    last_active_date: Option<&str>,
    today_key: &str,
) -> (i64, String) {
    let last = match last_active_date {
        None => return (1, today_key.to_string()),
        Some(d) => d,
    };
    match dates::diff_days(last, today_key) {
        Some(0) => (current_streak.max(1), today_key.to_string()),
        Some(1) => (current_streak.max(0) + 1, today_key.to_string()),
        // Terlewat >= 1 hari, atau kunci tanggal tidak valid / jam mundur:
        // reset ke 1 untuk aktivitas hari ini (sama seperti v2).
        _ => (1, today_key.to_string()),
    }
}

/// Streak yang ditampilkan: 0 bila jeda dari aktivitas terakhir > 1 hari.
pub fn get_display_streak(
    current_streak: i64,
    last_active_date: Option<&str>,
    today_key: &str,
) -> i64 {
    let last = match last_active_date {
        None => return 0,
        Some(d) => d,
    };
    if current_streak <= 0 {
        return 0;
    }
    match dates::diff_days(last, today_key) {
        Some(0) | Some(1) => current_streak,
        _ => 0,
    }
}

/// Minggu roadmap yang sudah ditaklukkan.
#[derive(Debug, Clone, PartialEq)]
pub struct WeeksConquered {
    pub conquered_min: u32,
    pub conquered_max: u32,
    pub raw_total_min: u32,
    pub raw_total_max: u32,
    pub effective_conquered_min: u32,
    pub effective_conquered_max: u32,
    pub effective_total_min: u32,
    pub effective_total_max: u32,
    pub percent_conquered: u32,
}

/// Minggu ditaklukkan vs total estimasi (mentah 79-104, efektif 66-88).
pub fn calculate_weeks_conquered(completed_rooms: &[String]) -> WeeksConquered {
    let mut conquered_min: u32 = 0;
    let mut conquered_max: u32 = 0;
    for topic in roadmap::all_topics() {
        if completed_rooms.iter().any(|r| r == &topic.id) {
            conquered_min += topic.weeks.min;
            conquered_max += topic.weeks.max;
        }
    }
    let raw_total_min: u32 = roadmap::floors().iter().map(|f| f.weeks.min).sum();
    let raw_total_max: u32 = roadmap::floors().iter().map(|f| f.weeks.max).sum();
    let ratio_min = if raw_total_min > 0 {
        conquered_min as f64 / raw_total_min as f64
    } else {
        0.0
    };
    let ratio_max = if raw_total_max > 0 {
        conquered_max as f64 / raw_total_max as f64
    } else {
        0.0
    };
    let effective_conquered_min = (ratio_min * 66.0).round() as u32;
    let effective_conquered_max = (ratio_max * 88.0).round() as u32;
    let percent_conquered = (((conquered_min + conquered_max) as f64
        / (raw_total_min + raw_total_max) as f64)
        * 100.0)
        .round() as u32;
    WeeksConquered {
        conquered_min,
        conquered_max,
        raw_total_min,
        raw_total_max,
        effective_conquered_min,
        effective_conquered_max,
        effective_total_min: 66,
        effective_total_max: 88,
        percent_conquered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_xp_formula() {
        // Port formulas.test.ts: round(((min + max) / 2) * 50).
        assert_eq!(calculate_room_xp(1, 2), 75);
        assert_eq!(calculate_room_xp(3, 4), 175);
        assert_eq!(calculate_room_xp(4, 5), 225);
        assert_eq!(calculate_room_xp(3, 3), 150);
        assert_eq!(calculate_room_xp(6, 8), 350);
        assert_eq!(BOSS_XP, 300);
        assert_eq!(TOWER_QUEST_XP, 150);
    }

    #[test]
    fn level_xp_threshold_formula() {
        assert_eq!(xp_required_for_level(1), 0);
        assert_eq!(xp_required_for_level(2), 60);
        assert_eq!(xp_required_for_level(3), 240);
        assert_eq!(xp_required_for_level(4), 540);
        assert_eq!(xp_required_for_level(10), 4860);

        assert_eq!(calculate_level_from_xp(0), 1);
        assert_eq!(calculate_level_from_xp(59), 1);
        assert_eq!(calculate_level_from_xp(60), 2);
        assert_eq!(calculate_level_from_xp(239), 2);
        assert_eq!(calculate_level_from_xp(240), 3);
        assert_eq!(calculate_level_from_xp(4860), 10);
    }

    #[test]
    fn titles_match_level_brackets() {
        assert_eq!(get_title_for_level(1), "Novice Data");
        assert_eq!(get_title_for_level(2), "Novice Data");
        assert_eq!(get_title_for_level(3), "Data Apprentice");
        assert_eq!(get_title_for_level(4), "Data Apprentice");
        assert_eq!(get_title_for_level(5), "Neural Knight");
        assert_eq!(get_title_for_level(6), "Neural Knight");
        assert_eq!(get_title_for_level(7), "Edge Ranger");
        assert_eq!(get_title_for_level(9), "Edge Ranger");
        assert_eq!(get_title_for_level(10), "ML Archmage");
        assert_eq!(get_title_for_level(15), "ML Archmage");
    }

    #[test]
    fn parallel_and_sequential_room_unlock_rules() {
        let floors = roadmap::floors();
        let floor1 = &floors[0];
        let floor5 = &floors[4];

        assert!(is_floor_unlocked(1, &[]));
        assert!(!is_floor_unlocked(2, &[]));
        assert!(is_floor_unlocked(5, &[]));
        assert!(!is_floor_unlocked(6, &[]));

        let no_rooms: Vec<String> = vec![];
        let no_bosses: Vec<String> = vec![];
        // Topik 1 (sekuensial pertama) dan Topik 6 (paralel) terbuka di awal.
        assert!(is_room_unlocked(
            &floor1.topics[0],
            &floor1.topics,
            &no_rooms,
            &no_bosses
        ));
        assert!(!is_room_unlocked(
            &floor1.topics[1],
            &floor1.topics,
            &no_rooms,
            &no_bosses
        ));
        assert!(is_room_unlocked(
            &floor1.topics[5],
            &floor1.topics,
            &no_rooms,
            &no_bosses
        ));
        // Menyelesaikan Topik 1 membuka Topik 2.
        assert!(is_room_unlocked(
            &floor1.topics[1],
            &floor1.topics,
            &["topic-1".to_string()],
            &no_bosses
        ));
        // Semua 8 Guild Quest Lantai 5 terbuka sejak awal.
        for gq in &floor5.topics {
            assert!(is_room_unlocked(gq, &floor5.topics, &no_rooms, &no_bosses));
        }
        // Boss 1 terbuka hanya bila keenam topik Lantai 1 selesai.
        let five: Vec<String> = (1..=5).map(|i| format!("topic-{i}")).collect();
        assert!(!is_boss_unlocked(1, &floor1.topics, &five, &no_bosses));
        let six: Vec<String> = (1..=6).map(|i| format!("topic-{i}")).collect();
        assert!(is_boss_unlocked(1, &floor1.topics, &six, &no_bosses));
        // Lantai 6 terbuka hanya bila kelima boss kalah.
        let all_bosses: Vec<String> = (1..=5).map(|i| format!("boss-{i}")).collect();
        assert!(is_floor_unlocked(6, &all_bosses));
    }

    #[test]
    fn streak_increments_and_resets() {
        let (s1, last1) = compute_next_streak(0, None, "2026-10-28");
        assert_eq!(s1, 1);
        // Hari yang sama: streak tetap 1.
        let (s1_same, _) = compute_next_streak(s1, Some(&last1), "2026-10-28");
        assert_eq!(s1_same, 1);
        // Hari berikutnya berurutan: naik ke 2.
        let (s2, last2) = compute_next_streak(s1, Some(&last1), "2026-10-29");
        assert_eq!(s2, 2);
        // Terlewat 2026-10-30, aktivitas 2026-10-31: tampilan 0, lalu reset ke 1.
        assert_eq!(get_display_streak(s2, Some(&last2), "2026-10-31"), 0);
        let (s_reset, _) = compute_next_streak(s2, Some(&last2), "2026-10-31");
        assert_eq!(s_reset, 1);
    }

    #[test]
    fn roadmap_data_integrity() {
        let all: Vec<String> = roadmap::all_topics().iter().map(|t| t.id.clone()).collect();
        assert_eq!(all.len(), 28);
        let weeks = calculate_weeks_conquered(&all);
        assert_eq!(weeks.conquered_min, 79);
        assert_eq!(weeks.conquered_max, 104);
        assert_eq!(weeks.percent_conquered, 100);
    }
}
