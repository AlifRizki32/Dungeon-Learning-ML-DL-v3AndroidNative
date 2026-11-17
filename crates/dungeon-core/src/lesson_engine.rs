//! Lesson engine — port `src/game/lessonEngine.ts` v2.
//!
//! Pelajaran bertahap (chunk), latihan 4 jenis, tokenizer glosarium, dan
//! penjadwal review terjadwal ("Ulasan Harian"). Data chunk berasal dari
//! `dungeon-data` (JSON v2 apa adanya); engine di sini murni.

use crate::dates;
use crate::rng::Rng;
use dungeon_data::lessons::{LessonChunk, LessonTerm, MatchPair};
pub use dungeon_data::lessons::LessonReviewItem;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Progres satu chunk pelajaran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonChunkProgress {
    pub done: bool,
    pub attempts: u32,
    #[serde(rename = "attemptedActivities", default)]
    pub attempted_activities: Vec<String>,
}

impl Default for LessonChunkProgress {
    fn default() -> Self {
        Self {
            done: false,
            attempts: 0,
            attempted_activities: Vec::new(),
        }
    }
}

/// Entri antrean review satu kartu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewQueueEntry {
    #[serde(rename = "box")]
    pub box_: u8,
    pub due: String,
}

/// Rating pemain saat review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewRating {
    Lupa,
    Ragu,
    Ingat,
}

/// Kartu review dari katalog, diperkaya konteks chunk-nya.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewCatalogItem {
    pub item: &'static LessonReviewItem,
    pub chunk_id: &'static str,
    pub chunk_title: &'static str,
    pub topic_id: i64,
    pub quest_id: Option<&'static str>,
}

/// Kartu review yang jatuh tempo (siap ditampilkan di sesi review).
#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedReviewItem {
    pub id: &'static str,
    pub front: &'static str,
    pub back: &'static str,
    pub chunk_id: &'static str,
    pub chunk_title: &'static str,
    pub topic_id: i64,
    pub quest_id: Option<&'static str>,
    pub box_: u8,
    pub due: String,
}

/// Katalog chunk yang digabung dari semua file pelajaran — cerminan peta
/// `CHUNKS_BY_TOPIC_ID` / `CHUNKS_BY_QUEST_ID` / `CHUNKS_BY_ID` /
/// `REVIEW_ITEMS_BY_ID` di v2 (dibangun sekali saat modul dimuat).
pub struct LessonCatalog {
    pub chunks_by_topic: BTreeMap<u32, Vec<&'static LessonChunk>>,
    pub chunks_by_quest: BTreeMap<String, Vec<&'static LessonChunk>>,
    pub chunks_by_id: BTreeMap<&'static str, &'static LessonChunk>,
    pub review_items_by_id: BTreeMap<&'static str, ReviewCatalogItem>,
}

fn build_catalog() -> LessonCatalog {
    let mut catalog = LessonCatalog {
        chunks_by_topic: BTreeMap::new(),
        chunks_by_quest: BTreeMap::new(),
        chunks_by_id: BTreeMap::new(),
        review_items_by_id: BTreeMap::new(),
    };
    for file in dungeon_data::lessons::all_lesson_files() {
        for t in &file.topics {
            let entry = catalog.chunks_by_topic.entry(t.topic_id).or_default();
            for chunk in &t.chunks {
                entry.push(chunk);
                catalog.chunks_by_id.insert(chunk.chunk_id.as_str(), chunk);
                for item in &chunk.review_items {
                    catalog.review_items_by_id.insert(
                        item.id.as_str(),
                        ReviewCatalogItem {
                            item,
                            chunk_id: chunk.chunk_id.as_str(),
                            chunk_title: chunk.title.as_str(),
                            topic_id: t.topic_id as i64,
                            quest_id: None,
                        },
                    );
                }
            }
        }
        for q in &file.quests {
            let key = q.quest_id.trim().to_lowercase();
            let entry = catalog.chunks_by_quest.entry(key.clone()).or_default();
            let quest_num: i64 = key
                .trim_start_matches('m')
                .parse()
                .unwrap_or(0);
            for chunk in &q.chunks {
                entry.push(chunk);
                catalog.chunks_by_id.insert(chunk.chunk_id.as_str(), chunk);
                for item in &chunk.review_items {
                    catalog.review_items_by_id.insert(
                        item.id.as_str(),
                        ReviewCatalogItem {
                            item,
                            chunk_id: chunk.chunk_id.as_str(),
                            chunk_title: chunk.title.as_str(),
                            topic_id: 600 + quest_num,
                            quest_id: Some(q.quest_id.as_str()),
                        },
                    );
                }
            }
        }
    }
    catalog
}

static CATALOG: OnceLock<LessonCatalog> = OnceLock::new();

/// Katalog chunk global (dibangun sekali).
pub fn catalog() -> &'static LessonCatalog {
    CATALOG.get_or_init(build_catalog)
}

/// Mengurai id topik numerik: menerima angka, `"12"`, atau `"topic-12"`.
/// `None` untuk bentuk lain (mis. `"m1"` — itu quest, bukan topik).
pub fn parse_numeric_topic_id(topic_id: &str) -> Option<u32> {
    let s = topic_id.trim();
    if let Some(num) = s.strip_prefix("topic-").or_else(|| s.strip_prefix("TOPIC-")) {
        return num.parse().ok();
    }
    if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
        return s.parse().ok();
    }
    // Bentuk "Topic-12" (kapital T saja) — tangani tanpa case-insensitive penuh.
    let lower = s.to_lowercase();
    if let Some(num) = lower.strip_prefix("topic-") {
        return num.parse().ok();
    }
    None
}

/// Chunk pelajaran untuk satu quest (`"m1"`..`"m7"`, tanpa memperhatikan kapital).
pub fn get_lesson_chunks_for_quest(quest_id: &str) -> Vec<&'static LessonChunk> {
    if quest_id.is_empty() {
        return Vec::new();
    }
    let key = quest_id.trim().to_lowercase();
    catalog()
        .chunks_by_quest
        .get(&key)
        .cloned()
        .unwrap_or_default()
}

