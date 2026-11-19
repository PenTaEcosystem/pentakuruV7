use crate::{PentaKuru, ui_theme as theme};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const COLUMNS: [(&str, &str); 3] = [
    ("todo", "Cần làm"),
    ("doing", "Đang làm"),
    ("done", "Hoàn tất"),
];

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    #[default]
    None,
    Daily,
    Weekly,
}

impl ScheduleKind {
    pub fn from_str_lossy(raw: &str) -> Self {
        let lower = raw.trim().to_lowercase();
        if matches!(
            lower.as_str(),
            "daily" | "mỗi ngày" | "hang ngay" | "hằng ngày"
        ) {
            Self::Daily
        } else if matches!(
            lower.as_str(),
            "weekly" | "mỗi tuần" | "hang tuan" | "hằng tuần"
        ) {
            Self::Weekly
        } else {
            Self::None
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub note: String,
    pub status: String,
    pub archived: bool,
    pub schedule: ScheduleKind,
    pub remind_enabled: bool,
    pub due_at: String,
    pub remind_interval_mins: u32,
    pub weekly_target: u8,
    pub completed_dates: Vec<String>,
    pub last_completed_date: String,
    pub last_completed_week: i64,
    pub last_reminded_unix: u64,
    pub next_week_command: String,
    pub week_cycle: u32,
    pub note_id: Option<String>,
}

/// Parses `"HH:MM"`, `"HHhMM"`, `"YYYY-MM-DD HH:MM"`, `"DD/MM/YYYY HH:MM"`, or `"YYYY-MM-DD"`.
pub fn parse_due_time(raw: &str) -> Option<(Option<String>, u32)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parse_hm = |part: &str| -> Option<u32> {
        let norm = part.trim().replace(['h', 'H'], ":");
        let (h_s, m_s) = norm.split_once(':')?;
        let h: u32 = h_s.trim().parse().ok()?;
        let m: u32 = if m_s.trim().is_empty() {
            0
        } else {
            m_s.trim().parse().ok()?
        };
        (h < 24 && m < 60).then_some(h * 60 + m)
    };
    let normalize_date = |part: &str| -> Option<String> {
        let p = part.trim();
        let segs: Vec<&str> = p.split(['-', '/']).collect();
        if segs.len() != 3 {
            return None;
        }
        if segs[0].len() == 4 {
            let y: u32 = segs[0].parse().ok()?;
            let m: u32 = segs[1].parse().ok()?;
            let d: u32 = segs[2].parse().ok()?;
            (y >= 2020 && (1..=12).contains(&m) && (1..=31).contains(&d))
                .then(|| format!("{y:04}-{m:02}-{d:02}"))
        } else if segs[2].len() == 4 {
            let d: u32 = segs[0].parse().ok()?;
            let m: u32 = segs[1].parse().ok()?;
            let y: u32 = segs[2].parse().ok()?;
            (y >= 2020 && (1..=12).contains(&m) && (1..=31).contains(&d))
                .then(|| format!("{y:04}-{m:02}-{d:02}"))
        } else {
            None
        }
    };
    if let Some((a, b)) = trimmed.split_once(' ').or_else(|| trimmed.split_once('T')) {
        if let (Some(d), Some(hm)) = (normalize_date(a), parse_hm(b)) {
            return Some((Some(d), hm));
        }
        if let (Some(hm), Some(d)) = (parse_hm(a), normalize_date(b)) {
            return Some((Some(d), hm));
        }
    }
    if let Some(hm) = parse_hm(trimmed) {
        return Some((None, hm));
    }
    if let Some(d) = normalize_date(trimmed) {
        return Some((Some(d), 9 * 60));
    }
    None
}

impl Card {
    pub fn is_completed_today(&self, today: &str) -> bool {
        self.status == "done"
            || (self.schedule != ScheduleKind::None && self.last_completed_date == today)
    }

    pub fn mark_done(&mut self, today: &str, week_idx: i64) {
        self.status = "done".into();
        if self.schedule != ScheduleKind::None {
            if self.last_completed_week != 0 && self.last_completed_week != week_idx {
                self.completed_dates.clear();
                self.week_cycle += 1;
            }
            if !self.completed_dates.iter().any(|d| d == today) {
                self.completed_dates.push(today.to_owned());
            }
            self.last_completed_date = today.to_owned();
            self.last_completed_week = week_idx;
        } else {
            self.last_completed_date = today.to_owned();
            self.remind_enabled = false;
        }
    }

    pub fn cancel_schedule(&mut self) {
        self.schedule = ScheduleKind::None;
        self.remind_enabled = false;
        self.due_at.clear();
        self.remind_interval_mins = 0;
        self.next_week_command.clear();
    }

