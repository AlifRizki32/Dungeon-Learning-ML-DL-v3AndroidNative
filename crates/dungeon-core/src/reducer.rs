//! Reducer — port aksi mutating `useGameStore.ts` v2.
//!
//! `Reducer::apply(state, action, ctx) -> (GameState, Vec<Effect>)`.
//! Murni dan deterministik: "hari ini" dan "sekarang" dipasok lewat
//! `ActionContext` (core tidak pernah membaca jam sistem); id sesi dan
//! `createdAt` dipasok aplikasi lewat aksi `LogStudySession`.
//!
//! State UI transien v2 (`currentView`, `selectedTopicId`, `levelUpModal`,
//! `newlyUnlockedIds`, dll.) TIDAK ada di sini: level-up dan ruangan yang
//! baru terbuka dikembalikan sebagai `Effect` untuk ditampilkan aplikasi.
//! `Effect::Persist` menandai state berubah dan perlu disimpan.

use crate::camp::{get_week_start_date_key, StudySession, StudySessionSource, WEEKLY_BONUS_XP};
use crate::formulas::{
    calculate_level_from_xp, calculate_room_xp, compute_next_streak, get_title_for_level,
    is_floor_unlocked, is_room_unlocked, BOSS_XP, TOWER_QUEST_XP,
};
use crate::lesson_engine::{
    get_lesson_chunk_by_id, get_lesson_chunks_for_topic, schedule_review_item,
    LessonChunkProgress, LessonReviewItem, ReviewRating,
};
use crate::spellbook::{
    calculate_question_weight, calculate_room_crit_bonus_xp, can_complete_tower_quest,
    can_tick_tower_step, get_next_boss_variant_index, get_topic_content,
    get_total_boss_phases, get_tower_quest_by_id, get_tower_relic_id, recover_boss_after_faint,
    update_question_stat, DOJO_CORRECT_XP, DOJO_DAILY_XP_CAP, ECHO_BONUS_XP,
};
use crate::state::{
    create_default_boss_progress_for_floor, import_save_json, BossProgressState, GameState,
};
use dungeon_data::roadmap::floor as roadmap_floor;

/// Konteks pemanggilan: tanggal lokal hari ini + waktu ISO sekarang.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionContext {
    /// Kunci tanggal lokal `YYYY-MM-DD`.
    pub today_key: String,
    /// Stempel waktu ISO 8601 (untuk `createdAt`/`issuedAt`/`exportedAt`).
    pub now_iso: String,
}

/// Semua aksi yang mengubah state persisten.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    StartAdventure {
        name: String,
        avatar_id: String,
    },
    ContinueAdventure,
    /// Pilih lantai aktif (hanya bila terbuka; lantai 6 selalu bisa dipilih).
    SelectFloor {
        floor_id: u32,
    },
    /// Pindah penanda node aktif satu lantai (dipakai saat membuka
    /// pertempuran ruangan/boss; tanpa validasi, sama seperti v2).
    SetActiveNode {
        floor_id: u32,
        node_id: String,
    },
    SetEvidenceNote {
        topic_id: String,
        note: String,
    },
    RecordQuizAttempt {
        topic_id: String,
        score_percent: i64,
    },
    RecordQuestionAnswer {
        question_id: String,
        is_correct: bool,
    },
    AwardDojoAnswerXp {
        question_id: String,
        is_correct: bool,
    },
    DefeatRoom {
        topic_id: String,
        correct_quiz_answers: i64,
        quiz_score_percent: Option<i64>,
        echo_correct: bool,
    },
    UseBossPotion {
        floor_id: u32,
    },
    AdvanceBossPhaseVariant {
        floor_id: u32,
        phase: u32,
    },
    CompleteBossPhase {
        floor_id: u32,
        phase: u32,
    },
    FaintBossFight {
        floor_id: u32,
    },
    ResetBossFightProgress {
        floor_id: Option<u32>,
    },
    DefeatBoss {
        floor_id: u32,
    },
    ToggleTowerStep {
        quest_id: String,
        step_id: String,
    },
    SetSpecialization {
        specialization: Option<String>,
    },
    CompleteTowerQuest {
        quest_id: String,
    },
    SetReflectionAnswer {
        prompt_id: String,
        answer: String,
    },
    IssueCertificate,
    ToggleTowerQuest {
        quest_id: String,
    },
    LogStudySession {
        session_id: String,
        created_at_ms: i64,
        topic_id: String,
        duration_minutes: i64,
        date: String,
        note: Option<String>,
        source: StudySessionSource,
    },
    DeleteStudySession {
        session_id: String,
    },
    SetWeeklyTargetHours {
        hours: f64,
    },
    ToggleSound,
    SetSfxVolume {
        volume: f64,
    },
    ToggleMusic,
    SetMusicEnabled {
        enabled: bool,
    },
    SetMusicVolume {
        volume: f64,
    },
    ToggleReduceMotion,
    RecordChunkPracticeAttempt {
        chunk_id: String,
        activity_id: String,
        total_practice_ids: Option<Vec<String>>,
    },
    CompleteLessonChunk {
        chunk_id: String,
        review_items: Vec<LessonReviewItem>,
    },
    SetChunkSummary {
        chunk_id: String,
        summary: String,
    },
    ToggleChunkRealTaskDone {
        chunk_id: String,
        done: Option<bool>,
    },
    GradeReviewItem {
        item_id: String,
        rating: ReviewRating,
    },
    SetIntervalScale {
        scale: f64,
    },
    ResetProgress,
    ImportSave {
        raw_json: String,
    },
}