pub fn has_lesson_chunks_for_quest(quest_id: &str) -> bool {
    !get_lesson_chunks_for_quest(quest_id).is_empty()
}

pub fn get_lesson_chunk_by_id(chunk_id: &str) -> Option<&'static LessonChunk> {
    if chunk_id.is_empty() {
        return None;
    }
    catalog().chunks_by_id.get(chunk_id).copied()
}

/// Chunk pelajaran untuk satu topik (`"topic-1"`, `"1"`, `1`, atau quest `"m1"`).
pub fn get_lesson_chunks_for_topic(topic_id: &str) -> Vec<&'static LessonChunk> {
    let trimmed = topic_id.trim();
    let lower = trimmed.to_lowercase();
    if lower.len() >= 2 && lower.starts_with('m') && lower[1..].chars().all(|c| c.is_ascii_digit())
    {
        return get_lesson_chunks_for_quest(topic_id);
    }
    match parse_numeric_topic_id(topic_id) {
        Some(num) => catalog()
            .chunks_by_topic
            .get(&num)
            .cloned()
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

/// Chunk pelajaran untuk satu nomor topik (1-28).
pub fn get_lesson_chunks_for_topic_number(number: u32) -> Vec<&'static LessonChunk> {
    catalog()
        .chunks_by_topic
        .get(&number)
        .cloned()
        .unwrap_or_default()
}

pub fn has_lesson_chunks_for_topic(topic_id: &str) -> bool {
    !get_lesson_chunks_for_topic(topic_id).is_empty()
}

/// Semua 156 kartu review di katalog.
pub fn get_all_review_catalog_items() -> Vec<&'static ReviewCatalogItem> {
    catalog().review_items_by_id.values().collect()
}

pub fn get_review_catalog_item_by_id(id: &str) -> Option<&'static ReviewCatalogItem> {
    catalog().review_items_by_id.get(id)
}

/// Chunk selesai bila semua aktivitas latihannya pernah dicoba minimal sekali
/// (atau `progress.done` sudah true). Chunk tanpa latihan dianggap selesai
/// bila ada progresnya.
pub fn is_chunk_complete(
    chunk: &LessonChunk,
    progress_map: &BTreeMap<String, LessonChunkProgress>,
) -> bool {
    let prog = match progress_map.get(chunk.chunk_id.as_str()) {
        Some(p) => p,
        None => return false,
    };
    if prog.done {
        return true;
    }
    if chunk.practice.is_empty() {
        return true;
    }
    chunk
        .practice
        .iter()
        .all(|act| prog.attempted_activities.iter().any(|a| a == act.id()))
}

/// Semua chunk satu topik selesai (topik tanpa chunk dianggap selesai).
pub fn are_all_topic_chunks_complete(
    topic_id: &str,
    progress_map: &BTreeMap<String, LessonChunkProgress>,
) -> bool {
    let chunks = get_lesson_chunks_for_topic(topic_id);
    if chunks.is_empty() {
        return true;
    }
    chunks.iter().all(|c| is_chunk_complete(c, progress_map))
}

// ============================================================================
// BANTUAN LATIHAN (PRACTICE)
// ============================================================================

/// Satu opsi pilihan yang sudah diacak.
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffledChoiceOption {
    pub option_id: String,
    pub original_index: usize,
    pub text: String,
    pub is_correct: bool,
}

/// Mengacak opsi pilihan sambil menjaga kebenaran lewat identitas opsi.
pub fn shuffle_choice_options(
    options: &[String],
    correct_index: usize,
    rng: &mut dyn Rng,
) -> Vec<ShuffledChoiceOption> {
    let mut items: Vec<ShuffledChoiceOption> = options
        .iter()
        .enumerate()
        .map(|(idx, text)| ShuffledChoiceOption {
            option_id: format!("opt-{idx}"),
            original_index: idx,
            text: text.clone(),
            is_correct: idx == correct_index,
        })
        .collect();
    crate::rng::shuffle(&mut items, rng);
    items
}

fn is_leading_nonzero_digits(s: &str) -> Option<usize> {
    // Mencocokkan [1-9]\d{0,2} di awal: mengembalikan panjang awalan (1-3).
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if ('1'..='9').contains(&c) => {}
        _ => return None,
    }
    let mut len = 1;
    for c in chars.take(2) {
        if c.is_ascii_digit() {
            len += 1;
        } else {
            break;
        }
    }
    Some(len)
}

fn match_groups(rest: &str, sep: char, min_groups: usize) -> bool {
    // Mencocokkan (sep + tepat 3 digit) sebanyak >= min_groups, sampai habis.
    let mut s = rest;
    let mut groups = 0;
    loop {
        if s.len() < 4 || !s.starts_with(sep) {
            return false;
        }
        let digits = &s[1..4];
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        groups += 1;
        s = &s[4..];
        if s.is_empty() {
            return groups >= min_groups;
        }
    }
}