    pub fn schedule_badge(&self) -> Option<String> {
        let mut parts = Vec::new();
        match self.schedule {
            ScheduleKind::Daily => {
                let target = self.weekly_target.max(7);
                parts.push(format!("Daily {}/{}", self.completed_dates.len(), target));
            }
            ScheduleKind::Weekly => {
                let target = self.weekly_target.max(1);
                parts.push(format!("Tuần {}/{}", self.completed_dates.len(), target));
            }
            ScheduleKind::None => {}
        }
        if !self.due_at.trim().is_empty() {
            parts.push(format!("Hạn {}", self.due_at.trim()));
        }
        if self.remind_enabled && self.remind_interval_mins > 0 {
            parts.push(format!("Nhắc {}p", self.remind_interval_mins));
        } else if self.remind_enabled && parts.is_empty() {
            parts.push("Có nhắc".into());
        }
        (!parts.is_empty()).then(|| parts.join(" · "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeeklyRollover {
    pub card_id: String,
    pub title: String,
    pub note_id: Option<String>,
    pub done_count: usize,
    pub weekly_target: u8,
    pub next_week_command: String,
    pub next_week_cycle: u32,
}

fn card_content(card: &Card) -> String {
    let body = match card.note.split_once('\n') {
        Some((first, rest)) if first.starts_with('#') => rest.trim_start_matches('\n'),
        _ if card.note.starts_with('#') => "",
        _ => card.note.as_str(),
    };
    format!("# {}\n\n{body}", card.title)
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Board {
    pub cards: Vec<Card>,
}

#[derive(Default)]
pub struct State {
    pub memory: crate::note_memory::State,
    pub library_open: bool,
    pub preview: bool,
    pub board_open: bool,
    pub profile_open: bool,
    pub board: Board,
    pub status: String,
    loaded_root: Option<PathBuf>,
    pub(super) storage_ready: bool,
    pub(super) new_card_open: bool,
    new_title: String,
    new_content: String,
    show_archived: bool,
    narrow_column: usize,
    editing: Option<Card>,
    pub ai_note_input: String,
    pub ai_note_bar_open: bool,
    pub pending_note_command: bool,
    pub pending_note_target_id: Option<String>,
}
impl State {
    pub fn with_preview() -> Self {
        Self {
            preview: true,
            ..Default::default()
        }
    }
}

enum Request {
    Create {
        title: String,
        content: String,
    },
    FromNote {
        note_id: String,
        card_id: Option<String>,
        title: String,
        content: String,
    },
    Edit(Card),
    Move {
        card_id: String,
        status: String,
    },
    Archive {
        card_id: String,
        archived: bool,
    },
}

impl Board {
    fn load(root: &Path) -> Result<Self, String> {
        match std::fs::read(root.join(".kanban.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.to_string()),
        }
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp = root.join(format!(".kanban-{}-{stamp}.tmp", std::process::id()));
        std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if let Err(error) = std::fs::rename(&temp, root.join(".kanban.json")) {
            let _ = std::fs::remove_file(temp);
            return Err(error.to_string());
        }
        Ok(())
    }
    fn create(&mut self, title: String, note: String) -> Result<String, String> {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err("Nhập tiêu đề công việc.".into());
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("local-{}-{stamp}-{}", std::process::id(), self.cards.len());
        self.cards.push(Card {
            id: id.clone(),
            title,
            note,
            status: "todo".into(),
            archived: false,
            ..Default::default()
        });
        Ok(id)
    }
    fn apply(&mut self, operation: Request) -> Result<Option<(String, String)>, String> {
        let (today, _, week_idx, _) = crate::system::local_clock_now();
        match operation {
            Request::Create { title, content } => {
                let id = self.create(title, content)?;
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                    card.last_completed_week = week_idx;
                    card.week_cycle = 1;
                }
            }
            Request::FromNote {
                note_id,
                card_id,
                title,
                content,
            } => {
                let id = if let Some(card) = self
                    .cards
                    .iter_mut()
                    .find(|card| Some(&card.id) == card_id.as_ref())
                {
                    card.title = title;
                    card.note = content;
                    card.archived = false;
                    card.note_id = Some(note_id.clone());
                    card.id.clone()
                } else {
                    let id = self.create(title, content)?;
                    if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                        card.note_id = Some(note_id.clone());
                    }
                    id
                };
                return Ok(Some((note_id, id)));
            }
            Request::Edit(updated) => {
                if updated.title.trim().is_empty() {
                    return Err("Nhập tiêu đề công việc.".into());
                }
                let card = self
                    .cards
                    .iter_mut()
                    .find(|card| card.id == updated.id)
                    .ok_or("Không tìm thấy thẻ.")?;
                card.title = updated.title.trim().into();
                card.note = updated.note;
                card.schedule = updated.schedule;
                card.due_at = updated.due_at.trim().to_owned();
                card.remind_interval_mins = updated.remind_interval_mins;
                card.remind_enabled = updated.remind_enabled;
                card.weekly_target = updated.weekly_target;
                card.next_week_command = updated.next_week_command.trim().to_owned();
            }
            Request::Move { card_id, status } => {
                if !COLUMNS.iter().any(|(value, _)| *value == status) {
                    return Err("Trạng thái không hợp lệ.".into());
                }
                let card = self
                    .cards
                    .iter_mut()
                    .find(|card| card.id == card_id)
                    .ok_or("Không tìm thấy thẻ.")?;
                if status == "done" {
                    card.mark_done(&today, week_idx);
                } else {
                    card.status = status;
                    if card.schedule != ScheduleKind::None && card.last_completed_date == today {
                        card.last_completed_date.clear();
                        card.completed_dates.retain(|d| d != &today);
                    }
                }
            }
            Request::Archive { card_id, archived } => {
                self.cards
                    .iter_mut()
                    .find(|card| card.id == card_id)
                    .ok_or("Không tìm thấy thẻ.")?
                    .archived = archived;
            }
        }
        Ok(None)
    }

    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let mut todo = 0;
        let mut doing = 0;
        let mut done = 0;
        for card in self.cards.iter().filter(|c| !c.archived) {
            match card.status.as_str() {
                "doing" => doing += 1,
                "done" => done += 1,
                _ => todo += 1,
            }
        }
        (todo + doing + done, todo, doing, done)
    }

    /// Context for AI prompts: strictly excludes completed (`done` / completed-today) and archived tasks.
    pub fn ai_context_summary(&self) -> String {
        let (today, _, _, _) = crate::system::local_clock_now();
        let mut pending_todo = Vec::new();
        let mut pending_doing = Vec::new();
        for card in self
            .cards
            .iter()
            .filter(|c| !c.archived && !c.is_completed_today(&today))
        {
            if card.status == "doing" {
                pending_doing.push(card);
            } else {
                pending_todo.push(card);
            }
        }
        let pending_total = pending_todo.len() + pending_doing.len();
        if pending_total == 0 {
            return "Hiện không còn công việc nào chưa hoàn thành (các việc đã xong không gửi lên AI).".into();
        }
        let mut out = format!(
            "Công việc chưa hoàn thành hôm nay: {pending_total} việc (Cần làm: {}, Đang làm: {}).\n",
            pending_todo.len(),
            pending_doing.len()
        );
        for (status_label, items) in [("Cần làm", pending_todo), ("Đang làm", pending_doing)] {
            if items.is_empty() {
                continue;
            }
            out.push_str(&format!("- {status_label}:\n"));
            for card in items.into_iter().take(25) {
                let desc: String = card
                    .note
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(80)
                    .collect();
                let badge = card
                    .schedule_badge()
                    .map(|b| format!(" [{b}]"))
                    .unwrap_or_default();
                if desc.trim().is_empty() {
                    out.push_str(&format!("  • {}{badge}\n", card.title));
                } else {
                    out.push_str(&format!("  • {}{badge}: {}\n", card.title, desc.trim()));
                }
            }
        }
        out
    }

    /// Reopens `Daily`/`Weekly` tasks on a new day/week and emits `WeeklyRollover` when a week finishes.
    pub fn rollover_schedules(
        &mut self,
        today: &str,
        week_idx: i64,
    ) -> (bool, Vec<WeeklyRollover>) {
        let mut changed = false;
        let mut rollovers = Vec::new();
        for card in self.cards.iter_mut().filter(|c| !c.archived) {
            match card.schedule {
                ScheduleKind::Daily => {
                    let week_changed =
                        card.last_completed_week != 0 && week_idx != card.last_completed_week;
                    if week_changed {
                        let done_count = card.completed_dates.len();
                        let target = card.weekly_target.max(7);
                        if done_count > 0 && !card.next_week_command.trim().is_empty() {
                            card.week_cycle = card.week_cycle.max(1) + 1;
                            rollovers.push(WeeklyRollover {
                                card_id: card.id.clone(),
                                title: card.title.clone(),
                                note_id: card.note_id.clone(),
                                done_count,
                                weekly_target: target,
                                next_week_command: card.next_week_command.clone(),
                                next_week_cycle: card.week_cycle,
                            });
                        } else if done_count > 0 {
                            card.week_cycle = card.week_cycle.max(1) + 1;
                        }
                        card.completed_dates.clear();
                        card.last_completed_week = week_idx;
                        card.status = "todo".into();
                        changed = true;
                    } else if card.status == "done" && card.last_completed_date != today {
                        card.status = "todo".into();
                        changed = true;
                    }
                }
                ScheduleKind::Weekly => {
                    if card.last_completed_week != 0 && week_idx != card.last_completed_week {
                        let done_count = card.completed_dates.len();
                        let target = card.weekly_target.max(1);
                        if done_count > 0 && !card.next_week_command.trim().is_empty() {
                            card.week_cycle = card.week_cycle.max(1) + 1;
                            rollovers.push(WeeklyRollover {
                                card_id: card.id.clone(),
                                title: card.title.clone(),
                                note_id: card.note_id.clone(),
                                done_count,
                                weekly_target: target,
                                next_week_command: card.next_week_command.clone(),
                                next_week_cycle: card.week_cycle,
                            });
                        }
                        card.completed_dates.clear();
                        card.last_completed_week = week_idx;
                        if card.status == "done" {
                            card.status = "todo".into();
                        }
                        changed = true;
                    }
                }
                ScheduleKind::None => {}
            }
        }
        (changed, rollovers)
    }

    /// Returns card indices that are currently due for reminder based on flexible per-card timing.
    pub fn due_reminder_cards(
        &self,
        today: &str,
        now_hm: u32,
        now_unix: u64,
        default_interval_mins: u32,
        is_startup: bool,
    ) -> Vec<usize> {
        let mut due = Vec::new();
        for (idx, card) in self.cards.iter().enumerate() {
            if card.archived || !card.remind_enabled || card.is_completed_today(today) {
                continue;
            }
            if is_startup {
                if now_unix.saturating_sub(card.last_reminded_unix) >= 300 {
                    due.push(idx);
                }
                continue;
            }
            if let Some((due_ymd, due_hm)) = parse_due_time(&card.due_at) {
                let date_reached = due_ymd.as_deref().is_none_or(|d| d <= today);
                if !date_reached {
                    continue;
                }
                let past_earlier_day = due_ymd.as_deref().is_some_and(|d| d < today);
                let approaching = past_earlier_day || now_hm >= due_hm.saturating_sub(15);
                if approaching {
                    let cooldown = card.remind_interval_mins.max(default_interval_mins).max(20);
                    if now_unix.saturating_sub(card.last_reminded_unix) >= u64::from(cooldown) * 60
                    {
                        due.push(idx);
                    }
                } else if card.remind_interval_mins > 0
                    && now_unix.saturating_sub(card.last_reminded_unix)
                        >= u64::from(card.remind_interval_mins) * 60
                {
                    due.push(idx);
                }
            } else {
                let eff_mins = if card.remind_interval_mins > 0 {
                    card.remind_interval_mins
                } else {
                    default_interval_mins
                };
                if eff_mins > 0
                    && now_unix.saturating_sub(card.last_reminded_unix) >= u64::from(eff_mins) * 60
                {
                    due.push(idx);
                }
            }
        }
        due
    }

    pub fn cancel_matching_schedules(&mut self, query: &str) -> usize {
        let q = crate::system::fold(query.trim());
        let cancel_all = q.is_empty() || matches!(q.as_str(), "all" | "tat ca" | "moi lich");
        let mut count = 0;
        for card in &mut self.cards {
            if card.archived || (card.schedule == ScheduleKind::None && !card.remind_enabled) {
                continue;
            }
            let title_match = cancel_all || crate::system::matches_terms(&card.title, &q);
            let note_match = !cancel_all && crate::system::matches_terms(&card.note, &q);
            if title_match || note_match {
                card.cancel_schedule();
                count += 1;
            }
        }
        count
    }

    pub fn complete_matching_tasks(&mut self, query: &str, today: &str, week_idx: i64) -> usize {
        let q = crate::system::fold(query.trim());
        if q.is_empty() {
            return 0;
        }
        let mut count = 0;
        for card in &mut self.cards {
            if card.archived || card.is_completed_today(today) {
                continue;
            }
            if crate::system::matches_terms(&card.title, &q)
                || crate::system::matches_terms(&card.note, &q)
            {
                card.mark_done(today, week_idx);
                count += 1;
            }
        }
        count
    }

    pub fn upsert_tasks(&mut self, tasks: &[(String, String, String)]) -> (usize, Option<String>) {
        let specs: Vec<crate::ai::PlannedTaskSpec> = tasks
            .iter()
            .map(|(title, note, status)| crate::ai::PlannedTaskSpec {
                title: title.clone(),
                note: note.clone(),
                status: status.clone(),
                ..Default::default()
            })
            .collect();
        self.upsert_scheduled_tasks(&specs, ScheduleKind::None, "", 0, 0, false, "", None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_scheduled_tasks(
        &mut self,
        tasks: &[crate::ai::PlannedTaskSpec],
        default_schedule: ScheduleKind,
        default_due: &str,
        default_interval_mins: u32,
        default_weekly_target: u8,
        default_remind: bool,
        default_next_week_cmd: &str,
        note_id: Option<&str>,
    ) -> (usize, Option<String>) {
        let (_, _, week_idx, _) = crate::system::local_clock_now();
        let mut count = 0;
        let mut first_id = None;
        for item in tasks {
            let clean_title = item.title.trim();
            if clean_title.is_empty() {
                continue;
            }
            let valid_status = if COLUMNS.iter().any(|(k, _)| *k == item.status.as_str()) {
                item.status.as_str()
            } else {
                "todo"
            };
            let sched = if item.schedule.trim().is_empty() {
                default_schedule
            } else {
                ScheduleKind::from_str_lossy(&item.schedule)
            };
            let due_at = if item.due_at.trim().is_empty() {
                default_due.trim().to_owned()
            } else {
                item.due_at.trim().to_owned()
            };
            let interval_mins = if item.interval_mins > 0 {
                item.interval_mins
            } else {
                default_interval_mins
            };
            let weekly_target = if item.weekly_target > 0 {
                item.weekly_target
            } else if default_weekly_target > 0 {
                default_weekly_target
            } else if sched == ScheduleKind::Daily {
                7
            } else if sched == ScheduleKind::Weekly {
                1
            } else {
                0
            };
            let next_week = if item.next_week_command.trim().is_empty() {
                default_next_week_cmd.trim().to_owned()
            } else {
                item.next_week_command.trim().to_owned()
            };
            let remind = item.remind.unwrap_or(default_remind)
                || sched != ScheduleKind::None
                || !due_at.is_empty()
                || interval_mins > 0;

            if let Some(existing) = self
                .cards
                .iter_mut()
                .find(|c| c.title.eq_ignore_ascii_case(clean_title))
            {
                if !item.note.trim().is_empty() {
                    existing.note = item.note.clone();
                }
                existing.status = valid_status.to_owned();
                existing.archived = false;
                existing.schedule = sched;
                existing.due_at = due_at;
                existing.remind_interval_mins = interval_mins;
                existing.weekly_target = weekly_target;
                existing.remind_enabled = remind;
                if !next_week.is_empty() {
                    existing.next_week_command = next_week;
                }
                if let Some(nid) = note_id {
                    existing.note_id = Some(nid.to_owned());
                }
                if existing.last_completed_week == 0 {
                    existing.last_completed_week = week_idx;
                }
                if first_id.is_none() {
                    first_id = Some(existing.id.clone());
                }
                count += 1;
            } else if let Ok(id) = self.create(clean_title.to_owned(), item.note.clone()) {
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == id) {
                    card.status = valid_status.to_owned();
                    card.schedule = sched;
                    card.due_at = due_at;
                    card.remind_interval_mins = interval_mins;
                    card.weekly_target = weekly_target;
                    card.remind_enabled = remind;
                    card.next_week_command = next_week;
                    card.note_id = note_id.map(str::to_owned);
                    card.last_completed_week = week_idx;
                    card.week_cycle = 1;
                }
                if first_id.is_none() {
                    first_id = Some(id);
                }
                count += 1;
            }
        }
        (count, first_id)
    }
}

impl PentaKuru {
    pub(super) fn poll_workspace(&mut self) {
        let root = &self.notes_mgr.root_dir;
        if self.workspace.loaded_root.as_ref() == Some(root) {
            return;
        }
        self.workspace.loaded_root = Some(root.clone());
        match Board::load(root) {
            Ok(board) => {
                self.workspace.board = board;
                self.workspace.storage_ready = true;
            }
            Err(error) => {
                self.workspace.storage_ready = false;
                self.workspace.status =
                    format!("Chưa đọc được Kanban: {error}. Dữ liệu hiện có được giữ nguyên.");
            }
        }
    }
    fn board_request(&mut self, operation: Request) -> bool {
        if !self.workspace.storage_ready {
            return false;
        }
        let mut board = self.workspace.board.clone();
        match board
            .apply(operation)
            .and_then(|link| board.save(&self.notes_mgr.root_dir).map(|()| link))
        {
            Ok(link) => {
                self.workspace.board = board;
                if let Some((note_id, card_id)) = link {
                    let selected = self.notes_mgr.selected_note_id.clone();
                    if let Some(note) = self
                        .notes_mgr
                        .notes
                        .iter_mut()
                        .find(|note| note.id == note_id)
                    {
                        note.task_card_id = Some(card_id);
                        self.notes_mgr.selected_note_id = Some(note_id);
                        self.notes_mgr.save_selected_note();
                        self.notes_mgr.selected_note_id = selected;
                    }
                }
                self.workspace.status = "Đã lưu Kanban trên máy.".into();
                true
            }
            Err(error) => {
                self.workspace.status = format!("Chưa lưu được: {error}");
                false
            }
        }
    }

    /// Đồng bộ ghi chú hiện tại sang Kanban (tự tách các dòng công việc `- [ ]` hoặc tạo thẻ từ note).
    pub(super) fn sync_note_to_kanban(&mut self, index: usize) {
        self.poll_workspace();
        if !self.workspace.storage_ready || index >= self.notes_mgr.notes.len() {
            return;
        }
        let note_id = self.notes_mgr.notes[index].id.clone();
        let note_title = self.notes_mgr.notes[index].title.clone();
        let note_content = self.notes_mgr.notes[index].content.clone();
        let note_card_id = self.notes_mgr.notes[index].task_card_id.clone();

        let plan = crate::ai::parse_ai_note_response(&note_content, &note_title);
        if !plan.tasks.is_empty() {
            let mut board = self.workspace.board.clone();
            let (count, first_id) = board.upsert_tasks(&plan.tasks);
            if count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                self.workspace.board = board;
                if let Some(card_id) = first_id {
                    self.notes_mgr.notes[index].task_card_id = Some(card_id);
                    self.notes_mgr.save_selected_note();
                }
                self.workspace.status = format!("Đã đồng bộ {count} công việc vào Kanban.");
                return;
            }
        }
        self.board_request(Request::FromNote {
            note_id,
            card_id: note_card_id,
            title: note_title,
            content: note_content,
        });
    }

    /// Áp dụng kết quả từ lệnh `@note` của AI: tạo hoặc cập nhật ghi chú (kèm thư mục) và đồng bộ Kanban.
    pub(super) fn apply_ai_note_plan(&mut self, reply: &str) -> String {
        self.poll_workspace();
        let target_id = self.workspace.pending_note_target_id.take();
        let plan = crate::ai::parse_ai_note_response(reply, "Ghi chú & Công việc AI");

        let note_id = if let Some(existing_id) = target_id
            && let Some(note) = self
                .notes_mgr
                .notes
                .iter_mut()
                .find(|n| n.id == existing_id)
        {
            note.title = plan.title.clone();
            note.content = plan.content.clone();
            note.ai_shared = true;
            self.notes_mgr.selected_note_id = Some(existing_id.clone());
            self.notes_mgr.save_selected_note();
            existing_id
        } else {
            let new_id = if let Some(folder) = &plan.folder {
                self.notes_mgr
                    .create_note_in_folder(folder, &plan.title, &plan.content)
            } else {
                self.notes_mgr.create_note(&plan.title, &plan.content)
            };
            if let Some(note) = self.notes_mgr.notes.iter_mut().find(|n| n.id == new_id) {
                note.ai_shared = true;
                note.icon = if plan.tasks.is_empty() {
                    "spark".into()
                } else {
                    "check".into()
                };
            }
            self.notes_mgr.save_selected_note();
            new_id
        };

        let mut synced_tasks = 0;
        if self.workspace.storage_ready && !plan.tasks.is_empty() {
            let mut board = self.workspace.board.clone();
            let specs: Vec<crate::ai::PlannedTaskSpec> = plan
                .tasks
                .iter()
                .map(|(title, note, status)| crate::ai::PlannedTaskSpec {
                    title: title.clone(),
                    note: note.clone(),
                    status: status.clone(),
                    ..Default::default()
                })
                .collect();
            let sched = ScheduleKind::from_str_lossy(&plan.schedule);
            let (count, first_card_id) = board.upsert_scheduled_tasks(
                &specs,
                sched,
                &plan.due_at,
                plan.interval_mins,
                plan.weekly_target,
                plan.remind,
                &plan.next_week_command,
                Some(&note_id),
            );
            if count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                self.workspace.board = board;
                synced_tasks = count;
                if let Some(card_id) = first_card_id
                    && let Some(note) = self.notes_mgr.notes.iter_mut().find(|n| n.id == note_id)
                {
                    note.task_card_id = Some(card_id);
                    self.notes_mgr.save_selected_note();
                }
            }
            if plan.remind
                || sched != ScheduleKind::None
                || !plan.due_at.trim().is_empty()
                || plan.interval_mins > 0
            {
                self.reminders_paused_by_chat = false;
            }
        }

        self.notes_open = true;
        self.workspace.preview = true;
        let summary_msg = if synced_tasks > 0 {
            format!(
                "Đã tạo ghi chú «{}» và thêm {} công việc vào Kanban.",
                plan.title, synced_tasks
            )
        } else {
            format!("Đã lưu ghi chú «{}» từ lệnh @note.", plan.title)
        };
        self.workspace.status = summary_msg.clone();
        summary_msg
    }

    /// Thực thi các hành động Note / Thư mục / Kanban Daily / Hủy lịch phát sinh từ chat tự nhiên.
    pub(super) fn apply_ai_task_actions(
        &mut self,
        actions: &[crate::ai::AiTaskAction],
    ) -> Option<String> {
        if actions.is_empty() {
            return None;
        }
        self.poll_workspace();
        let (today, _, week_idx, _) = crate::system::local_clock_now();
        let mut logs = Vec::new();
        for action in actions {
            match action {
                crate::ai::AiTaskAction::CreateFolder { folder } => {
                    if let Some(clean) = crate::notes::clean_folder(folder) {
                        self.notes_mgr.add_folder(&clean);
                        logs.push(format!("Đã tạo thư mục «{clean}»."));
                    }
                }
                crate::ai::AiTaskAction::CreatePlan {
                    folder,
                    title,
                    content,
                    schedule,
                    due_at,
                    interval_mins,
                    weekly_target,
                    remind,
                    next_week_command,
                    tasks,
                } => {
                    let clean_title = if title.trim().is_empty() {
                        tasks
                            .first()
                            .map(|t| t.title.trim())
                            .filter(|t| !t.is_empty())
                            .unwrap_or("Kế hoạch AI")
                            .to_owned()
                    } else {
                        title.trim().to_owned()
                    };
                    let mut body = content.trim().to_owned();
                    if body.is_empty() {
                        body = format!("# {clean_title}\n");
                        for t in tasks {
                            body.push_str(&format!("\n- [ ] {}", t.title.trim()));
                            if !t.note.trim().is_empty() {
                                body.push_str(&format!(": {}", t.note.trim()));
                            }
                        }
                    }
                    let note_id = if folder.trim().is_empty() {
                        self.notes_mgr.create_note(&clean_title, &body)
                    } else {
                        self.notes_mgr
                            .create_note_in_folder(folder, &clean_title, &body)
                    };
                    if let Some(note) = self.notes_mgr.notes.iter_mut().find(|n| n.id == note_id) {
                        note.ai_shared = true;
                        note.icon = "check".into();
                    }
                    self.notes_mgr.save_selected_note();

                    let sched = ScheduleKind::from_str_lossy(schedule);
                    let mut task_specs = tasks.clone();
                    if task_specs.is_empty() {
                        task_specs.push(crate::ai::PlannedTaskSpec {
                            title: clean_title.clone(),
                            note: body.lines().skip(1).collect::<Vec<_>>().join(" "),
                            status: "todo".into(),
                            ..Default::default()
                        });
                    }
                    let mut synced = 0;
                    if self.workspace.storage_ready {
                        let mut board = self.workspace.board.clone();
                        let (count, first_id) = board.upsert_scheduled_tasks(
                            &task_specs,
                            sched,
                            due_at,
                            *interval_mins,
                            *weekly_target,
                            *remind,
                            next_week_command,
                            Some(&note_id),
                        );
                        if count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                            self.workspace.board = board;
                            synced = count;
                            if let Some(card_id) = first_id
                                && let Some(note) =
                                    self.notes_mgr.notes.iter_mut().find(|n| n.id == note_id)
                            {
                                note.task_card_id = Some(card_id);
                                self.notes_mgr.save_selected_note();
                            }
                        }
                    }
                    if *remind
                        || sched != ScheduleKind::None
                        || !due_at.trim().is_empty()
                        || *interval_mins > 0
                    {
                        self.reminders_paused_by_chat = false;
                    }
                    let folder_note = if folder.trim().is_empty() {
                        String::new()
                    } else {
                        format!(" trong «{}»", folder.trim())
                    };
                    logs.push(format!(
                        "Đã tạo ghi chú «{clean_title}»{folder_note} và {synced} việc Kanban."
                    ));
                }
                crate::ai::AiTaskAction::CancelSchedule { query } => {
                    if self.workspace.storage_ready {
                        let mut board = self.workspace.board.clone();
                        let cancelled = board.cancel_matching_schedules(query);
                        if cancelled > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                            self.workspace.board = board;
                            logs.push(format!("Đã hủy lịch nhắc cho {cancelled} công việc."));
                        } else {
                            logs.push("Không tìm thấy lịch đang bật khớp yêu cầu để hủy.".into());
                        }
                    }
                }
                crate::ai::AiTaskAction::CompleteTask { query } => {
                    if self.workspace.storage_ready {
                        let mut board = self.workspace.board.clone();
                        let done_count = board.complete_matching_tasks(query, &today, week_idx);
                        if done_count > 0 && board.save(&self.notes_mgr.root_dir).is_ok() {
                            self.workspace.board = board;
                            logs.push(format!(
                                "Đã đánh dấu hoàn thành {done_count} công việc (sẽ không nhắc lại hôm nay)."
                            ));
                        }
                    }
                }
                crate::ai::AiTaskAction::ConfigureReminders {
                    startup,
                    interval_mins,
                    focus_alert_mins,
                    proactive_rhythm,
                    paused,
                } => {
                    let mut notes = Vec::new();
                    if let Some(on) = startup {
                        self.prefs.startup_reminder = *on;
                        notes.push(if *on {
                            "bật nhắc khi mở máy".to_owned()
                        } else {
                            "tắt nhắc khi mở máy".to_owned()
                        });
                    }
                    if let Some(mins) = interval_mins {
                        self.prefs.default_reminder_mins = (*mins).min(1440);
                        notes.push(if *mins == 0 {
                            "chỉ nhắc khi mở máy".to_owned()
                        } else {
                            format!("nhắc mặc định mỗi {mins} phút")
                        });
                    }
                    if let Some(mins) = focus_alert_mins {
                        self.prefs.edge_tts.focus_alert_mins = (*mins).clamp(15, 240);
                        notes.push(format!(
                            "nhắc nhịp làm việc sau {} phút không nhạc",
                            self.prefs.edge_tts.focus_alert_mins
                        ));
                    }
                    if let Some(on) = proactive_rhythm {
                        self.prefs.edge_tts.proactive_work_rhythm = *on;
                        notes.push(if *on {
                            "bật phân tích nhịp làm việc".to_owned()
                        } else {
                            "tắt phân tích nhịp làm việc".to_owned()
                        });
                    }
                    if *paused == Some(true) {
                        self.reminders_paused_by_chat = true;
                        self.reminder_job = None;
                        self.work_rhythm_job = None;
                        self.tts_player.stop();
                        notes.push("đã tạm dừng nhắc nhở tự động".to_owned());
                    } else if *paused == Some(false)
                        || *startup == Some(true)
                        || interval_mins.is_some()
                        || focus_alert_mins.is_some()
                        || *proactive_rhythm == Some(true)
                    {
                        self.reminders_paused_by_chat = false;
                    }
                    if notes.is_empty() {
                        logs.push("Đã cập nhật thiết lập nhắc nhở qua AI.".into());
                    } else {
                        logs.push(format!("Đã cập nhật nhắc nhở: {}.", notes.join(", ")));
                    }
                }
            }
        }
        if logs.is_empty() {
            None
        } else {
            let joined = logs.join(" ");
            self.workspace.status = joined.clone();
            Some(joined)
        }
    }

