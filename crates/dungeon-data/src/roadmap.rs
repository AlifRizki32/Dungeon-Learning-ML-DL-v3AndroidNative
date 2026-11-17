//! Model serde untuk roadmap v2 (`src/data/roadmap.js`).
//!
//! `roadmap.js` adalah modul JavaScript, bukan JSON. Isinya dikonversi SEKALI
//! secara mekanis menjadi `assets/roadmap.json` (Node: import + JSON.stringify,
//! tanpa perubahan isi) lalu dimuat di sini dengan serde. Konten tidak ditulis
//! ulang; status kurikulum `needs-human-review` ikut apa adanya (di file
//! konten/pelajaran, bukan di roadmap).
//!
//! Turunan (`ALL_TOPICS`, `ALL_RELICS`, `ALL_BADGES`) dihitung di Rust seperti
//! di v2 (`FLOORS.flatMap(...)`), bukan disalin sebagai data.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Rentang minggu estimasi sebuah topik/lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeeksRange {
    pub min: u32,
    pub max: u32,
}

/// Relik (hadiah boss lantai atau Guild Quest).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelicItem {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(rename = "iconType")]
    pub icon_type: String,
    pub description: String,
}

/// Lencana (badge) boss lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadgeItem {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Satu topik (ruangan) di roadmap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicItem {
    pub id: String,
    pub number: u32,
    #[serde(rename = "floorId")]
    pub floor_id: u32,
    pub name: String,
    pub scope: String,
    pub weeks: WeeksRange,
    #[serde(rename = "understandingSign")]
    pub understanding_sign: String,
    pub parallel: bool,
    #[serde(rename = "parallelNote", default)]
    pub parallel_note: Option<String>,
    #[serde(rename = "monsterName")]
    pub monster_name: String,
    pub flavor: String,
    #[serde(default)]
    pub relic: Option<RelicItem>,
}

/// Boss satu lantai.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossItem {
    pub id: String,
    #[serde(rename = "floorId")]
    pub floor_id: u32,
    pub name: String,
    pub title: String,
    #[serde(rename = "spriteType")]
    pub sprite_type: String,
    pub challenge: String,
    pub badge: BadgeItem,
    pub relic: RelicItem,
}

/// Satu lantai (fase) roadmap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorItem {
    pub id: u32,
    #[serde(rename = "floorNumber")]
    pub floor_number: u32,
    #[serde(rename = "phaseTitle")]
    pub phase_title: String,
    #[serde(rename = "shortTitle")]
    pub short_title: String,
    pub subtitle: String,
    pub weeks: WeeksRange,
    #[serde(rename = "monsterFamily")]
    pub monster_family: String,
    #[serde(rename = "monsterType")]
    pub monster_type: String,
    #[serde(rename = "themeColor")]
    pub theme_color: String,
    #[serde(rename = "isGuildHall")]
    pub is_guild_hall: bool,
    #[serde(rename = "parallelTag", default)]
    pub parallel_tag: Option<String>,
    pub boss: BossItem,
    pub topics: Vec<TopicItem>,
}

/// Satu quest Road to Master (Lantai 6, versi roadmap).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerQuest {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub description: String,
    pub references: Vec<String>,
    pub xp: u32,
}

/// Lantai 6: Menara Master (versi roadmap; konten lengkap ada di content.rs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TowerFloor {
    pub id: u32,
    #[serde(rename = "floorNumber")]
    pub floor_number: u32,
    pub title: String,
    pub subtitle: String,
    #[serde(rename = "unverifiedLinksNote")]
    pub unverified_links_note: String,
    #[serde(rename = "guardianName")]
    pub guardian_name: String,
    pub quests: Vec<TowerQuest>,
}

/// Satu avatar yang bisa dipilih pemain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Avatar {
    pub id: String,
    pub name: String,
    pub role: String,
    pub color: String,
}