/// Mengurai angka dari masukan lokal: koma atau titik sebagai pemisah desimal,
/// minus di depan (`-` atau `−`), spasi di sekeliling, dan pemisah ribuan
/// yang tidak ambigu (mis. `"120.000"`, `"1.000.000"`, `"1.200,5"`).
/// Port `parseLocalizedNumber` v2 (tanpa regex — pola yang sama, dicek manual).
pub fn parse_localized_number(raw: &str) -> Option<f64> {
    let trimmed = raw.trim().replace('\u{2212}', "-");
    if trimmed.is_empty() {
        return None;
    }
    let (sign, rest) = match trimmed.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", trimmed.as_str()),
    };
    let lead_len = is_leading_nonzero_digits(rest).unwrap_or(0);
    let after_lead = &rest[lead_len..];
    let lead = &rest[..lead_len];

    let mut normalized: Option<String> = None;
    if lead_len > 0 {
        // 1. Keduanya ada: "120.000,5" atau "120,000.5".
        if let Some(idx) = after_lead.find(',') {
            let before = &after_lead[..idx];
            let after = &after_lead[idx + 1..];
            if !after.is_empty()
                && after.chars().all(|c| c.is_ascii_digit())
                && match_groups(before, '.', 1)
            {
                normalized = Some(format!("{sign}{lead}{}.{}", before.replace('.', ""), after));
            }
        }
        // 2. Kelompok ribuan koma lalu titik desimal: "120,000.5".
        if normalized.is_none() {
            if let Some(idx) = after_lead.find('.') {
                let before = &after_lead[..idx];
                let after = &after_lead[idx + 1..];
                if !after.is_empty()
                    && after.chars().all(|c| c.is_ascii_digit())
                    && match_groups(before, ',', 1)
                {
                    normalized =
                        Some(format!("{sign}{lead}{}.{}", before.replace(',', ""), after));
                }
            }
        }
        // 3. Beberapa kelompok ribuan titik: "1.000.000".
        if normalized.is_none() && match_groups(after_lead, '.', 2) {
            normalized = Some(trimmed.replace('.', ""));
        }
        // 4. Beberapa kelompok ribuan koma: "1,000,000".
        if normalized.is_none() && match_groups(after_lead, ',', 2) {
            normalized = Some(trimmed.replace(',', ""));
        }
        // 5. Satu pemisah ribuan ".000" / ",000": "120.000", "80.000".
        if normalized.is_none() && (after_lead == ".000" || after_lead == ",000") {
            normalized = Some(trimmed.replace(['.', ','], ""));
        }
    }
    let normalized = normalized.unwrap_or_else(|| trimmed.replace(',', "."));

    if !is_plain_number(&normalized) {
        return None;
    }
    let val: f64 = normalized.parse().ok()?;
    if val.is_finite() {
        Some(val)
    } else {
        None
    }
}

fn is_plain_number(s: &str) -> bool {
    // Pola ^-?\d+(\.\d+)?$
    let body = s.strip_prefix('-').unwrap_or(s);
    if body.is_empty() || !body.chars().next().unwrap().is_ascii_digit() {
        return false;
    }
    let mut parts = body.splitn(2, '.');
    let int_part = parts.next().unwrap();
    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    match parts.next() {
        None => true,
        Some(frac) => !frac.is_empty() && frac.chars().all(|c| c.is_ascii_digit()),
    }
}

/// Hasil evaluasi isian angka.
#[derive(Debug, Clone, PartialEq)]
pub struct FillNumberResult {
    pub valid: bool,
    pub numeric_value: Option<f64>,
    pub is_correct: bool,
}

pub fn evaluate_fill_number(raw_input: &str, answer: f64, tolerance: f64) -> FillNumberResult {
    let mut parsed = parse_localized_number(raw_input);
    // Bila jawaban bilangan bulat >= 1000 dan masukan satu kelompok ribuan
    // ("12.500" / "12,500"), tafsirkan sebagai ribuan.
    if answer.fract() == 0.0 && answer.abs() >= 1000.0 {
        let trimmed = raw_input.trim().replace('\u{2212}', "-");
        let body = trimmed.strip_prefix('-').unwrap_or(trimmed.as_str());
        if let Some(lead_len) = is_leading_nonzero_digits(body) {
            let after = &body[lead_len..];
            if after.len() == 4
                && (after.starts_with('.') || after.starts_with(','))
                && after[1..].chars().all(|c| c.is_ascii_digit())
            {
                let as_thousands: f64 = trimmed.replace(['.', ','], "").parse().unwrap_or(f64::NAN);
                if as_thousands.is_finite() {
                    parsed = Some(as_thousands);
                }
            }
        }
    }
    let parsed = match parsed {
        Some(v) => v,
        None => {
            return FillNumberResult {
                valid: false,
                numeric_value: None,
                is_correct: false,
            }
        }
    };
    let diff = (parsed - answer).abs();
    let tol = tolerance.max(0.0);
    FillNumberResult {
        valid: true,
        numeric_value: Some(parsed),
        is_correct: diff <= tol + 1e-9,
    }
}

