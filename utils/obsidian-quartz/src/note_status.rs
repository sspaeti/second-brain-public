//! Per-note maturity level ("started" / "growing" / "evergreen") for the
//! public second brain, derived from git edit history and word count, or
//! overridden by a `#garden/<level>` tag in the vault.
//!
//! Spec: docs/superpowers/specs/2026-10-01-note-status-design.md
//!
//! Runs as `obsidian-quartz note-status` in `make prepare`, right after
//! `utils/recent_updates.py` (whose data/recent_updates.json it consumes).
//! Writes data/note_status.json (read by Hugo) and
//! data/note_status_report.md (read by the author, gitignored).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Started,
    Growing,
    Evergreen,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Started => "started",
            Level::Growing => "growing",
            Level::Evergreen => "evergreen",
        }
    }

    /// Case-insensitive; anything else is None.
    pub fn parse(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "started" => Some(Level::Started),
            "growing" => Some(Level::Growing),
            "evergreen" => Some(Level::Evergreen),
            _ => None,
        }
    }

    /// Ordering for "how far apart are two levels" in the report.
    pub fn rank(&self) -> u8 {
        match self {
            Level::Started => 0,
            Level::Growing => 1,
            Level::Evergreen => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Git,
    Manual,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Git => "git",
            Source::Manual => "manual",
        }
    }
}

/// Thresholds from config.toml [params] (noteStatus*). Defaults mirror the spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thresholds {
    pub evergreen_words: usize,
    pub evergreen_sessions: usize,
    pub evergreen_span_days: i64,
    pub started_words: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            evergreen_words: 600,
            evergreen_sessions: 5,
            evergreen_span_days: 180,
            started_words: 120,
        }
    }
}

/// Everything the rules need about one note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteFacts {
    pub words: usize,
    pub sessions: usize,
    pub span_days: i64,
    pub manual: Option<Level>,
}

/// The rules, in spec order: manual tag, evergreen, started, else growing.
pub fn classify(f: &NoteFacts, t: &Thresholds) -> (Level, Source) {
    if let Some(level) = f.manual {
        return (level, Source::Manual);
    }
    if f.words >= t.evergreen_words
        && f.sessions >= t.evergreen_sessions
        && f.span_days >= t.evergreen_span_days
    {
        return (Level::Evergreen, Source::Git);
    }
    if f.words < t.started_words && f.sessions <= 1 {
        return (Level::Started, Source::Git);
    }
    (Level::Growing, Source::Git)
}

/// "3,120"
pub fn with_commas(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn plural(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {}", unit)
    } else {
        format!("{} {}s", n, unit)
    }
}

/// "1 day", "12 days", "3 months", "2 years" (floored; < 1 day reads "1 day").
pub fn humanize_span(days: i64) -> String {
    if days < 1 {
        "1 day".to_string()
    } else if days < 30 {
        plural(days, "day")
    } else if days < 365 {
        plural(days / 30, "month")
    } else {
        plural(days / 365, "year")
    }
}

/// The sentence shown under the level in the popover. Built here so the Hugo
/// template only prints it.
pub fn reason(f: &NoteFacts, source: Source) -> String {
    if source == Source::Manual {
        return "Set by author".to_string();
    }
    let history = match f.sessions {
        0 => "no edits recorded".to_string(),
        1 => "1 session".to_string(),
        n => format!("{} sessions over {}", n, humanize_span(f.span_days)),
    };
    format!(
        "Estimated from edit history · {} · {} words",
        history,
        with_commas(f.words)
    )
}

/// Words in a note body: frontmatter stripped, split on whitespace.
/// Mirrors `_word_count` in utils/recent_updates.py so both tools agree.
pub fn word_count(text: &str) -> usize {
    strip_frontmatter(text).split_whitespace().count()
}

/// Everything after the closing `---` of a leading YAML frontmatter block.
pub fn strip_frontmatter(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return &rest[end + 4..];
        }
    }
    text
}

use chrono::NaiveDate;

/// The frontmatter keys note-status cares about, read with a line scan (the
/// published frontmatter is flat `key: value`; no YAML parser needed and the
/// `title: "\"x\""` quirk in file_utils.rs can't trip it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontmatterFields {
    /// Some(level) only when `status_source: manual` AND `status:` is a level.
    pub manual: Option<Level>,
    pub createddate: Option<NaiveDate>,
    pub lastmod: Option<NaiveDate>,
}

fn frontmatter_block(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .or_else(|| s.strip_prefix('"').and_then(|x| x.strip_suffix('"')))
        .unwrap_or(s)
}

/// `2026-09-23 12:16:49`, `2026-09-23`, `'2026-09-23'` -> the date part.
fn date_prefix(s: &str) -> Option<NaiveDate> {
    let s = unquote(s);
    if s.len() < 10 {
        return None;
    }
    NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()
}