/// Asumsi estimasi waktu roadmap (teks apa adanya dari v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadmapAssumptions {
    #[serde(rename = "summaryShort")]
    pub summary_short: String,
    #[serde(rename = "totalBeforeOverlap")]
    pub total_before_overlap: TotalLabel,
    #[serde(rename = "totalAfterPythonParallel")]
    pub total_after_python_parallel: TotalLabel,
    #[serde(rename = "totalEffective")]
    pub total_effective: EffectiveTotal,
    pub disclaimer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TotalLabel {
    pub min: u32,
    pub max: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveTotal {
    pub min: u32,
    pub max: u32,
    #[serde(rename = "monthsMin")]
    pub months_min: u32,
    #[serde(rename = "monthsMax")]
    pub months_max: u32,
    #[serde(rename = "hoursPerWeek")]
    pub hours_per_week: String,
    pub label: String,
}

/// Akar `assets/roadmap.json` (konversi mekanis `roadmap.js`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roadmap {
    #[serde(rename = "ROADMAP_ASSUMPTIONS")]
    pub assumptions: RoadmapAssumptions,
    #[serde(rename = "AVATARS")]
    pub avatars: Vec<Avatar>,
    #[serde(rename = "FLOORS")]
    pub floors: Vec<FloorItem>,
    #[serde(rename = "TOWER_FLOOR")]
    pub tower_floor: TowerFloor,
}

static ROADMAP: OnceLock<Roadmap> = OnceLock::new();

/// Memuat roadmap dari aset yang dibundel (di-parse sekali, lalu dipakai ulang).
///
/// Panic hanya bila aset bundel rusak — aset ini diverifikasi oleh tes
/// `dungeon-data` (`roadmap_parses_and_counts_match_v2`).
pub fn roadmap() -> &'static Roadmap {
    ROADMAP.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/roadmap.json"))
            .expect("assets/roadmap.json rusak: tidak bisa di-parse")
    })
}

/// Semua lantai 1-5.
pub fn floors() -> &'static [FloorItem] {
    &roadmap().floors
}

/// Lantai berdasarkan id (1-5), `None` bila tidak ada.
pub fn floor(floor_id: u32) -> Option<&'static FloorItem> {
    floors().iter().find(|f| f.id == floor_id)
}

/// Semua 28 topik, urutan sama seperti v2 (`FLOORS.flatMap(f => f.topics)`).
pub fn all_topics() -> Vec<&'static TopicItem> {
    floors().iter().flat_map(|f| f.topics.iter()).collect()
}

/// Topik berdasarkan id roadmap (`"topic-1"` .. `"topic-28"`).
pub fn topic_by_id(topic_id: &str) -> Option<&'static TopicItem> {
    floors()
        .iter()
        .flat_map(|f| f.topics.iter())
        .find(|t| t.id == topic_id)
}

/// Topik berdasarkan nomor (1-28).
pub fn topic_by_number(number: u32) -> Option<&'static TopicItem> {
    topic_by_id(&format!("topic-{number}"))
}

/// Lantai untuk nomor topik (1-6 -> 1, 7-11 -> 2, 12-17 -> 3, 18-20 -> 4,
/// 21-28 -> 5). Di luar itu kembali 1, sama seperti `getFloorForTopicNumber` v2.
pub fn floor_for_topic_number(number: u32) -> u32 {
    match number {
        1..=6 => 1,
        7..=11 => 2,
        12..=17 => 3,
        18..=20 => 4,
        21..=28 => 5,
        _ => 1,
    }
}

/// Semua relik: 5 relik boss + 8 relik Guild Quest (Lantai 5).
pub fn all_relics() -> Vec<&'static RelicItem> {
    let mut relics: Vec<&'static RelicItem> =
        floors().iter().map(|f| &f.boss.relic).collect();
    if let Some(f5) = floor(5) {
        relics.extend(f5.topics.iter().filter_map(|t| t.relic.as_ref()));
    }
    relics
}

/// Semua 5 badge boss lantai.
pub fn all_badges() -> Vec<&'static BadgeItem> {
    floors().iter().map(|f| &f.boss.badge).collect()
}