fn format_id_number(value: f64, max_fraction_digits: usize) -> String {
    // Membulatkan ke max_fraction_digits lalu memformat gaya id-ID:
    // '.' ribuan, ',' desimal, tanpa nol di belakang.
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let scale = 10f64.powi(max_fraction_digits as i32);
    let rounded = (value * scale).round() / scale;
    if rounded == 0.0 {
        return "0".to_string();
    }
    let negative = rounded < 0.0;
    let abs = rounded.abs();
    let int_part = abs.trunc() as u64;
    let frac_part = ((abs - int_part as f64) * scale).round() as u64;
    // Kelompokkan ribuan dengan '.'.
    let int_str = int_part.to_string();
    let mut grouped = String::new();
    for (i, c) in int_str.chars().enumerate() {
        if i > 0 && (int_str.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&grouped);
    if frac_part > 0 {
        let frac_str = format!("{frac_part:0width$}", width = max_fraction_digits);
        let frac_trimmed = frac_str.trim_end_matches('0');
        if !frac_trimmed.is_empty() {
            out.push(',');
            out.push_str(frac_trimmed);
        }
    }
    out
}

/// Format nilai diagram batang: maks 3 desimal, pemisah ribuan '.' dan
/// desimal ',' (locale id-ID). Nol (atau -0) selalu `"0"`.
pub fn format_bar_value(value: f64) -> String {
    format_id_number(value, 3)
}

/// Format sel matriks: maks 3 desimal, tanpa pemisah ribuan, minus dipertahankan.
pub fn format_matrix_cell(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        return "0".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        return "0".to_string();
    }
    // String(Number(x.toFixed(3))): buang nol di belakang.
    let s = format!("{rounded:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Satu butir urutan yang sudah diacak.
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffledOrderItem {
    pub id: String,
    pub text: String,
    pub correct_index: usize,
}

/// Mengacak butir latihan `order`. Bila hasilnya identik dengan urutan benar
/// (dan ada >= 2 butir), dua butir pertama ditukar — sama seperti v2.
pub fn shuffle_order_items(items: &[String], rng: &mut dyn Rng) -> Vec<ShuffledOrderItem> {
    let mut tagged: Vec<ShuffledOrderItem> = items
        .iter()
        .enumerate()
        .map(|(idx, text)| ShuffledOrderItem {
            id: format!("ord-{idx}"),
            text: text.clone(),
            correct_index: idx,
        })
        .collect();
    if tagged.len() <= 1 {
        return tagged;
    }
    crate::rng::shuffle(&mut tagged, rng);
    let identical = tagged
        .iter()
        .enumerate()
        .all(|(idx, item)| item.correct_index == idx);
    if identical {
        tagged.swap(0, 1);
    }
    tagged
}

/// Hasil evaluasi latihan `order`.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderEvaluation {
    pub is_correct: bool,
    pub is_all_correct: bool,
    pub wrong_positions: Vec<usize>,
}

pub fn evaluate_order_attempt(
    built_sequence_texts: &[String],
    correct_items: &[String],
) -> OrderEvaluation {
    let mut wrong_positions = Vec::new();
    for (i, correct) in correct_items.iter().enumerate() {
        if built_sequence_texts.get(i) != Some(correct) {
            wrong_positions.push(i);
        }
    }
    let is_all_correct =
        built_sequence_texts.len() == correct_items.len() && wrong_positions.is_empty();
    OrderEvaluation {
        is_correct: is_all_correct,
        is_all_correct,
        wrong_positions,
    }
}

/// Satu opsi kanan latihan `match` yang sudah disiapkan.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedMatchRightOption {
    pub id: String,
    pub text: String,
    pub original_pair_index: usize,
}

/// Menyiapkan opsi kanan latihan `match` (diacak, tidak identik bila >= 2).
pub fn prepare_match_right_options(
    pairs: &[MatchPair],
    rng: &mut dyn Rng,
) -> Vec<PreparedMatchRightOption> {
    let mut right_list: Vec<PreparedMatchRightOption> = pairs
        .iter()
        .enumerate()
        .map(|(idx, p)| PreparedMatchRightOption {
            id: format!("right-{idx}"),
            text: p.right.clone(),
            original_pair_index: idx,
        })
        .collect();
    if right_list.len() <= 1 {
        return right_list;
    }
    crate::rng::shuffle(&mut right_list, rng);
    let identical = right_list
        .iter()
        .enumerate()
        .all(|(idx, item)| item.original_pair_index == idx);
    if identical {
        right_list.swap(0, 1);
    }
    right_list
}

/// Hasil evaluasi latihan `match`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchEvaluation {
    pub is_correct: bool,
    pub is_all_correct: bool,
    pub wrong_left_indices: Vec<usize>,
}

pub fn evaluate_match_attempt(
    connections_by_left_index: &BTreeMap<usize, String>,
    pairs: &[MatchPair],
) -> MatchEvaluation {
    let mut wrong_left_indices = Vec::new();
    for (i, pair) in pairs.iter().enumerate() {
        if connections_by_left_index.get(&i) != Some(&pair.right) {
            wrong_left_indices.push(i);
        }
    }
    let is_all_correct = wrong_left_indices.is_empty();
    MatchEvaluation {
        is_correct: is_all_correct,
        is_all_correct,
        wrong_left_indices,
    }
}

// ============================================================================
// TOKENIZER GLOSARIUM (TANPA HURUF BESAR-KECIL, KATA UTUH)
// ============================================================================

/// Jenis segmen teks hasil tokenisasi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlossarySegmentKind {
    Text,
    Term,
}

/// Satu segmen teks: teks biasa atau kemunculan istilah glosarium.
#[derive(Debug, Clone, PartialEq)]
pub struct GlossaryTextSegment {
    pub kind: GlossarySegmentKind,
    pub text: String,
    pub term_entry: Option<LessonTerm>,
}

fn is_word_char(c: char) -> bool {
    // Sama seperti [\p{L}\p{N}_] pada regex v2.
    c.is_alphanumeric() || c == '_'
}