pub fn frontmatter_fields(text: &str) -> FrontmatterFields {
    let mut status: Option<String> = None;
    let mut source: Option<String> = None;
    let mut out = FrontmatterFields { manual: None, createddate: None, lastmod: None };
    if let Some(block) = frontmatter_block(text) {
        for line in block.lines() {
            let Some((key, value)) = line.split_once(':') else { continue };
            match key.trim() {
                "status" => status = Some(unquote(value).to_string()),
                "status_source" => source = Some(unquote(value).to_string()),
                "createddate" => out.createddate = date_prefix(value),
                "lastmod" => out.lastmod = date_prefix(value),
                _ => {}
            }
        }
    }
    if source.as_deref() == Some("manual") {
        out.manual = status.as_deref().and_then(Level::parse);
    }
    out
}

/// (session count, days between the newest and oldest session) for one
/// data/recent_updates.json entry. Rows are newest-first; every row is a
/// session, whether it carries `added`/`removed` or `kind: published`.
pub fn sessions_facts(entry: Option<&serde_json::Value>) -> (usize, i64) {
    let Some(rows) = entry.and_then(|e| e.get("sessions")).and_then(|s| s.as_array()) else {
        return (0, 0);
    };
    let dates: Vec<NaiveDate> = rows
        .iter()
        .filter_map(|r| r.get("iso").and_then(|v| v.as_str()))
        .filter_map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .collect();
    let span = match (dates.iter().max(), dates.iter().min()) {
        (Some(newest), Some(oldest)) => (*newest - *oldest).num_days(),
        _ => 0,
    };
    (rows.len(), span.max(0))
}