    /// Kích hoạt AI thống kê các công việc người dùng làm và tổng hợp vào ghi chú.
    pub(super) fn trigger_ai_work_stats(&mut self, ctx: &egui::Context) {
        self.poll_workspace();
        let stats_context = self.workspace.board.ai_context_summary();
        self.workspace.pending_note_target_id = None;
        self.ai_prompt = format!(
            "@note Thống kê các công việc tôi đang làm, đã hoàn tất và cần làm tiếp theo dựa trên bảng công việc hiện tại. Phân tích ngắn gọn tiến độ và tạo danh sách việc ưu tiên tiếp theo.\n{stats_context}"
        );
        self.ai_open = true;
        self.send_ai_prompt(ctx);
    }

    pub(super) fn note_inline_ai_bar(&mut self, ui: &mut egui::Ui, target_note_id: Option<String>) {
        if !self.workspace.ai_note_bar_open {
            return;
        }
        let busy = self.ai_job.is_some();
        egui::Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                    ui.label(
                        egui::RichText::new("@note")
                            .size(11.0)
                            .strong()
                            .color(egui::Color32::from_rgb(100, 220, 175)),
                    );
                    let width = (ui.available_width() - 56.0).max(80.0);
                    let resp = ui.add_sized(
                        [width, 22.0],
                        egui::TextEdit::singleline(&mut self.workspace.ai_note_input)
                            .frame(egui::Frame::NONE)
                            .font(egui::FontId::proportional(12.0))
                            .hint_text("Nhập yêu cầu để AI viết ghi chú & tạo công việc…"),
                    );
                    let enter = resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.is_none());
                    let can_run = !busy && !self.workspace.ai_note_input.trim().is_empty();
                    if theme::mini_icon(
                        ui,
                        theme::Glyph::Arrow,
                        "Gửi lệnh @note tạo ghi chú & công việc (Enter)",
                        can_run,
                        false,
                    )
                    .clicked()
                        || (enter && can_run)
                    {
                        let raw = self.workspace.ai_note_input.trim().to_string();
                        self.workspace.ai_note_input.clear();
                        self.workspace.pending_note_target_id = target_note_id;
                        self.ai_prompt = if crate::ai::note_command(&raw).is_some() {
                            raw
                        } else {
                            format!("@note {raw}")
                        };
                        self.send_ai_prompt(ui.ctx());
                    }
                    if theme::mini_icon(
                        ui,
                        theme::Glyph::Chart,
                        "AI thống kê tiến độ các việc đang làm",
                        !busy,
                        false,
                    )
                    .clicked()
                    {
                        self.trigger_ai_work_stats(ui.ctx());
                    }
                });
            });
    }

    pub(super) fn kanban_ui(
        &mut self,
        ui: &mut egui::Ui,
    ) -> egui::scroll_area::ScrollAreaOutput<()> {
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        egui::ScrollArea::vertical()
            .id_salt("kanban-page")
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.kanban_contents_ui(ui);
            })
    }

    fn kanban_contents_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
        ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
        self.poll_workspace();

        let (total, todo_c, doing_c, done_c) = self.workspace.board.stats();
        let pct = (done_c * 100).checked_div(total).unwrap_or(0);
        let busy = self.ai_job.is_some();

        // Hàng điều khiển Kanban thu gọn cỡ chữ nhỏ + Icon mini
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            ui.label(egui::RichText::new("Công việc").size(12.5).strong());
            ui.label(
                egui::RichText::new(format!("· {total} việc ({pct}% hoàn tất)"))
                    .size(10.5)
                    .color(theme::MUTED),
            );
            let form_label = if self.workspace.new_card_open {
                "− Thu gọn"
            } else {
                "+ Công việc mới"
            };
            if ui
                .selectable_label(
                    self.workspace.new_card_open,
                    egui::RichText::new(form_label).size(11.0),
                )
                .clicked()
            {
                self.workspace.new_card_open = !self.workspace.new_card_open;
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Spark,
                "Lệnh @note: AI viết ghi chú & tạo danh sách công việc",
                true,
                self.workspace.ai_note_bar_open,
            )
            .clicked()
            {
                self.workspace.ai_note_bar_open = !self.workspace.ai_note_bar_open;
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Chart,
                "AI thống kê các việc người dùng làm",
                !busy,
                false,
            )
            .clicked()
            {
                self.trigger_ai_work_stats(ui.ctx());
            }
            if theme::mini_icon(
                ui,
                theme::Glyph::Archive,
                "Chỉ xem thẻ đã lưu trữ",
                true,
                self.workspace.show_archived,
            )
            .clicked()
            {
                self.workspace.show_archived = !self.workspace.show_archived;
            }
        });

        self.note_inline_ai_bar(ui, None);

        if !self.workspace.storage_ready {
            theme::caption(ui, &self.workspace.status);
            if ui.small_button("Thử đọc lại").clicked() {
                self.workspace.loaded_root = None;
            }
            return;
        }
        if self.workspace.new_card_open {
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .corner_radius(8)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.workspace.new_title)
                            .font(egui::FontId::proportional(11.5))
                            .hint_text("Tiêu đề công việc")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut self.workspace.new_content)
                            .font(egui::FontId::proportional(11.0))
                            .hint_text("Mô tả ngắn (đặt lịch/nhắc nhở bằng cách nhắn với AI)")
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                !self.workspace.new_title.trim().is_empty(),
                                egui::Button::new("Thêm công việc"),
                            )
                            .clicked()
                            && self.board_request(Request::Create {
                                title: self.workspace.new_title.clone(),
                                content: self.workspace.new_content.clone(),
                            })
                        {
                            self.workspace.new_title.clear();
                            self.workspace.new_content.clear();
                            self.workspace.new_card_open = false;
                        }
                        if ui.small_button("Hủy").clicked() {
                            self.workspace.new_card_open = false;
                        }
                    });
                });
        }
        if let Some(mut card) = self.workspace.editing.take() {
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .corner_radius(8)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Sửa công việc").size(11.5).strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut card.title)
                            .font(egui::FontId::proportional(11.5))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut card.note)
                            .font(egui::FontId::proportional(11.0))
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    let mut close = false;
                    ui.horizontal(|ui| {
                        if ui.small_button("Lưu").clicked() {
                            close = self.board_request(Request::Edit(card.clone()));
                        }
                        if ui.small_button("Hủy").clicked() {
                            close = true;
                        }
                    });
                    if !close {
                        self.workspace.editing = Some(card);
                    }
                });
        }
        if !self.workspace.status.is_empty() {
            ui.label(
                egui::RichText::new(&self.workspace.status)
                    .size(10.0)
                    .color(theme::MUTED),
            );
        }

        let cards = self.workspace.board.cards.clone();
        let mut operation = None;

        // Gom tất cả các công việc nằm gọn trong 1 Section tối ưu duy nhất
        egui::Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(10)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Thanh lọc trạng thái chữ nhỏ ngay đầu section
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    let all_count = cards
                        .iter()
                        .filter(|c| c.archived == self.workspace.show_archived)
                        .count();
                    ui.selectable_value(
                        &mut self.workspace.narrow_column,
                        3,
                        egui::RichText::new(format!("Tất cả ({all_count})")).size(11.0),
                    );
                    for (index, (status, label)) in COLUMNS.iter().enumerate() {
                        let count = cards
                            .iter()
                            .filter(|card| {
                                card.status == *status
                                    && card.archived == self.workspace.show_archived
                            })
                            .count();
                        ui.selectable_value(
                            &mut self.workspace.narrow_column,
                            index,
                            egui::RichText::new(format!("{label} ({count})")).size(11.0),
                        );
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "Cần làm {todo_c} · Đang làm {doing_c} · Xong {done_c}"
                        ))
                        .size(10.0)
                        .color(theme::MUTED),
                    );
                });
                ui.separator();

                let groups: Vec<(&str, &str)> = match self.workspace.narrow_column {
                    0 => vec![COLUMNS[0]],
                    1 => vec![COLUMNS[1]],
                    2 => vec![COLUMNS[2]],
                    _ => COLUMNS.to_vec(),
                };

                for (status, label) in groups {
                    let group_cards: Vec<_> = cards
                        .iter()
                        .filter(|c| {
                            c.status == status && c.archived == self.workspace.show_archived
                        })
                        .collect();
                    let (_, dropped) = ui.dnd_drop_zone::<String, _>(
                        egui::Frame::new()
                            .fill(theme::PANEL)
                            .corner_radius(6)
                            .inner_margin(6),
                        |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new(format!("{} · {}", label, group_cards.len()))
                                    .size(11.5)
                                    .strong()
                                    .color(theme::MUTED),
                            );
                            if group_cards.is_empty() {
                                ui.label(
                                    egui::RichText::new("Chưa có mục nào · kéo thả thẻ vào đây")
                                        .size(10.0)
                                        .color(theme::MUTED),
                                );
                            }
                            for card in group_cards {
                                ui.push_id(&card.id, |ui| {
                                    egui::Frame::new()
                                        .fill(theme::SIDEBAR)
                                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                                        .corner_radius(6)
                                        .inner_margin(6)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.dnd_drag_source(
                                                egui::Id::new(("kanban-card", &card.id)),
                                                card.id.clone(),
                                                |ui| {
                                                    ui.set_max_width(
                                                        (ui.available_width() - 8.0).max(120.0),
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(&card.title)
                                                                .size(11.5)
                                                                .strong(),
                                                        )
                                                        .wrap(),
                                                    );
                                                    let snippet: String =
                                                        card.note.chars().take(100).collect();
                                                    if !snippet.trim().is_empty() {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(snippet)
                                                                    .size(10.5)
                                                                    .color(theme::MUTED),
                                                            )
                                                            .wrap(),
                                                        );
                                                    }
                                                    if let Some(badge) = card.schedule_badge() {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(badge)
                                                                    .size(10.0)
                                                                    .color(
                                                                        egui::Color32::from_rgb(
                                                                            100, 220, 175,
                                                                        ),
                                                                    ),
                                                            )
                                                            .wrap(),
                                                        );
                                                    }
                                                },
                                            );
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(4.0, 2.0);
                                                egui::ComboBox::from_id_salt(("status", &card.id))
                                                    .width(86.0)
                                                    .selected_text(
                                                        egui::RichText::new(label).size(10.5),
                                                    )
                                                    .show_ui(ui, |ui| {
                                                        for (next, title) in COLUMNS {
                                                            if ui
                                                                .selectable_label(
                                                                    card.status == next,
                                                                    egui::RichText::new(title)
                                                                        .size(11.0),
                                                                )
                                                                .clicked()
                                                            {
                                                                operation = Some(Request::Move {
                                                                    card_id: card.id.clone(),
                                                                    status: next.into(),
                                                                });
                                                            }
                                                        }
                                                    });
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::Edit,
                                                    "Sửa công việc",
                                                    true,
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    self.workspace.editing = Some(card.clone());
                                                }
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::Archive,
                                                    if card.archived {
                                                        "Khôi phục thẻ"
                                                    } else {
                                                        "Lưu trữ thẻ"
                                                    },
                                                    true,
                                                    card.archived,
                                                )
                                                .clicked()
                                                {
                                                    operation = Some(Request::Archive {
                                                        card_id: card.id.clone(),
                                                        archived: !card.archived,
                                                    });
                                                }
                                                if theme::mini_icon(
                                                    ui,
                                                    theme::Glyph::File,
                                                    "Mở ghi chú liên kết",
                                                    true,
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    if let Some(note) =
                                                        self.notes_mgr.notes.iter().find(|note| {
                                                            note.task_card_id.as_ref()
                                                                == Some(&card.id)
                                                        })
                                                    {
                                                        self.notes_mgr.selected_note_id =
                                                            Some(note.id.clone());
                                                    } else {
                                                        let id = self.notes_mgr.create_note(
                                                            &card.title,
                                                            &card_content(card),
                                                        );
                                                        if let Some(note) = self
                                                            .notes_mgr
                                                            .notes
                                                            .iter_mut()
                                                            .find(|note| note.id == id)
                                                        {
                                                            note.task_card_id =
                                                                Some(card.id.clone());
                                                        }
                                                        self.notes_mgr.save_selected_note();
                                                    }
                                                    self.workspace.board_open = false;
                                                }
                                            });
                                        });
                                });
                            }
                        },
                    );
                    if let Some(id) = dropped {
                        operation = Some(Request::Move {
                            card_id: (*id).clone(),
                            status: status.into(),
                        });
                    }
                }
            });

        if let Some(operation) = operation {
            self.board_request(operation);
        }
    }
}