/// Memecah teks menjadi segmen; kemunculan `terms` (tanpa huruf besar-kecil,
/// kata utuh) ditandai sebagai segmen istilah. Istilah dengan akhiran
/// kurung seperti "simpangan baku (σ)" juga cocok dengan kata dasarnya
/// "simpangan baku". Port `tokenizeTextWithGlossaryTerms` v2 (tanpa regex).
pub fn tokenize_text_with_glossary_terms(
    text: &str,
    terms: &[LessonTerm],
) -> Vec<GlossaryTextSegment> {
    if text.is_empty() || terms.is_empty() {
        return vec![GlossaryTextSegment {
            kind: GlossarySegmentKind::Text,
            text: text.to_string(),
            term_entry: None,
        }];
    }

    // Pola kandidat: istilah utuh + kata dasar sebelum '('.
    let mut patterns: Vec<(String, LessonTerm)> = Vec::new();
    for t in terms {
        let raw_term = t.term.trim();
        if raw_term.is_empty() {
            continue;
        }
        let lower_full = raw_term.to_lowercase();
        if !patterns.iter().any(|(p, _)| p == &lower_full) {
            patterns.push((lower_full, t.clone()));
        }
        if let Some(paren_idx) = raw_term.find('(') {
            if paren_idx > 0 {
                let base = raw_term[..paren_idx].trim();
                if base.chars().count() >= 2 {
                    let lower_base = base.to_lowercase();
                    if !patterns.iter().any(|(p, _)| p == &lower_base) {
                        patterns.push((lower_base, t.clone()));
                    }
                }
            }
        }
    }
    if patterns.is_empty() {
        return vec![GlossaryTextSegment {
            kind: GlossarySegmentKind::Text,
            text: text.to_string(),
            term_entry: None,
        }];
    }
    // Pola terpanjang dulu supaya "turunan parsial" cocok sebelum "turunan".
    patterns.sort_by(|a, b| {
        b.0.chars()
            .count()
            .cmp(&a.0.chars().count())
            .then(a.0.cmp(&b.0))
    });

    let text_lower = text.to_lowercase();
    let chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = text_lower.chars().collect();
    // Peta indeks karakter -> indeks byte (untuk mengiris string asli).
    let mut byte_idx: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    byte_idx.push(text.len());

    let mut segments: Vec<GlossaryTextSegment> = Vec::new();
    let mut i = 0usize; // indeks karakter
    let mut seg_start = 0usize; // awal segmen teks berjalan (indeks karakter)
    while i < chars.len() {
        let mut matched: Option<(usize, LessonTerm)> = None; // (panjang karakter, istilah)
        for (pat, entry) in &patterns {
            let pat_chars: Vec<char> = pat.chars().collect();
            let plen = pat_chars.len();
            if i + plen > lower_chars.len() {
                continue;
            }
            if lower_chars[i..i + plen] != pat_chars[..] {
                continue;
            }
            // Batas kata: karakter sebelum/sesudah bukan huruf/angka/'_'.
            let before_ok = i == 0 || !is_word_char(chars[i - 1]);
            let after_ok = i + plen >= chars.len() || !is_word_char(chars[i + plen]);
            if before_ok && after_ok {
                matched = Some((plen, entry.clone()));
                break;
            }
        }
        match matched {
            Some((plen, entry)) => {
                if i > seg_start {
                    segments.push(GlossaryTextSegment {
                        kind: GlossarySegmentKind::Text,
                        text: text[byte_idx[seg_start]..byte_idx[i]].to_string(),
                        term_entry: None,
                    });
                }
                segments.push(GlossaryTextSegment {
                    kind: GlossarySegmentKind::Term,
                    text: text[byte_idx[i]..byte_idx[i + plen]].to_string(),
                    term_entry: Some(entry),
                });
                i += plen;
                seg_start = i;
            }
            None => {
                i += 1;
            }
        }
    }
    if seg_start < chars.len() {
        segments.push(GlossaryTextSegment {
            kind: GlossarySegmentKind::Text,
            text: text[byte_idx[seg_start]..byte_idx[chars.len()]].to_string(),
            term_entry: None,
        });
    }
    if segments.is_empty() {
        segments.push(GlossaryTextSegment {
            kind: GlossarySegmentKind::Text,
            text: text.to_string(),
            term_entry: None,
        });
    }
    segments
}

// ============================================================================
// PENJADWAL REVIEW TERJADWAL ("ULASAN HARIAN")
// ============================================================================

/// Interval per kotak (hari): [1, 3, 7, 14, 30].
pub const REVIEW_INTERVALS_DAYS: [i64; 5] = [1, 3, 7, 14, 30];
pub const REVIEW_BOX_INTERVALS: [i64; 5] = REVIEW_INTERVALS_DAYS;
/// Skala interval bawaan.
pub const DEFAULT_INTERVAL_SCALE: f64 = 1.0;
/// Maksimal item per sesi review.
pub const MAX_REVIEW_SESSION_ITEMS: usize = 10;

/// Menjepit skala interval ke 0,5-2 (dibulatkan ke 1 desimal).
/// Nilai bukan angka memakai bawaan 1.
pub fn clamp_interval_scale(scale: f64) -> f64 {
    if !scale.is_finite() {
        return DEFAULT_INTERVAL_SCALE;
    }
    ((scale * 10.0).round() / 10.0).clamp(0.5, 2.0)
}

/// Hasil penjadwalan satu kartu review.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledReview {
    pub box_: u8,
    pub due: String,
    pub interval_days: i64,
}

/// Menjadwalkan ulang satu kartu review:
/// - "ingat": naik satu kotak (maks 4), jatuh tempo = hari ini + interval[kotak baru] * skala.
/// - "ragu": kotak tetap, jatuh tempo = hari ini + interval[kotak] * skala.
/// - "lupa": kembali ke kotak 0, jatuh tempo besok.
/// Interval minimal 1 hari. `today_key` dipasok pemanggil.
pub fn schedule_review_item(
    current: Option<&ReviewQueueEntry>,
    rating: ReviewRating,
    interval_scale: f64,
    today_key: &str,
) -> ScheduledReview {
    let safe_scale = clamp_interval_scale(interval_scale);
    let current_box: i64 = match current {
        Some(e) => (e.box_ as i64).clamp(0, 4),
        None => 0,
    };
    match rating {
        ReviewRating::Lupa => ScheduledReview {
            box_: 0,
            due: dates::add_days(today_key, 1, today_key),
            interval_days: 1,
        },
        ReviewRating::Ragu => {
            let raw_days = REVIEW_INTERVALS_DAYS[current_box as usize] as f64 * safe_scale;
            let days = (raw_days.round() as i64).max(1);
            ScheduledReview {
                box_: current_box as u8,
                due: dates::add_days(today_key, days, today_key),
                interval_days: days,
            }
        }
        ReviewRating::Ingat => {
            let new_box = (current_box + 1).min(4);
            let raw_days = REVIEW_INTERVALS_DAYS[new_box as usize] as f64 * safe_scale;
            let days = (raw_days.round() as i64).max(1);
            ScheduledReview {
                box_: new_box as u8,
                due: dates::add_days(today_key, days, today_key),
                interval_days: days,
            }
        }
    }
}

/// Hasil filter kartu review yang jatuh tempo.
#[derive(Debug, Clone, PartialEq)]
pub struct DueReviewItems {
    pub total_due_count: usize,
    pub due_items: Vec<EnrichedReviewItem>,
    pub session_items: Vec<EnrichedReviewItem>,
}