/// Hasil samping aksi yang perlu diketahui aplikasi.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// State berubah — simpan ke penyimpanan.
    Persist,
    /// Naik level (aplikasi menampilkan modal level-up).
    LevelUp {
        old_level: i64,
        new_level: i64,
        new_title: String,
    },
    RoomDefeated {
        xp_gained: i64,
        base_xp_gained: i64,
        bonus_xp_gained: i64,
        echo_xp_gained: i64,
        unlocked_spell_id: Option<String>,
        newly_unlocked_ids: Vec<String>,
        new_level: i64,
    },
    BossDefeated {
        xp_gained: i64,
        new_level: i64,
    },
    TowerQuestCompleted {
        xp_gained: i64,
        relic_name: Option<String>,
        relic_id: Option<String>,
        new_level: i64,
    },
    TowerQuestToggled {
        xp_gained: i64,
        new_level: i64,
    },
    DojoAnswer {
        xp_awarded: i64,
        daily_total: i64,
        new_weight: i64,
        new_level: i64,
    },
    QuestionAnswered {
        question_id: String,
        seen: u32,
        wrong: u32,
        correct_streak: u32,
    },
    StudySessionLogged {
        session_id: String,
        bonus_xp_awarded: i64,
        new_level: i64,
    },
    WeeklyTargetUpdated {
        bonus_xp_awarded: i64,
        new_level: i64,
    },
    ReviewGraded {
        item_id: String,
        box_: u8,
        due: String,
    },
    CertificateIssued {
        issued_at: String,
        already_issued: bool,
    },
    BossPotionUsed {
        floor_id: u32,
        potions_left: u32,
    },
    BossPotionDenied {
        floor_id: u32,
    },
    BossVariantAdvanced {
        floor_id: u32,
        phase: u32,
        variant_index: u8,
    },
    TowerStepToggled {
        quest_id: String,
        step_id: String,
        ticked: bool,
    },
    TowerStepDenied {
        quest_id: String,
        step_id: String,
    },
    ImportSucceeded,
    ImportFailed {
        message: String,
    },
}

fn level_up_effect(old_xp: i64, new_xp: i64, effects: &mut Vec<Effect>) -> (bool, i64) {
    let old_level = calculate_level_from_xp(old_xp);
    let new_level = calculate_level_from_xp(new_xp);
    if new_level > old_level {
        effects.push(Effect::LevelUp {
            old_level,
            new_level,
            new_title: get_title_for_level(new_level).to_string(),
        });
        (true, new_level)
    } else {
        (false, new_level)
    }
}

fn next_streak(state: &GameState, today_key: &str) -> (i64, Option<String>) {
    let (streak, date) = compute_next_streak(
        state.streak,
        state.last_active_date.as_deref(),
        today_key,
    );
    (streak, Some(date))
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn push_if_absent(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|x| x == item) {
        list.push(item.to_string());
    }
}

/// Reducer murni game.
pub struct Reducer;