pub fn format_selection(content: &mut String, start: usize, end: usize, marker: &str) {
    let offset = |index| {
        content
            .char_indices()
            .nth(index)
            .map_or(content.len(), |(offset, _)| offset)
    };
    let mut first = offset(start.min(end));
    let last = offset(start.max(end));
    if marker.starts_with('#') {
        first = content[..first].rfind('\n').map_or(0, |index| index + 1);
        let existing = content[first..]
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if existing > 0 && content[first + existing..].starts_with(' ') {
            content.replace_range(first..first + existing + 1, marker);
        } else {
            content.insert_str(first, marker);
        }
    } else {
        let selected = if first == last {
            "văn bản".to_owned()
        } else {
            content[first..last].to_owned()
        };
        content.replace_range(first..last, &format!("{marker}{selected}{marker}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_kanban_forms_stay_inside_resized_viewport_and_collapse_after_create() {
        let ctx = egui::Context::default();
        let mut app = PentaKuru::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            None,
            true,
        );
        let root = std::env::temp_dir().join(format!("penta-kanban-layout-{}", std::process::id()));
        app.notes_mgr = crate::notes::NotesManager::at(root.clone());
        app.poll_workspace();
        app.workspace.new_card_open = true;
        app.workspace.new_title = "Công việc mới".into();
        app.workspace.new_content = "Mô tả công việc".into();
        app.workspace.board.cards = (0..12)
            .map(|i| Card {
                id: format!("card-{i}"),
                title: "Tiêu đề dài khi thu nhỏ cửa sổ ghi chú".repeat(4),
                note: "Nội dung công việc cần được xuống dòng.".repeat(4),
                status: "todo".into(),
                archived: false,
                ..Default::default()
            })
            .collect();
        let render = |app: &mut PentaKuru, size: egui::Vec2, events| {
            let mut bounds = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let page = app.kanban_ui(ui);
                    bounds = Some((page.inner_rect, page.content_size));
                },
            );
            output.textures_delta.clear();
            (output, bounds.unwrap())
        };
        for size in [
            egui::vec2(380.0, 460.0),
            egui::vec2(480.0, 560.0),
            egui::vec2(900.0, 620.0),
        ] {
            for editing in [false, true] {
                app.workspace.editing = editing.then(|| app.workspace.board.cards[0].clone());
                let (_, (viewport, content)) = render(&mut app, size, vec![]);
                assert!(viewport.right() <= size.x + 1.0 && viewport.bottom() <= size.y + 1.0);
                assert!(
                    content.x <= viewport.width() + 1.0,
                    "Horizontal overflow at {size:?}: {content:?}, {viewport:?}"
                );
                assert!(content.y > viewport.height(), "Long board must scroll");
            }
        }
        app.workspace.editing = None;
        app.workspace.board.cards.clear();
        let (output, _) = render(&mut app, egui::vec2(480.0, 560.0), vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Thêm công việc")
                        .then(|| text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("Create action must remain visible inside the form");
        for pressed in [true, false] {
            render(
                &mut app,
                egui::vec2(480.0, 560.0),
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(
            !app.workspace.new_card_open,
            "Successful creation must collapse the form"
        );
        assert_eq!(Board::load(&root).unwrap().cards[0].title, "Công việc mới");
        // Remove only this test's own files, without a recursive directory delete.
        for entry in std::fs::read_dir(&root).unwrap().flatten() {
            if entry.file_type().unwrap().is_dir() {
                for file in std::fs::read_dir(entry.path()).unwrap().flatten() {
                    std::fs::remove_file(file.path()).unwrap();
                }
                std::fs::remove_dir(entry.path()).unwrap();
            } else {
                std::fs::remove_file(entry.path()).unwrap();
            }
        }
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn formatting_preserves_vietnamese_and_replaces_heading_level() {
        let mut text = "Học tiếng Việt".to_owned();
        format_selection(&mut text, 9, 4, "**");
        assert_eq!(text, "Học **tiếng** Việt");
        format_selection(&mut text, 0, 0, "# ");
        format_selection(&mut text, 3, 3, "## ");
        assert_eq!(text, "## Học **tiếng** Việt");
        let mut empty = String::new();
        format_selection(&mut empty, 0, 0, "*");
        assert_eq!(empty, "*văn bản*");
    }

    #[test]
    fn local_board_persists_note_links_moves_edits_and_archive() {
        let root = std::env::temp_dir().join(format!("penta-kanban-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut board = Board::load(&root).unwrap();
        board.cards.clear();
        let link = board
            .apply(Request::FromNote {
                note_id: "note-1".into(),
                card_id: Some("old-remote-id".into()),
                title: "Công việc".into(),
                content: "Nội dung".repeat(2000),
            })
            .unwrap()
            .unwrap();
        assert_eq!(link.0, "note-1");
        let id = link.1;
        board
            .apply(Request::Move {
                card_id: id.clone(),
                status: "doing".into(),
            })
            .unwrap();
        board
            .apply(Request::FromNote {
                note_id: "note-1".into(),
                card_id: Some(id.clone()),
                title: "Đã cập nhật".into(),
                content: "Nội dung mới".into(),
            })
            .unwrap();
        assert_eq!(board.cards.len(), 1);
        assert_eq!(board.cards[0].status, "doing");
        board.save(&root).unwrap();
        let mut reloaded = Board::load(&root).unwrap();
        assert_eq!(reloaded.cards, board.cards);
        reloaded
            .apply(Request::Archive {
                card_id: id,
                archived: true,
            })
            .unwrap();
        reloaded.save(&root).unwrap();
        assert!(Board::load(&root).unwrap().cards[0].archived);
        std::fs::remove_file(root.join(".kanban.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn corrupt_board_is_reported_and_invalid_moves_are_rejected() {
        let root =
            std::env::temp_dir().join(format!("penta-kanban-invalid-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".kanban.json"), "broken").unwrap();
        assert!(Board::load(&root).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join(".kanban.json")).unwrap(),
            "broken"
        );
        let mut board = Board::default();
        assert!(board.create("  ".into(), "".into()).is_err());
        let id = board.create("Task".into(), "".into()).unwrap();
        assert!(
            board
                .apply(Request::Move {
                    card_id: id,
                    status: "invalid".into()
                })
                .is_err()
        );
        assert_eq!(board.cards[0].status, "todo");
        std::fs::remove_file(root.join(".kanban.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn flexible_schedules_exclude_completed_and_rollover_weekly_cycles() {
        let mut board = Board::default();
        board.upsert_scheduled_tasks(
            &[
                crate::ai::PlannedTaskSpec {
                    title: "Luyện cơ bắp".into(),
                    note: "Hít đất & kéo xà".into(),
                    ..Default::default()
                },
                crate::ai::PlannedTaskSpec {
                    title: "Nộp báo cáo".into(),
                    note: "Gửi PDF".into(),
                    schedule: "none".into(),
                    due_at: "17:00".into(),
                    interval_mins: 30,
                    remind: Some(true),
                    ..Default::default()
                },
            ],
            ScheduleKind::Daily,
            "19:30",
            0,
            6,
            true,
            "Tăng độ khó tuần mới",
            None,
        );
        assert_eq!(board.cards.len(), 2);

        // Before 19:15, card 0 with explicit due_at 19:30 & interval=0 stays quiet,
        // while card 1 with interval=30m is due once interval passes.
        let due_afternoon = board.due_reminder_cards("2026-11-18", 14 * 60, 10_000, 60, false);
        assert_eq!(due_afternoon, vec![1]);

        // Approaching 19:30 (19:20) triggers timed card 0 too.
        let due_evening = board.due_reminder_cards("2026-11-18", 19 * 60 + 20, 10_000, 60, false);
        assert_eq!(due_evening, vec![0, 1]);

        // Completing a task stops it from being reminded or sent up to AI context.
        board.cards[0].mark_done("2026-11-18", 2968);
        board.cards[1].mark_done("2026-11-18", 2968);
        assert!(
            board
                .due_reminder_cards("2026-11-18", 19 * 60 + 25, 20_000, 60, true)
                .is_empty()
        );
        let summary = board.ai_context_summary();
        assert!(!summary.contains("Luyện cơ bắp"));
        assert!(!summary.contains("Nộp báo cáo"));

        // New day in same week reopens Daily card 0 only; regular card 1 stays done.
        let (changed, rollovers) = board.rollover_schedules("2026-11-19", 2968);
        assert!(changed);
        assert!(rollovers.is_empty());
        assert_eq!(board.cards[0].status, "todo");
        assert_eq!(board.cards[1].status, "done");

        // Crossing into the next week triggers WeeklyRollover when next_week_command is set.
        let (changed_w, rollovers_w) = board.rollover_schedules("2026-11-23", 2969);
        assert!(changed_w);
        assert_eq!(rollovers_w.len(), 1);
        assert_eq!(rollovers_w[0].next_week_cycle, 2);

        // Cancelling a schedule disables both Daily repetition and reminders cleanly.
        assert_eq!(board.cancel_matching_schedules("cơ bắp"), 1);
        assert_eq!(board.cards[0].schedule, ScheduleKind::None);
        assert!(!board.cards[0].remind_enabled);
    }
}