/// Thresholds from the site's config.toml ([params] noteStatus*). Anything
/// missing or unparsable keeps the default, so a typo never silently
/// reclassifies the whole site to one level.
pub fn thresholds_from_toml(text: &str) -> Thresholds {
    let mut t = Thresholds::default();
    let Ok(doc) = text.parse::<toml::Table>() else { return t };
    let Some(params) = doc.get("params").and_then(|p| p.as_table()) else { return t };
    let int = |key: &str| params.get(key).and_then(|v| v.as_integer()).filter(|n| *n >= 0);
    if let Some(n) = int("noteStatusEvergreenWords") {
        t.evergreen_words = n as usize;
    }
    if let Some(n) = int("noteStatusEvergreenSessions") {
        t.evergreen_sessions = n as usize;
    }
    if let Some(n) = int("noteStatusEvergreenSpanDays") {
        t.evergreen_span_days = n;
    }
    if let Some(n) = int("noteStatusStartedWords") {
        t.started_words = n as usize;
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(words: usize, sessions: usize, span_days: i64) -> NoteFacts {
        NoteFacts { words, sessions, span_days, manual: None }
    }

    #[test]
    fn level_parse_is_case_insensitive_and_strict() {
        assert_eq!(Level::parse("Growing"), Some(Level::Growing));
        assert_eq!(Level::parse(" EVERGREEN "), Some(Level::Evergreen));
        assert_eq!(Level::parse("started"), Some(Level::Started));
        assert_eq!(Level::parse("tree"), None);
        assert_eq!(Level::parse(""), None);
    }

    #[test]
    fn manual_level_wins_over_everything() {
        let t = Thresholds::default();
        let mut f = facts(5000, 50, 900);
        f.manual = Some(Level::Started);
        assert_eq!(classify(&f, &t), (Level::Started, Source::Manual));
    }

    #[test]
    fn evergreen_needs_all_three_thresholds() {
        let t = Thresholds::default();
        assert_eq!(classify(&facts(600, 5, 180), &t), (Level::Evergreen, Source::Git));
        assert_eq!(classify(&facts(599, 5, 180), &t), (Level::Growing, Source::Git));
        assert_eq!(classify(&facts(600, 4, 180), &t), (Level::Growing, Source::Git));
        assert_eq!(classify(&facts(600, 5, 179), &t), (Level::Growing, Source::Git));
    }

    #[test]
    fn started_is_short_and_untouched() {
        let t = Thresholds::default();
        assert_eq!(classify(&facts(95, 1, 0), &t), (Level::Started, Source::Git));
        assert_eq!(classify(&facts(95, 0, 0), &t), (Level::Started, Source::Git));
        // a second session or enough words makes it growing
        assert_eq!(classify(&facts(95, 2, 3), &t), (Level::Growing, Source::Git));
        assert_eq!(classify(&facts(120, 1, 0), &t), (Level::Growing, Source::Git));
    }

    #[test]
    fn thresholds_are_not_hardcoded() {
        let t = Thresholds { evergreen_words: 10, evergreen_sessions: 1, evergreen_span_days: 0, started_words: 1 };
        assert_eq!(classify(&facts(10, 1, 0), &t), (Level::Evergreen, Source::Git));
    }

    #[test]
    fn reason_strings_match_the_spec() {
        assert_eq!(reason(&facts(95, 1, 0), Source::Manual), "Set by author");
        assert_eq!(
            reason(&facts(95, 1, 0), Source::Git),
            "Estimated from edit history · 1 session · 95 words"
        );
        assert_eq!(
            reason(&facts(3120, 23, 753), Source::Git),
            "Estimated from edit history · 23 sessions over 2 years · 3,120 words"
        );
        assert_eq!(
            reason(&facts(40, 0, 0), Source::Git),
            "Estimated from edit history · no edits recorded · 40 words"
        );
        assert_eq!(
            reason(&facts(400, 2, 0), Source::Git),
            "Estimated from edit history · 2 sessions over 1 day · 400 words"
        );
    }

    #[test]
    fn humanize_span_floors_units() {
        assert_eq!(humanize_span(0), "1 day");
        assert_eq!(humanize_span(1), "1 day");
        assert_eq!(humanize_span(29), "29 days");
        assert_eq!(humanize_span(30), "1 month");
        assert_eq!(humanize_span(364), "12 months");
        assert_eq!(humanize_span(365), "1 year");
        assert_eq!(humanize_span(753), "2 years");
    }

    #[test]
    fn with_commas_groups_thousands() {
        assert_eq!(with_commas(0), "0");
        assert_eq!(with_commas(999), "999");
        assert_eq!(with_commas(1000), "1,000");
        assert_eq!(with_commas(3120), "3,120");
        assert_eq!(with_commas(1234567), "1,234,567");
    }

    #[test]
    fn word_count_strips_frontmatter_and_splits_on_whitespace() {
        let note = "---\ntitle: \"X\"\ntags: [a]\n---\nCheck with `lsblk`\nwhat  drive\n\n```\nsudo umount\n```\n";
        assert_eq!(word_count(note), 9); // Check with `lsblk` what drive ``` sudo umount ```
        assert_eq!(word_count("no frontmatter here"), 3);
        assert_eq!(word_count("---\ntitle: x\n---\n"), 0);
        assert_eq!(word_count(""), 0);
    }

    #[test]
    fn frontmatter_fields_reads_status_only_with_manual_source() {
        let manual = "---\ncreateddate: 2024-08-31\nlastmod: 2026-09-23 12:16:49\nstatus: evergreen\nstatus_source: manual\ntitle: \"x\"\n---\nbody";
        let f = frontmatter_fields(manual);
        assert_eq!(f.manual, Some(Level::Evergreen));
        assert_eq!(f.createddate, chrono::NaiveDate::from_ymd_opt(2024, 8, 31));
        assert_eq!(f.lastmod, chrono::NaiveDate::from_ymd_opt(2026, 9, 23));

        // a hand-typed `status:` without status_source is NOT an override
        let typed = "---\nstatus: evergreen\n---\nbody";
        assert_eq!(frontmatter_fields(typed).manual, None);

        // quoted dates and a status_source other than manual
        let quoted = "---\ncreateddate: '2026-10-01'\nstatus: growing\nstatus_source: git\n---\n";
        let f = frontmatter_fields(quoted);
        assert_eq!(f.manual, None);
        assert_eq!(f.createddate, chrono::NaiveDate::from_ymd_opt(2026, 10, 1));
        assert_eq!(f.lastmod, None);

        // no frontmatter at all
        let f = frontmatter_fields("just text");
        assert_eq!(f, FrontmatterFields { manual: None, createddate: None, lastmod: None });
    }

    #[test]
    fn sessions_facts_counts_rows_and_spans_newest_to_oldest() {
        let entry: serde_json::Value = serde_json::json!({
            "status": "updated", "words": 107,
            "sessions": [
                {"date": "Sep 23", "iso": "2026-09-23", "rel": "8 days ago", "added": 212, "removed": 40},
                {"date": "Aug 28", "iso": "2026-08-28", "rel": "1 month ago", "added": 1004, "removed": 117},
                {"date": "Aug 31, 2024", "iso": "2024-08-31", "rel": "2 years ago", "kind": "published", "words": 1420}
            ]
        });
        assert_eq!(sessions_facts(Some(&entry)), (3, 753));

        let one: serde_json::Value = serde_json::json!({"status": "new", "words": 95,
            "sessions": [{"date": "Oct 1", "iso": "2026-10-01", "rel": "today", "kind": "published", "words": 95}]});
        assert_eq!(sessions_facts(Some(&one)), (1, 0));

        // entry without sessions (zero-word edits only), and no entry at all
        let none: serde_json::Value = serde_json::json!({"status": "updated", "words": 0});
        assert_eq!(sessions_facts(Some(&none)), (0, 0));
        assert_eq!(sessions_facts(None), (0, 0));
    }

    #[test]
    fn thresholds_from_toml_reads_params_and_falls_back() {
        let cfg = "baseURL = \"x\"\n[params]\nauthor = \"S\"\nnoteStatusEvergreenWords = 700\nnoteStatusEvergreenSessions = 6\nnoteStatusEvergreenSpanDays = 200\nnoteStatusStartedWords = 100\n";
        assert_eq!(
            thresholds_from_toml(cfg),
            Thresholds { evergreen_words: 700, evergreen_sessions: 6, evergreen_span_days: 200, started_words: 100 }
        );
        assert_eq!(thresholds_from_toml("[params]\nnoteStatusStartedWords = 50\n"),
            Thresholds { started_words: 50, ..Thresholds::default() });
        assert_eq!(thresholds_from_toml("not even = [toml"), Thresholds::default());
    }
}