/// Semua kartu yang jatuh tempo (`due <= today`), diurutkan tanggal jatuh
/// tempo lalu kotak lalu id; sesi dibatasi `max_session_items` (bawaan 10).
pub fn get_due_review_items(
    review_queue: &BTreeMap<String, ReviewQueueEntry>,
    today_key: &str,
    max_session_items: usize,
) -> DueReviewItems {
    let mut due_items: Vec<EnrichedReviewItem> = Vec::new();
    for (item_id, entry) in review_queue {
        if entry.due.as_str() <= today_key {
            if let Some(cat) = get_review_catalog_item_by_id(item_id) {
                due_items.push(EnrichedReviewItem {
                    id: cat.item.id.as_str(),
                    front: cat.item.front.as_str(),
                    back: cat.item.back.as_str(),
                    chunk_id: cat.chunk_id,
                    chunk_title: cat.chunk_title,
                    topic_id: cat.topic_id,
                    quest_id: cat.quest_id,
                    box_: entry.box_.clamp(0, 4),
                    due: entry.due.clone(),
                });
            }
        }
    }
    due_items.sort_by(|a, b| {
        a.due
            .cmp(&b.due)
            .then(a.box_.cmp(&b.box_))
            .then(a.id.cmp(b.id))
    });
    let total_due_count = due_items.len();
    let session_items: Vec<EnrichedReviewItem> =
        due_items.iter().take(max_session_items).cloned().collect();
    DueReviewItems {
        total_due_count,
        due_items,
        session_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ConstRng;

    #[test]
    fn lesson_chunks_load_per_floor() {
        // Port lessonEngine.test.ts: chunk per lantai 22/22/23/13/29/19.
        let mut floor1_total = 0;
        for t in 1..=6 {
            assert!(has_lesson_chunks_for_topic(&t.to_string()));
            let chunks = get_lesson_chunks_for_topic(&t.to_string());
            assert!(chunks.len() >= 3, "Topik {t} minimal 3 chunk");
            floor1_total += chunks.len();
            for c in &chunks {
                assert_eq!(c.real_task, None);
            }
        }
        assert_eq!(floor1_total, 22);

        let mut floor2_total = 0;
        for t in 7..=11 {
            assert!(has_lesson_chunks_for_topic(&t.to_string()));
            let chunks = get_lesson_chunks_for_topic(&t.to_string());
            assert!(chunks.len() >= 3);
            floor2_total += chunks.len();
            for c in &chunks {
                assert_eq!(c.real_task, None);
            }
        }
        assert_eq!(floor2_total, 22);

        let mut floor3_total = 0;
        let mut minilabs_f3: Vec<(&str, &str)> = Vec::new();
        for t in 12..=17 {
            assert!(has_lesson_chunks_for_topic(&t.to_string()));
            let chunks = get_lesson_chunks_for_topic(&t.to_string());
            assert!(chunks.len() >= 3);
            floor3_total += chunks.len();
            for c in &chunks {
                assert_eq!(c.real_task, None);
                if let Some(m) = &c.mini_lab {
                    minilabs_f3.push((c.chunk_id.as_str(), m.as_str()));
                }
            }
        }
        assert_eq!(floor3_total, 23);
        assert_eq!(
            minilabs_f3,
            vec![("t12c1", "neuron"), ("t16c1", "softmax")]
        );

        let mut floor4_total = 0;
        let mut minilabs_f4: Vec<(&str, &str)> = Vec::new();
        for t in 18..=20 {
            assert!(has_lesson_chunks_for_topic(&t.to_string()));
            let chunks = get_lesson_chunks_for_topic(&t.to_string());
            assert!(chunks.len() >= 4);
            floor4_total += chunks.len();
            for c in &chunks {
                assert_eq!(c.real_task, None);
                if let Some(m) = &c.mini_lab {
                    minilabs_f4.push((c.chunk_id.as_str(), m.as_str()));
                }
            }
        }
        assert_eq!(floor4_total, 13);
        assert_eq!(
            minilabs_f4,
            vec![("t19c5", "quant"), ("t20c2", "latency")]
        );

        let mut floor5_total = 0;
        for t in 21..=28 {
            assert!(has_lesson_chunks_for_topic(&t.to_string()));
            let chunks = get_lesson_chunks_for_topic(&t.to_string());
            assert!(chunks.len() >= 3);
            floor5_total += chunks.len();
            for c in &chunks {
                assert!(c.real_task.as_ref().is_some_and(|s| !s.trim().is_empty()));
            }
        }
        assert_eq!(floor5_total, 29);

        let mut floor6_total = 0;
        for q in 1..=7 {
            let qid = format!("m{q}");
            assert!(has_lesson_chunks_for_quest(&qid));
            assert!(has_lesson_chunks_for_topic(&qid));
            let chunks = get_lesson_chunks_for_quest(&qid);
            assert!(chunks.len() >= 2, "Quest {qid} minimal 2 chunk");
            floor6_total += chunks.len();
            for c in &chunks {
                assert!(c.real_task.as_ref().is_some_and(|s| !s.trim().is_empty()));
                assert!(c.formula_title.as_ref().is_some_and(|s| !s.trim().is_empty()));
                assert_eq!(
                    get_lesson_chunk_by_id(&c.chunk_id).map(|x| x.chunk_id.as_str()),
                    Some(c.chunk_id.as_str())
                );
            }
        }
        assert_eq!(floor6_total, 19);

        assert_eq!(get_all_review_catalog_items().len(), 156);
        let f6_review = get_review_catalog_item_by_id("m1c1r1").unwrap();
        assert_eq!(f6_review.quest_id, Some("m1"));
    }

    #[test]
    fn chunk_completion_tracks_attempted_activities() {
        let t1_chunks = get_lesson_chunks_for_topic("1");
        let first = t1_chunks[0];
        let empty: BTreeMap<String, LessonChunkProgress> = BTreeMap::new();
        assert!(!is_chunk_complete(first, &empty));

        let partial: BTreeMap<String, LessonChunkProgress> = [(
            first.chunk_id.clone(),
            LessonChunkProgress {
                done: false,
                attempts: 1,
                attempted_activities: vec![first.practice[0].id().to_string()],
            },
        )]
        .into_iter()
        .collect();
        assert!(!is_chunk_complete(first, &partial));

        let full: BTreeMap<String, LessonChunkProgress> = [(
            first.chunk_id.clone(),
            LessonChunkProgress {
                done: false,
                attempts: first.practice.len() as u32,
                attempted_activities: first
                    .practice
                    .iter()
                    .map(|p| p.id().to_string())
                    .collect(),
            },
        )]
        .into_iter()
        .collect();
        assert!(is_chunk_complete(first, &full));
        assert!(!are_all_topic_chunks_complete("1", &full));

        let all_done: BTreeMap<String, LessonChunkProgress> = t1_chunks
            .iter()
            .map(|c| {
                (
                    c.chunk_id.clone(),
                    LessonChunkProgress {
                        done: true,
                        attempts: c.practice.len() as u32,
                        attempted_activities: c
                            .practice
                            .iter()
                            .map(|p| p.id().to_string())
                            .collect(),
                    },
                )
            })
            .collect();
        assert!(are_all_topic_chunks_complete("1", &all_done));
    }

    #[test]
    fn shuffle_choice_options_keeps_correctness_by_identity() {
        let options = vec![
            "Opsi A".to_string(),
            "Opsi B (Benar)".to_string(),
            "Opsi C".to_string(),
        ];
        let shuffled = shuffle_choice_options(&options, 1, &mut ConstRng(0.1));
        assert_eq!(shuffled.len(), 3);
        let correct = shuffled.iter().find(|o| o.is_correct).unwrap();
        assert_eq!(correct.text, "Opsi B (Benar)");
        assert_eq!(correct.option_id, "opt-1");
    }

    #[test]
    fn parse_localized_numbers_and_fill_number_tolerance() {
        assert_eq!(parse_localized_number("  2,5 "), Some(2.5));
        assert_eq!(parse_localized_number(" -1,25 "), Some(-1.25));
        assert_eq!(parse_localized_number(" −0.75 "), Some(-0.75));
        assert_eq!(parse_localized_number("abc"), None);

        assert!(evaluate_fill_number(" 17,5 ", 17.5, 0.01).is_correct);
        assert!(evaluate_fill_number("0,22", 0.222, 0.01).is_correct);
        assert!(!evaluate_fill_number("0,25", 0.222, 0.01).is_correct);

        assert!(evaluate_fill_number("1,385", 1.386, 0.002).is_correct);
        assert!(evaluate_fill_number("1.388", 1.386, 0.002).is_correct);
        assert!(!evaluate_fill_number("1,39", 1.386, 0.002).is_correct);
        assert!(evaluate_fill_number("0,18", 0.18, 0.005).is_correct);
        assert!(evaluate_fill_number("0,175", 0.18, 0.005).is_correct);
        assert!(evaluate_fill_number("-25", -25.0, 0.01).is_correct);
        assert!(evaluate_fill_number("−25", -25.0, 0.01).is_correct);
        assert!(evaluate_fill_number(" -18 ", -18.0, 0.0).is_correct);
        assert!(evaluate_fill_number(" −18 ", -18.0, 0.0).is_correct);
        assert!(evaluate_fill_number(" -12 ", -12.0, 0.0).is_correct);
        assert!(evaluate_fill_number(" −12 ", -12.0, 0.0).is_correct);
        assert!(evaluate_fill_number("0,05", 0.05, 0.0001).is_correct);
        assert!(evaluate_fill_number("0,001", 0.001, 0.00001).is_correct);
        assert!(evaluate_fill_number("1,39", 1.39, 0.01).is_correct);
        assert!(evaluate_fill_number("1.386", 1.39, 0.01).is_correct);
        assert!(evaluate_fill_number("120000", 120000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("120.000", 120000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("120,000", 120000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("80000", 80000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("80.000", 80000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("80,000", 80000.0, 0.0).is_correct);
        assert!(evaluate_fill_number("0,01", 0.01, 0.0001).is_correct);
        assert!(evaluate_fill_number("0.01", 0.01, 0.0001).is_correct);
        assert!(!evaluate_fill_number("0,011", 0.01, 0.0001).is_correct);
        assert!(evaluate_fill_number("0,06", 0.06, 0.001).is_correct);
        assert!(evaluate_fill_number("0.06", 0.06, 0.001).is_correct);
        assert!(evaluate_fill_number("0,4125", 0.4125, 0.001).is_correct);
        assert!(evaluate_fill_number("0.4125", 0.4125, 0.001).is_correct);
        assert!(evaluate_fill_number("0,412", 0.4125, 0.001).is_correct);
        assert!(evaluate_fill_number("21,5", 21.5, 0.001).is_correct);
        assert!(evaluate_fill_number("80,75", 80.75, 0.01).is_correct);
        assert!(evaluate_fill_number("2,5", 2.5, 0.001).is_correct);
        assert!(evaluate_fill_number("14400", 14400.0, 0.0).is_correct);
        assert!(evaluate_fill_number("14.400", 14400.0, 0.0).is_correct);
        assert!(evaluate_fill_number("14,400", 14400.0, 0.0).is_correct);
    }

    #[test]
    fn format_bar_values_and_matrix_cells() {
        assert_eq!(format_bar_value(0.0), "0");
        assert_eq!(format_bar_value(-0.0), "0");
        assert_eq!(format_bar_value(5.0), "5");
        assert_eq!(format_bar_value(100000.0), "100.000");
        assert_eq!(format_bar_value(200000.0), "200.000");
        assert_eq!(format_bar_value(50000.0), "50.000");
        assert_eq!(format_bar_value(0.105), "0,105");
        assert_eq!(format_bar_value(0.625), "0,625");
        assert_eq!(format_bar_value(1.3864), "1,386");
        assert_eq!(format_bar_value(0.09), "0,09");
        assert_eq!(format_bar_value(5.76), "5,76");

        assert_eq!(format_matrix_cell(-0.5), "-0.5");
        assert_eq!(format_matrix_cell(-0.05), "-0.05");
        assert_eq!(format_matrix_cell(0.7), "0.7");
        assert_eq!(format_matrix_cell(0.0), "0");
        assert_eq!(format_matrix_cell(-0.0), "0");
        assert_eq!(format_matrix_cell(3.0), "3");
    }

    #[test]
    fn order_and_match_activities_evaluate() {
        let items = vec![
            "Langkah 1".to_string(),
            "Langkah 2".to_string(),
            "Langkah 3".to_string(),
        ];
        let shuffled = shuffle_order_items(&items, &mut ConstRng(0.2));
        assert_eq!(shuffled.len(), 3);

        let ok = evaluate_order_attempt(&items, &items);
        assert!(ok.is_correct);
        assert!(ok.wrong_positions.is_empty());

        let wrong = evaluate_order_attempt(
            &[
                "Langkah 1".to_string(),
                "Langkah 3".to_string(),
                "Langkah 2".to_string(),
            ],
            &items,
        );
        assert!(!wrong.is_correct);
        assert_eq!(wrong.wrong_positions, vec![1, 2]);

        let pairs = vec![
            MatchPair {
                left: "Mean".to_string(),
                right: "Rata-rata".to_string(),
            },
            MatchPair {
                left: "Median".to_string(),
                right: "Nilai tengah".to_string(),
            },
        ];
        let right_opts = prepare_match_right_options(&pairs, &mut ConstRng(0.3));
        assert_eq!(right_opts.len(), 2);

        let mut conn_ok: BTreeMap<usize, String> = BTreeMap::new();
        conn_ok.insert(0, "Rata-rata".to_string());
        conn_ok.insert(1, "Nilai tengah".to_string());
        assert!(evaluate_match_attempt(&conn_ok, &pairs).is_correct);

        let mut conn_wrong: BTreeMap<usize, String> = BTreeMap::new();
        conn_wrong.insert(0, "Nilai tengah".to_string());
        conn_wrong.insert(1, "Rata-rata".to_string());
        let m = evaluate_match_attempt(&conn_wrong, &pairs);
        assert!(!m.is_correct);
        assert_eq!(m.wrong_left_indices, vec![0, 1]);
    }

    #[test]
    fn glossary_tokenizer_matches_whole_words() {
        let terms = vec![
            LessonTerm {
                term: "mean".to_string(),
                plain: "Rata-rata jumlah dibagi banyak data.".to_string(),
            },
            LessonTerm {
                term: "simpangan baku (σ)".to_string(),
                plain: "Akar dari varians.".to_string(),
            },
        ];
        let text = "Nilai Mean dan simpangan baku sangat penting dibandingkan meaningful.";
        let tokens = tokenize_text_with_glossary_terms(text, &terms);
        let matched: Vec<&GlossaryTextSegment> = tokens
            .iter()
            .filter(|t| t.term_entry.is_some())
            .collect();
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].text, "Mean");
        assert_eq!(matched[0].term_entry.as_ref().unwrap().term, "mean");
        assert_eq!(matched[1].text, "simpangan baku");
        assert_eq!(
            matched[1].term_entry.as_ref().unwrap().term,
            "simpangan baku (σ)"
        );
    }

    #[test]
    fn review_scheduling_for_ratings() {
        let today = "2026-11-02";
        let initial = ReviewQueueEntry {
            box_: 0,
            due: today.to_string(),
        };

        let ingat1 = schedule_review_item(Some(&initial), ReviewRating::Ingat, 1.0, today);
        assert_eq!(ingat1.box_, 1);
        assert_eq!(ingat1.interval_days, 3);
        assert_eq!(ingat1.due, "2026-11-05");

        let ingat2 = schedule_review_item(
            Some(&ReviewQueueEntry {
                box_: 1,
                due: today.to_string(),
            }),
            ReviewRating::Ingat,
            2.0,
            today,
        );
        assert_eq!(ingat2.box_, 2);
        assert_eq!(ingat2.interval_days, 14);
        assert_eq!(ingat2.due, "2026-11-16");

        let ragu = schedule_review_item(
            Some(&ReviewQueueEntry {
                box_: 2,
                due: today.to_string(),
            }),
            ReviewRating::Ragu,
            0.5,
            today,
        );
        assert_eq!(ragu.box_, 2);
        assert_eq!(ragu.interval_days, 4);
        assert_eq!(ragu.due, "2026-11-06");

        let lupa = schedule_review_item(
            Some(&ReviewQueueEntry {
                box_: 3,
                due: today.to_string(),
            }),
            ReviewRating::Lupa,
            1.5,
            today,
        );
        assert_eq!(lupa.box_, 0);
        assert_eq!(lupa.interval_days, 1);
        assert_eq!(lupa.due, "2026-11-03");

        // Filter jatuh tempo dibatasi 10 per sesi.
        let all_items = get_all_review_catalog_items();
        let first_id = all_items[0].item.id.clone();
        let second_id = all_items[1].item.id.clone();
        let mut queue: BTreeMap<String, ReviewQueueEntry> = BTreeMap::new();
        queue.insert(
            first_id.clone(),
            ReviewQueueEntry {
                box_: 0,
                due: "2026-11-02".to_string(),
            },
        );
        queue.insert(
            second_id,
            ReviewQueueEntry {
                box_: 1,
                due: "2026-11-05".to_string(),
            },
        );
        let due_info = get_due_review_items(&queue, today, 10);
        assert_eq!(due_info.total_due_count, 1);
        assert_eq!(due_info.due_items[0].id, first_id.as_str());
    }
}