impl Reducer {
    pub fn apply(
        mut state: GameState,
        action: Action,
        ctx: &ActionContext,
    ) -> (GameState, Vec<Effect>) {
        let today = ctx.today_key.as_str();
        let mut effects: Vec<Effect> = Vec::new();

        match action {
            Action::StartAdventure { name, avatar_id } => {
                let clean_name = {
                    let t = name.trim();
                    if t.is_empty() {
                        "Petualang Tensor".to_string()
                    } else {
                        t.to_string()
                    }
                };
                state.has_started = true;
                state.player_name = clean_name;
                state.avatar_id = if avatar_id.trim().is_empty() {
                    "alchemist".to_string()
                } else {
                    avatar_id
                };
                effects.push(Effect::Persist);
            }
            Action::ContinueAdventure => {
                state.has_started = true;
                effects.push(Effect::Persist);
            }
            Action::SelectFloor { floor_id } => {
                if floor_id == 6 || is_floor_unlocked(floor_id, &state.defeated_bosses) {
                    state.active_floor_id = floor_id;
                    effects.push(Effect::Persist);
                }
            }
            Action::SetActiveNode { floor_id, node_id } => {
                state.active_node_by_floor.insert(floor_id, node_id);
                effects.push(Effect::Persist);
            }
            Action::SetEvidenceNote { topic_id, note } => {
                let safe_note = truncate_chars(&note, 300);
                // v2 menulis kunci ganda: "21" <-> "topic-21".
                let mut keys = vec![topic_id.clone()];
                if topic_id
                    .strip_prefix("topic-")
                    .map(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                    .unwrap_or(false)
                {
                    keys.push(topic_id.trim_start_matches("topic-").to_string());
                } else if !topic_id.is_empty()
                    && topic_id.bytes().all(|b| b.is_ascii_digit())
                {
                    keys.push(format!("topic-{topic_id}"));
                }
                for k in keys {
                    state.evidence_notes.insert(k, safe_note.clone());
                }
                effects.push(Effect::Persist);
            }
            Action::RecordQuizAttempt {
                topic_id,
                score_percent,
            } => {
                let prev_best = state
                    .best_score_by_topic
                    .get(&topic_id)
                    .copied()
                    .or_else(|| state.quiz_best_scores.get(&topic_id).copied());
                let next_best = match prev_best {
                    Some(p) => p.max(score_percent),
                    None => score_percent,
                };
                state
                    .quiz_best_scores
                    .insert(topic_id.clone(), next_best);
                state
                    .best_score_by_topic
                    .insert(topic_id, next_best);
                effects.push(Effect::Persist);
            }
            Action::RecordQuestionAnswer {
                question_id,
                is_correct,
            } => {
                let prev = state.question_stats.get(&question_id).copied();
                let next_stat = update_question_stat(prev.as_ref(), is_correct);
                state.question_stats.insert(question_id.clone(), next_stat);
                effects.push(Effect::Persist);
                effects.push(Effect::QuestionAnswered {
                    question_id,
                    seen: next_stat.seen,
                    wrong: next_stat.wrong,
                    correct_streak: next_stat.correct_streak,
                });
            }
            Action::AwardDojoAnswerXp {
                question_id,
                is_correct,
            } => {
                let prev = state.question_stats.get(&question_id).copied();
                let next_stat = update_question_stat(prev.as_ref(), is_correct);
                let new_weight = calculate_question_weight(Some(&next_stat));
                let current_daily = if state.dojo_daily_xp.date == today {
                    state.dojo_daily_xp.xp
                } else {
                    0
                };
                let mut xp_awarded = 0i64;
                if is_correct && current_daily < DOJO_DAILY_XP_CAP {
                    xp_awarded = (DOJO_CORRECT_XP).min(DOJO_DAILY_XP_CAP - current_daily);
                }
                let next_daily_total = current_daily + xp_awarded;
                let old_xp = state.xp;
                let new_xp = state.xp + xp_awarded;
                let (streak, last_active) = if xp_awarded > 0 {
                    next_streak(&state, today)
                } else {
                    (state.streak, state.last_active_date.clone())
                };
                state.question_stats.insert(question_id, next_stat);
                state.dojo_daily_xp.date = today.to_string();
                state.dojo_daily_xp.xp = next_daily_total;
                state.xp = new_xp;
                state.streak = streak;
                state.last_active_date = last_active;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::DojoAnswer {
                    xp_awarded,
                    daily_total: next_daily_total,
                    new_weight,
                    new_level,
                });
            }
            Action::DefeatRoom {
                topic_id,
                correct_quiz_answers,
                quiz_score_percent,
                echo_correct,
            } => {
                let found = (1..=5u32).find_map(|f_id| {
                    let f = roadmap_floor(f_id)?;
                    let t = f.topics.iter().find(|t| t.id == topic_id)?;
                    Some((f.clone(), t.clone()))
                });
                let Some((floor, topic)) = found else {
                    effects.push(Effect::RoomDefeated {
                        xp_gained: 0,
                        base_xp_gained: 0,
                        bonus_xp_gained: 0,
                        echo_xp_gained: 0,
                        unlocked_spell_id: None,
                        newly_unlocked_ids: Vec::new(),
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                };
                let already_completed = state.completed_rooms.contains(&topic_id);
                let already_claimed_bonus =
                    state.quiz_bonus_claimed_rooms.contains(&topic_id);
                let already_claimed_echo =
                    state.echo_bonus_claimed_rooms.contains(&topic_id);

                let base_room_xp = calculate_room_xp(topic.weeks.min, topic.weeks.max);
                let base_xp_gained = if already_completed { 0 } else { base_room_xp };
                let bonus_xp_gained = if correct_quiz_answers > 0 {
                    calculate_room_crit_bonus_xp(base_room_xp, already_claimed_bonus)
                } else {
                    0
                };
                let echo_xp_gained = if echo_correct && !already_claimed_echo {
                    ECHO_BONUS_XP
                } else {
                    0
                };
                let xp_gained = base_xp_gained + bonus_xp_gained + echo_xp_gained;
                let old_xp = state.xp;
                let new_xp = state.xp + xp_gained;

                let mut next_completed_rooms = state.completed_rooms.clone();
                if !already_completed {
                    next_completed_rooms.push(topic_id.clone());
                }
                if correct_quiz_answers > 0 && !already_claimed_bonus {
                    state.quiz_bonus_claimed_rooms.push(topic_id.clone());
                }
                if echo_correct && !already_claimed_echo {
                    state.echo_bonus_claimed_rooms.push(topic_id.clone());
                }

                let unlocked_spell_id =
                    get_topic_content(&topic_id).map(|c| c.topic.spell.id.clone());
                if let Some(spell_id) = &unlocked_spell_id {
                    push_if_absent(&mut state.spells_unlocked, spell_id);
                }

                // Quirk v2: tanpa skor kuis, kedua peta skor menjadi
                // bestScoreByTopic (entri khusus quizBestScores hilang).
                let prev_best = state
                    .best_score_by_topic
                    .get(&topic_id)
                    .copied()
                    .or_else(|| state.quiz_best_scores.get(&topic_id).copied());
                let next_best_score = quiz_score_percent.map(|q| match prev_best {
                    Some(p) => p.max(q),
                    None => q,
                });
                if let Some(next_best) = next_best_score {
                    let mut merged = state.quiz_best_scores.clone();
                    merged.extend(state.best_score_by_topic.clone());
                    merged.insert(topic_id.clone(), next_best);
                    state.quiz_best_scores = merged.clone();
                    state.best_score_by_topic = merged;
                } else {
                    state.quiz_best_scores = state.best_score_by_topic.clone();
                }

                // Ruangan/boss yang baru terbuka di lantai ini.
                let mut newly_unlocked: Vec<String> = Vec::new();
                for t in &floor.topics {
                    let was = is_room_unlocked(
                        t,
                        &floor.topics,
                        &state.completed_rooms,
                        &state.defeated_bosses,
                    );
                    let now = is_room_unlocked(
                        t,
                        &floor.topics,
                        &next_completed_rooms,
                        &state.defeated_bosses,
                    );
                    if !was && now {
                        newly_unlocked.push(t.id.clone());
                    }
                }
                let boss_was_unlocked = floor
                    .topics
                    .iter()
                    .all(|t| state.completed_rooms.contains(&t.id));
                let boss_now_unlocked = floor
                    .topics
                    .iter()
                    .all(|t| next_completed_rooms.contains(&t.id));
                if !boss_was_unlocked && boss_now_unlocked {
                    newly_unlocked.push(floor.boss.id.clone());
                }

                if let Some(relic) = &topic.relic {
                    push_if_absent(&mut state.relics, &relic.id);
                }

                let (streak, last_active) = next_streak(&state, today);
                let next_target = floor.topics.iter().find(|t| {
                    !next_completed_rooms.contains(&t.id)
                        && is_room_unlocked(
                            t,
                            &floor.topics,
                            &next_completed_rooms,
                            &state.defeated_bosses,
                        )
                });
                let next_node_id = match next_target {
                    Some(t) => t.id.clone(),
                    None => {
                        if boss_now_unlocked {
                            floor.boss.id.clone()
                        } else {
                            topic_id.clone()
                        }
                    }
                };

                state.completed_rooms = next_completed_rooms;
                state.xp = new_xp;
                state.streak = streak;
                state.last_active_date = last_active;
                state
                    .active_node_by_floor
                    .insert(floor.id, next_node_id);
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::RoomDefeated {
                    xp_gained,
                    base_xp_gained,
                    bonus_xp_gained,
                    echo_xp_gained,
                    unlocked_spell_id,
                    newly_unlocked_ids: newly_unlocked,
                    new_level,
                });
            }
            Action::UseBossPotion { floor_id } => {
                let fp = state
                    .boss_progress
                    .get(&floor_id)
                    .cloned()
                    .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
                if fp.potions == 0 {
                    effects.push(Effect::BossPotionDenied { floor_id });
                } else {
                    let mut next = fp.clone();
                    next.potions -= 1;
                    state.boss_progress.insert(floor_id, next.clone());
                    effects.push(Effect::Persist);
                    effects.push(Effect::BossPotionUsed {
                        floor_id,
                        potions_left: next.potions,
                    });
                }
            }
            Action::AdvanceBossPhaseVariant { floor_id, phase } => {
                let fp = state
                    .boss_progress
                    .get(&floor_id)
                    .cloned()
                    .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
                let prev_idx = fp.last_variant_by_phase.get(&phase).copied();
                let next_idx = get_next_boss_variant_index(prev_idx);
                let mut next = fp.clone();
                next.last_variant_by_phase.insert(phase, next_idx);
                state.boss_progress.insert(floor_id, next);
                effects.push(Effect::Persist);
                effects.push(Effect::BossVariantAdvanced {
                    floor_id,
                    phase,
                    variant_index: next_idx,
                });
            }
            Action::CompleteBossPhase { floor_id, phase } => {
                let fp = state
                    .boss_progress
                    .get(&floor_id)
                    .cloned()
                    .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
                if !fp.cleared_phases.contains(&phase) {
                    let mut next = fp.clone();
                    next.cleared_phases.push(phase);
                    state.boss_progress.insert(floor_id, next);
                    effects.push(Effect::Persist);
                }
            }
            Action::FaintBossFight { floor_id } => {
                let fp = state
                    .boss_progress
                    .get(&floor_id)
                    .cloned()
                    .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
                let mut next = fp.clone();
                next.cleared_phases = recover_boss_after_faint(&fp.cleared_phases);
                state.boss_progress.insert(floor_id, next);
                effects.push(Effect::Persist);
            }
            Action::ResetBossFightProgress { floor_id } => {
                match floor_id {
                    Some(f) => {
                        state
                            .boss_progress
                            .insert(f, create_default_boss_progress_for_floor(f));
                    }
                    None => {
                        state.boss_progress = (1..=5u32)
                            .map(|f| (f, create_default_boss_progress_for_floor(f)))
                            .collect();
                    }
                }
                effects.push(Effect::Persist);
            }
            Action::DefeatBoss { floor_id } => {
                let Some(floor) = roadmap_floor(floor_id) else {
                    effects.push(Effect::BossDefeated {
                        xp_gained: 0,
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                };
                let boss_id = floor.boss.id.clone();
                let already_defeated = state.defeated_bosses.contains(&boss_id);
                let xp_gained = if already_defeated { 0 } else { BOSS_XP };
                let old_xp = state.xp;
                let new_xp = state.xp + xp_gained;
                if !already_defeated {
                    state.defeated_bosses.push(boss_id);
                }
                push_if_absent(&mut state.badges, &floor.boss.badge.id);
                push_if_absent(&mut state.relics, &floor.boss.relic.id);
                let (streak, last_active) = next_streak(&state, today);
                state.streak = streak;
                state.last_active_date = last_active;
                if (1..=5).contains(&floor_id) {
                    let fp = state
                        .boss_progress
                        .get(&floor_id)
                        .cloned()
                        .unwrap_or_else(|| create_default_boss_progress_for_floor(floor_id));
                    let total_phases = get_total_boss_phases(floor_id);
                    let mut next: BossProgressState = fp;
                    next.cleared_phases = (1..=total_phases as u32).collect();
                    state.boss_progress.insert(floor_id, next);
                }
                state.xp = new_xp;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::BossDefeated {
                    xp_gained,
                    new_level,
                });
            }
            Action::ToggleTowerStep { quest_id, step_id } => {
                let denied = |effects: &mut Vec<Effect>| {
                    effects.push(Effect::TowerStepDenied {
                        quest_id: quest_id.clone(),
                        step_id: step_id.clone(),
                    });
                };
                if state.tower_completed.contains(&quest_id) {
                    denied(&mut effects);
                    return (state, effects);
                }
                let Some(quest) = get_tower_quest_by_id(&quest_id) else {
                    denied(&mut effects);
                    return (state, effects);
                };
                if !quest.steps.iter().any(|s| s.id == step_id) {
                    denied(&mut effects);
                    return (state, effects);
                }
                let current = state
                    .tower_steps
                    .get(&quest_id)
                    .and_then(|m| m.get(&step_id))
                    .copied()
                    .unwrap_or(false);
                if !current
                    && !can_tick_tower_step(&quest_id, &step_id, state.specialization.as_deref())
                {
                    denied(&mut effects);
                    return (state, effects);
                }
                let entry = state.tower_steps.entry(quest_id.clone()).or_default();
                if current {
                    entry.remove(&step_id);
                } else {
                    entry.insert(step_id.clone(), true);
                }
                effects.push(Effect::Persist);
                effects.push(Effect::TowerStepToggled {
                    quest_id,
                    step_id,
                    ticked: !current,
                });
            }
            Action::SetSpecialization { specialization } => {
                let clean = match &specialization {
                    Some(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
                    _ => None,
                };
                if clean.is_none()
                    && !state.tower_completed.contains(&"m6".to_string())
                    && state
                        .tower_steps
                        .get("m6")
                        .and_then(|m| m.get("m6s3"))
                        .copied()
                        .unwrap_or(false)
                {
                    if let Some(m6) = state.tower_steps.get_mut("m6") {
                        m6.remove("m6s3");
                    }
                }
                state.specialization = clean;
                effects.push(Effect::Persist);
            }
            Action::CompleteTowerQuest { quest_id } => {
                let Some(quest) = get_tower_quest_by_id(&quest_id) else {
                    effects.push(Effect::TowerQuestCompleted {
                        xp_gained: 0,
                        relic_name: None,
                        relic_id: None,
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                };
                if !can_complete_tower_quest(
                    &quest_id,
                    &state.tower_steps,
                    &state.tower_completed,
                    state.specialization.as_deref(),
                ) {
                    effects.push(Effect::TowerQuestCompleted {
                        xp_gained: 0,
                        relic_name: None,
                        relic_id: None,
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                }
                let xp_gained = if quest.xp == 0 {
                    TOWER_QUEST_XP
                } else {
                    quest.xp as i64
                };
                let old_xp = state.xp;
                let new_xp = state.xp + xp_gained;
                let (streak, last_active) = next_streak(&state, today);
                let relic_id = get_tower_relic_id(&quest.id);
                push_if_absent(&mut state.relics, &relic_id);
                if !state.tower_completed.contains(&quest.id) {
                    state.tower_completed.push(quest.id.clone());
                }
                if !state.completed_tower_quests.contains(&quest.id) {
                    state.completed_tower_quests.push(quest.id.clone());
                }
                state.xp = new_xp;
                state.streak = streak;
                state.last_active_date = last_active;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::TowerQuestCompleted {
                    xp_gained,
                    relic_name: Some(quest.relic.clone()),
                    relic_id: Some(relic_id),
                    new_level,
                });
            }
            Action::SetReflectionAnswer { prompt_id, answer } => {
                let safe = truncate_chars(&answer, 500);
                state.reflections.insert(prompt_id, safe);
                effects.push(Effect::Persist);
            }
            Action::IssueCertificate => {
                if let Some(existing) = state.certificate_issued_at.clone() {
                    effects.push(Effect::CertificateIssued {
                        issued_at: existing,
                        already_issued: true,
                    });
                } else {
                    let now_iso = ctx.now_iso.clone();
                    state.certificate_issued_at = Some(now_iso.clone());
                    effects.push(Effect::Persist);
                    effects.push(Effect::CertificateIssued {
                        issued_at: now_iso,
                        already_issued: false,
                    });
                }
            }
            Action::ToggleTowerQuest { quest_id } => {
                let normalized_id = if quest_id.starts_with("tower-") {
                    let num = quest_id.trim_start_matches("tower-");
                    if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) {
                        format!("m{num}")
                    } else {
                        quest_id.clone()
                    }
                } else {
                    quest_id.clone()
                };
                let quest = get_tower_quest_by_id(&normalized_id).or_else(|| {
                    // Fallback v2: cari di daftar quest Menara dengan id mentah.
                    dungeon_data::content::floor6_content()
                        .quests
                        .iter()
                        .find(|q| q.id == quest_id)
                });
                let Some(_quest) = quest else {
                    effects.push(Effect::TowerQuestToggled {
                        xp_gained: 0,
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                };
                let already_done = state.tower_completed.contains(&normalized_id)
                    || state.completed_tower_quests.contains(&quest_id);
                if already_done {
                    effects.push(Effect::TowerQuestToggled {
                        xp_gained: 0,
                        new_level: calculate_level_from_xp(state.xp),
                    });
                    return (state, effects);
                }
                let xp_gained = TOWER_QUEST_XP;
                let old_xp = state.xp;
                let new_xp = state.xp + xp_gained;
                let (streak, last_active) = next_streak(&state, today);
                let next_completed = {
                    let mut v = state.tower_completed.clone();
                    v.push(normalized_id.clone());
                    v
                };
                let relic_id = get_tower_relic_id(&normalized_id);
                push_if_absent(&mut state.relics, &relic_id);
                state.completed_tower_quests = next_completed.clone();
                state.tower_completed = next_completed;
                state.xp = new_xp;
                state.streak = streak;
                state.last_active_date = last_active;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::TowerQuestToggled {
                    xp_gained,
                    new_level,
                });
            }
            Action::LogStudySession {
                session_id,
                created_at_ms,
                topic_id,
                duration_minutes,
                date,
                note,
                source,
            } => {
                let duration = duration_minutes.clamp(1, 1440);
                let date_key = if date.trim().is_empty() {
                    today.to_string()
                } else {
                    date
                };
                let session = StudySession {
                    id: session_id.clone(),
                    topic_id: if topic_id.trim().is_empty() {
                        "topic-1".to_string()
                    } else {
                        topic_id
                    },
                    duration_minutes: duration as f64,
                    date: date_key.clone(),
                    note: note
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    source,
                    created_at: created_at_ms,
                };
                let mut next_sessions = vec![session];
                next_sessions.extend(state.study_sessions.iter().cloned());
                let session_week_key = get_week_start_date_key(&date_key, today);
                let week_minutes: f64 = next_sessions
                    .iter()
                    .filter(|s| get_week_start_date_key(&s.date, today) == session_week_key)
                    .map(|s| s.duration_minutes)
                    .sum();
                let week_hours = week_minutes / 60.0;
                let already_claimed = state
                    .claimed_weekly_bonus_weeks
                    .contains(&session_week_key);
                let meets_target = week_hours >= state.weekly_target_hours;
                let bonus_xp_awarded =
                    if !already_claimed && meets_target { WEEKLY_BONUS_XP } else { 0 };
                if bonus_xp_awarded > 0 {
                    state
                        .claimed_weekly_bonus_weeks
                        .push(session_week_key);
                }
                let old_xp = state.xp;
                let new_xp = state.xp + bonus_xp_awarded;
                let (streak, last_active) = if date_key == today {
                    next_streak(&state, today)
                } else {
                    (state.streak, state.last_active_date.clone())
                };
                state.study_sessions = next_sessions;
                state.xp = new_xp;
                state.streak = streak;
                state.last_active_date = last_active;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::StudySessionLogged {
                    session_id,
                    bonus_xp_awarded,
                    new_level,
                });
            }
            Action::DeleteStudySession { session_id } => {
                let before = state.study_sessions.len();
                state
                    .study_sessions
                    .retain(|s| s.id != session_id);
                if state.study_sessions.len() != before {
                    effects.push(Effect::Persist);
                }
            }
            Action::SetWeeklyTargetHours { hours } => {
                let safe_hours = ((hours * 10.0).round() / 10.0).clamp(1.0, 60.0);
                let current_week_key = get_week_start_date_key(today, today);
                let current_week_minutes: f64 = state
                    .study_sessions
                    .iter()
                    .filter(|s| get_week_start_date_key(&s.date, today) == current_week_key)
                    .map(|s| s.duration_minutes)
                    .sum();
                let current_week_hours = current_week_minutes / 60.0;
                let already_claimed = state
                    .claimed_weekly_bonus_weeks
                    .contains(&current_week_key);
                let meets_target =
                    current_week_hours > 0.0 && current_week_hours >= safe_hours;
                let bonus_xp_awarded =
                    if !already_claimed && meets_target { WEEKLY_BONUS_XP } else { 0 };
                if bonus_xp_awarded > 0 {
                    state
                        .claimed_weekly_bonus_weeks
                        .push(current_week_key);
                }
                let old_xp = state.xp;
                let new_xp = state.xp + bonus_xp_awarded;
                state.weekly_target_hours = safe_hours;
                state.xp = new_xp;
                let (_, new_level) = level_up_effect(old_xp, new_xp, &mut effects);
                effects.push(Effect::Persist);
                effects.push(Effect::WeeklyTargetUpdated {
                    bonus_xp_awarded,
                    new_level,
                });
            }
            Action::ToggleSound => {
                state.sound_enabled = !state.sound_enabled;
                effects.push(Effect::Persist);
            }
            Action::SetSfxVolume { volume } => {
                state.sfx_volume = if volume.is_finite() {
                    ((volume * 100.0).round() / 100.0).clamp(0.0, 1.0)
                } else {
                    0.75
                };
                effects.push(Effect::Persist);
            }
            Action::ToggleMusic => {
                state.music_enabled = !state.music_enabled;
                effects.push(Effect::Persist);
            }
            Action::SetMusicEnabled { enabled } => {
                state.music_enabled = enabled;
                effects.push(Effect::Persist);
            }
            Action::SetMusicVolume { volume } => {
                state.music_volume = if volume.is_finite() {
                    ((volume * 100.0).round() / 100.0).clamp(0.0, 1.0)
                } else {
                    0.35
                };
                effects.push(Effect::Persist);
            }
            Action::ToggleReduceMotion => {
                state.reduce_motion = !state.reduce_motion;
                effects.push(Effect::Persist);
            }
            Action::RecordChunkPracticeAttempt {
                chunk_id,
                activity_id,
                total_practice_ids,
            } => {
                let prev = state
                    .lesson_progress
                    .get(&chunk_id)
                    .cloned()
                    .unwrap_or_default();
                let mut attempted = prev.attempted_activities.clone();
                if !attempted.contains(&activity_id) {
                    attempted.push(activity_id);
                }
                let found_chunk = get_lesson_chunk_by_id(&chunk_id);
                let mut all_attempted = prev.done;
                if let Some(ids) = &total_practice_ids {
                    if !ids.is_empty() {
                        all_attempted =
                            prev.done || ids.iter().all(|id| attempted.contains(id));
                    }
                } else if let Some(chunk) = found_chunk {
                    all_attempted = prev.done
                        || chunk
                            .practice
                            .iter()
                            .all(|act| attempted.contains(&act.id().to_string()));
                }
                if all_attempted {
                    if let Some(chunk) = found_chunk {
                        for item in &chunk.review_items {
                            state.review_queue.entry(item.id.clone()).or_insert(
                                crate::lesson_engine::ReviewQueueEntry {
                                    box_: 0,
                                    due: today.to_string(),
                                },
                            );
                        }
                    }
                }
                state.lesson_progress.insert(
                    chunk_id,
                    LessonChunkProgress {
                        done: all_attempted,
                        attempts: prev.attempts + 1,
                        attempted_activities: attempted,
                    },
                );
                effects.push(Effect::Persist);
            }
            Action::CompleteLessonChunk {
                chunk_id,
                review_items,
            } => {
                let prev = state
                    .lesson_progress
                    .get(&chunk_id)
                    .cloned()
                    .unwrap_or_default();
                let attempted_set = prev.attempted_activities.clone();
                // Quirk v2: hanya mencari di chunk topik 1-28; chunk quest
                // tidak ditemukan sehingga allAttempted tetap prev.done.
                let mut all_attempted = prev.done;
                for t_id in 1..=28i64 {
                    if let Some(found) = get_lesson_chunks_for_topic(&t_id.to_string())
                        .iter()
                        .find(|c| c.chunk_id == chunk_id)
                    {
                        all_attempted = prev.done
                            || found
                                .practice
                                .iter()
                                .all(|act| attempted_set.contains(&act.id().to_string()));
                        break;
                    }
                }
                for item in &review_items {
                    state.review_queue.entry(item.id.clone()).or_insert(
                        crate::lesson_engine::ReviewQueueEntry {
                            box_: 0,
                            due: today.to_string(),
                        },
                    );
                }
                state.lesson_progress.insert(
                    chunk_id.clone(),
                    LessonChunkProgress {
                        done: all_attempted,
                        attempts: prev.attempts.max(1),
                        attempted_activities: prev.attempted_activities,
                    },
                );
                effects.push(Effect::Persist);
            }
            Action::SetChunkSummary { chunk_id, summary } => {
                let clean = truncate_chars(&summary, 400);
                state.summaries.insert(chunk_id, clean);
                effects.push(Effect::Persist);
            }
            Action::ToggleChunkRealTaskDone { chunk_id, done } => {
                let next_val = match done {
                    Some(v) => v,
                    None => !state.task_done.get(&chunk_id).copied().unwrap_or(false),
                };
                if next_val {
                    state.task_done.insert(chunk_id, true);
                } else {
                    state.task_done.remove(&chunk_id);
                }
                effects.push(Effect::Persist);
            }
            Action::GradeReviewItem { item_id, rating } => {
                let current = state.review_queue.get(&item_id).cloned();
                let next_entry = schedule_review_item(
                    current.as_ref(),
                    rating,
                    state.settings.interval_scale,
                    today,
                );
                let (streak, last_active) = next_streak(&state, today);
                let stored = crate::lesson_engine::ReviewQueueEntry {
                    box_: next_entry.box_,
                    due: next_entry.due.clone(),
                };
                state.review_queue.insert(item_id.clone(), stored);
                state.streak = streak;
                state.last_active_date = last_active;
                effects.push(Effect::Persist);
                effects.push(Effect::ReviewGraded {
                    item_id,
                    box_: next_entry.box_,
                    due: next_entry.due,
                });
            }
            Action::SetIntervalScale { scale } => {
                state.settings.interval_scale =
                    crate::lesson_engine::clamp_interval_scale(scale);
                effects.push(Effect::Persist);
            }
            Action::ResetProgress => {
                state = GameState::default();
                effects.push(Effect::Persist);
            }
            Action::ImportSave { raw_json } => match import_save_json(&raw_json) {
                Ok(imported) => {
                    state = imported;
                    effects.push(Effect::Persist);
                    effects.push(Effect::ImportSucceeded);
                }
                Err(message) => {
                    effects.push(Effect::ImportFailed { message });
                }
            },
        }

        (state, effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{calculate_level_from_xp, calculate_room_xp};
    use crate::lesson_engine::{get_lesson_chunk_by_id, ReviewRating};
    use crate::spellbook::get_tower_quest_by_id;
    use dungeon_data::roadmap::topic_by_number;

    fn ctx() -> ActionContext {
        ActionContext {
            today_key: "2026-11-17".to_string(),
            now_iso: "2026-11-17T08:00:00.000Z".to_string(),
        }
    }

    fn fresh() -> GameState {
        GameState::default()
    }

    fn has_effect(effects: &[Effect], pred: impl Fn(&Effect) -> bool) -> bool {
        effects.iter().any(pred)
    }

    #[test]
    fn start_adventure_sets_name_avatar_and_started() {
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::StartAdventure {
                name: "  Alya Tensor ".to_string(),
                avatar_id: "ranger".to_string(),
            },
            &ctx(),
        );
        assert!(state.has_started);
        assert_eq!(state.player_name, "Alya Tensor");
        assert_eq!(state.avatar_id, "ranger");
        assert!(has_effect(&effects, |e| matches!(e, Effect::Persist)));

        let (state2, _) = Reducer::apply(
            fresh(),
            Action::StartAdventure {
                name: "   ".to_string(),
                avatar_id: "".to_string(),
            },
            &ctx(),
        );
        assert_eq!(state2.player_name, "Petualang Tensor");
        assert_eq!(state2.avatar_id, "alchemist");
    }

    #[test]
    fn defeat_room_awards_xp_crit_spell_and_unlocks_next() {
        let topic = topic_by_number(1).unwrap();
        let base = calculate_room_xp(topic.weeks.min, topic.weeks.max);
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::DefeatRoom {
                topic_id: "topic-1".to_string(),
                correct_quiz_answers: 5,
                quiz_score_percent: Some(100),
                echo_correct: false,
            },
            &ctx(),
        );
        let crit = (base as f64 * 0.1).round() as i64;
        assert_eq!(state.xp, base + crit);
        assert!(state.completed_rooms.contains(&"topic-1".to_string()));
        assert!(state.spells_unlocked.contains(&"s1".to_string()));
        assert_eq!(state.best_score_by_topic.get("topic-1"), Some(&100));
        assert_eq!(state.streak, 1);
        assert_eq!(state.last_active_date.as_deref(), Some("2026-11-17"));
        // Ruangan berikutnya (topic-2) terbuka.
        let room = effects.iter().find_map(|e| match e {
            Effect::RoomDefeated { newly_unlocked_ids, .. } => Some(newly_unlocked_ids.clone()),
            _ => None,
        });
        assert!(room.unwrap().contains(&"topic-2".to_string()));
        // Node aktif pindah ke topic-2.
        assert_eq!(state.active_node_by_floor.get(&1).map(String::as_str), Some("topic-2"));

        // Mengalahkan ruangan yang sama lagi: tanpa XP dasar/bonus.
        let (state2, effects2) = Reducer::apply(
            state,
            Action::DefeatRoom {
                topic_id: "topic-1".to_string(),
                correct_quiz_answers: 5,
                quiz_score_percent: Some(100),
                echo_correct: false,
            },
            &ctx(),
        );
        let gained = effects2.iter().find_map(|e| match e {
            Effect::RoomDefeated { xp_gained, .. } => Some(*xp_gained),
            _ => None,
        });
        assert_eq!(gained, Some(0));
        assert_eq!(state2.xp, base + crit);
    }

    #[test]
    fn defeat_room_unknown_topic_is_noop() {
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::DefeatRoom {
                topic_id: "topic-99".to_string(),
                correct_quiz_answers: 5,
                quiz_score_percent: Some(100),
                echo_correct: false,
            },
            &ctx(),
        );
        assert_eq!(state.xp, 0);
        assert!(!has_effect(&effects, |e| matches!(e, Effect::Persist)));
    }

    #[test]
    fn defeat_boss_awards_300_badge_relic_and_clears_phases() {
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::DefeatBoss { floor_id: 1 },
            &ctx(),
        );
        assert_eq!(state.xp, 300);
        assert!(state.defeated_bosses.contains(&"boss-1".to_string()));
        assert!(state.badges.contains(&"badge-f1".to_string()));
        assert!(state.relics.contains(&"relic-boss-1".to_string()));
        assert_eq!(
            state.boss_progress[&1].cleared_phases,
            vec![1, 2, 3, 4, 5, 6]
        );
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::BossDefeated { xp_gained: 300, .. }
        )));

        // Boss yang sama lagi: tanpa XP.
        let (state2, _) = Reducer::apply(state, Action::DefeatBoss { floor_id: 1 }, &ctx());
        assert_eq!(state2.xp, 300);
    }

    #[test]
    fn dojo_xp_capped_at_20_per_day() {
        let mut state = fresh();
        for _ in 0..10 {
            let (next, _) = Reducer::apply(
                state,
                Action::AwardDojoAnswerXp {
                    question_id: "t1q1".to_string(),
                    is_correct: true,
                },
                &ctx(),
            );
            state = next;
        }
        assert_eq!(state.xp, 20);
        assert_eq!(state.dojo_daily_xp.xp, 20);
        // Jawaban benar ke-11 hari yang sama: 0 XP.
        let (state2, effects) = Reducer::apply(
            state,
            Action::AwardDojoAnswerXp {
                question_id: "t1q1".to_string(),
                is_correct: true,
            },
            &ctx(),
        );
        assert_eq!(state2.xp, 20);
        let awarded = effects.iter().find_map(|e| match e {
            Effect::DojoAnswer { xp_awarded, .. } => Some(*xp_awarded),
            _ => None,
        });
        assert_eq!(awarded, Some(0));
    }

    #[test]
    fn tower_quest_complete_and_toggle() {
        // completeTowerQuest m1: centang semua langkah dulu.
        let mut state = fresh();
        let quest = get_tower_quest_by_id("m1").unwrap();
        for step in &quest.steps {
            let (next, _) = Reducer::apply(
                state,
                Action::ToggleTowerStep {
                    quest_id: "m1".to_string(),
                    step_id: step.id.clone(),
                },
                &ctx(),
            );
            state = next;
        }
        let (state, effects) = Reducer::apply(
            state,
            Action::CompleteTowerQuest {
                quest_id: "m1".to_string(),
            },
            &ctx(),
        );
        assert_eq!(state.xp, 150);
        assert!(state.tower_completed.contains(&"m1".to_string()));
        assert!(state.completed_tower_quests.contains(&"m1".to_string()));
        assert!(state.relics.contains(&"relic-m1".to_string()));
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::TowerQuestCompleted { xp_gained: 150, .. }
        )));

        // toggleTowerQuest m2 (tanpa langkah): tetap +150 XP (quirk v2).
        let (state2, _) = Reducer::apply(
            state,
            Action::ToggleTowerQuest {
                quest_id: "m2".to_string(),
            },
            &ctx(),
        );
        assert_eq!(state2.xp, 300);
        assert!(state2.tower_completed.contains(&"m2".to_string()));
        assert!(state2.relics.contains(&"relic-m2".to_string()));

        // m6s3 tidak bisa dicentang tanpa spesialisasi.
        let (_, denied) = Reducer::apply(
            fresh(),
            Action::ToggleTowerStep {
                quest_id: "m6".to_string(),
                step_id: "m6s3".to_string(),
            },
            &ctx(),
        );
        assert!(has_effect(&denied, |e| matches!(
            e,
            Effect::TowerStepDenied { .. }
        )));
    }

    #[test]
    fn log_study_session_weekly_bonus_once_per_week() {
        let mut state = fresh();
        // 9 jam dalam minggu yang sama (Senin 2026-11-16) -> bonus 50 XP.
        let (next, effects) = Reducer::apply(
            state,
            Action::LogStudySession {
                session_id: "ses-1".to_string(),
                created_at_ms: 1763260800000,
                topic_id: "topic-1".to_string(),
                duration_minutes: 540,
                date: "2026-11-17".to_string(),
                note: None,
                source: StudySessionSource::Manual,
            },
            &ctx(),
        );
        state = next;
        assert_eq!(state.xp, 50);
        assert!(state
            .claimed_weekly_bonus_weeks
            .contains(&"2026-11-16".to_string()));
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::StudySessionLogged {
                bonus_xp_awarded: 50,
                ..
            }
        )));

        // Sesi kedua minggu yang sama: tanpa bonus lagi.
        let (state2, effects2) = Reducer::apply(
            state,
            Action::LogStudySession {
                session_id: "ses-2".to_string(),
                created_at_ms: 1763260900000,
                topic_id: "topic-2".to_string(),
                duration_minutes: 60,
                date: "2026-11-17".to_string(),
                note: None,
                source: StudySessionSource::Pomodoro,
            },
            &ctx(),
        );
        assert_eq!(state2.xp, 50);
        assert_eq!(state2.study_sessions.len(), 2);
        assert!(has_effect(&effects2, |e| matches!(
            e,
            Effect::StudySessionLogged {
                bonus_xp_awarded: 0,
                ..
            }
        )));
    }

    #[test]
    fn chunk_practice_completes_chunk_and_enqueues_reviews() {
        let chunk = get_lesson_chunk_by_id("t1c1").unwrap();
        let mut state = fresh();
        for act in &chunk.practice {
            let (next, _) = Reducer::apply(
                state,
                Action::RecordChunkPracticeAttempt {
                    chunk_id: "t1c1".to_string(),
                    activity_id: act.id().to_string(),
                    total_practice_ids: None,
                },
                &ctx(),
            );
            state = next;
        }
        let progress = state.lesson_progress.get("t1c1").unwrap();
        assert!(progress.done);
        assert_eq!(progress.attempts as usize, chunk.practice.len());
        // Kartu review chunk masuk antrean dengan due hari ini.
        for item in &chunk.review_items {
            let entry = state.review_queue.get(&item.id).unwrap();
            assert_eq!(entry.box_, 0);
            assert_eq!(entry.due, "2026-11-17");
        }

        // Menilai satu kartu "ingat": naik kotak, jatuh tempo +3 hari.
        let first_item = chunk.review_items[0].id.clone();
        let (state2, effects) = Reducer::apply(
            state,
            Action::GradeReviewItem {
                item_id: first_item.clone(),
                rating: ReviewRating::Ingat,
            },
            &ctx(),
        );
        assert_eq!(state2.review_queue[&first_item].box_, 1);
        assert_eq!(state2.review_queue[&first_item].due, "2026-11-20");
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::ReviewGraded { box_: 1, .. }
        )));
    }

    #[test]
    fn level_up_effect_emitted_on_threshold_crossing() {
        // Level 2 butuh 60 XP: kalahkan boss (+300) dari 0 melewati level 2.
        let (_, effects) = Reducer::apply(
            fresh(),
            Action::DefeatBoss { floor_id: 1 },
            &ctx(),
        );
        let level_up = effects.iter().find_map(|e| match e {
            Effect::LevelUp {
                old_level,
                new_level,
                new_title,
            } => Some((*old_level, *new_level, new_title.clone())),
            _ => None,
        });
        let (old_level, new_level, _) = level_up.unwrap();
        assert_eq!(old_level, 1);
        assert_eq!(new_level, calculate_level_from_xp(300));
        assert!(new_level > 2);
    }

    #[test]
    fn import_save_via_reducer_and_failed_import_keeps_state() {
        let good = serde_json::json!({
            "state": { "playerName": "Impor Uji", "xp": 450, "completedRooms": ["topic-1"] },
            "version": 1
        })
        .to_string();
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::ImportSave { raw_json: good },
            &ctx(),
        );
        assert_eq!(state.player_name, "Impor Uji");
        assert_eq!(state.xp, 450);
        assert!(state.has_started);
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::ImportSucceeded
        )));

        let (state2, effects2) = Reducer::apply(
            state.clone(),
            Action::ImportSave {
                raw_json: "{not valid json".to_string(),
            },
            &ctx(),
        );
        assert_eq!(state2, state);
        assert!(has_effect(&effects2, |e| matches!(
            e,
            Effect::ImportFailed { .. }
        )));
    }

    #[test]
    fn boss_potion_and_variant_rules() {
        // Lantai 1: 2 ramuan.
        let (state, effects) = Reducer::apply(
            fresh(),
            Action::UseBossPotion { floor_id: 1 },
            &ctx(),
        );
        assert_eq!(state.boss_progress[&1].potions, 1);
        assert!(has_effect(&effects, |e| matches!(
            e,
            Effect::BossPotionUsed {
                potions_left: 1,
                ..
            }
        )));
        let (state, _) = Reducer::apply(state, Action::UseBossPotion { floor_id: 1 }, &ctx());
        assert_eq!(state.boss_progress[&1].potions, 0);
        let (_, denied) = Reducer::apply(
            state,
            Action::UseBossPotion { floor_id: 1 },
            &ctx(),
        );
        assert!(has_effect(&denied, |e| matches!(
            e,
            Effect::BossPotionDenied { .. }
        )));

        // Varian: A dulu, lalu B, lalu A lagi.
        let (state, _) = Reducer::apply(
            fresh(),
            Action::AdvanceBossPhaseVariant {
                floor_id: 1,
                phase: 1,
            },
            &ctx(),
        );
        assert_eq!(state.boss_progress[&1].last_variant_by_phase[&1], 0);
        let (state, _) = Reducer::apply(
            state,
            Action::AdvanceBossPhaseVariant {
                floor_id: 1,
                phase: 1,
            },
            &ctx(),
        );
        assert_eq!(state.boss_progress[&1].last_variant_by_phase[&1], 1);

        // Pingsan: fase terakhir yang dibersihkan kembali.
        let (state, _) = Reducer::apply(
            fresh(),
            Action::CompleteBossPhase {
                floor_id: 1,
                phase: 1,
            },
            &ctx(),
        );
        let (state, _) = Reducer::apply(
            state,
            Action::CompleteBossPhase {
                floor_id: 1,
                phase: 2,
            },
            &ctx(),
        );
        let (state, _) = Reducer::apply(state, Action::FaintBossFight { floor_id: 1 }, &ctx());
        assert_eq!(state.boss_progress[&1].cleared_phases, vec![1]);
    }
}
