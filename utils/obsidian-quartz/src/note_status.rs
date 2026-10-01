//! Per-note maturity level ("started" / "growing" / "evergreen") for the
//! public second brain, derived from git edit history and word count, or
//! overridden by a `#garden/<level>` tag in the vault.
//!
//! Spec: docs/superpowers/specs/2026-10-01-note-status-design.md
//!
//! Runs as `obsidian-quartz note-status` in `make prepare`, right after
//! `utils/recent_updates.py` (whose data/recent_updates.json it consumes).
//! Writes data/note_status.json (read by Hugo) and
//! note_status_report.md at the repo root (read by the author, gitignored;
//! not under data/ because Hugo tries to parse every file there).

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
    /// started when fewer words than this AND every edit fell within
    /// `started_span_days` of the first (a same-week fix commit is not tending).
    pub started_words: usize,
    pub started_span_days: i64,
    /// started whatever the history when fewer words than this.
    pub tiny_words: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            evergreen_words: 600,
            evergreen_sessions: 5,
            evergreen_span_days: 180,
            started_words: 120,
            started_span_days: 30,
            tiny_words: 50,
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
/// (started: tiny, or short and never tended past its first month — see the
/// 2026-10-01 calibration ruling in the plan ledger / CHANGELOG.)
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
    if f.words < t.tiny_words || (f.words < t.started_words && f.span_days < t.started_span_days) {
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
    let s = unquote(s).get(..10)?; // None when shorter, or when byte 10 splits a multibyte char
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
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

/// Thresholds from the site's config.toml ([params] noteStatus*). A missing
/// key keeps its default. An unparsable file or a key that is not a
/// non-negative integer also keeps the default, but is reported in the
/// returned warnings so a tuning typo never silently does nothing.
pub fn parse_thresholds(text: &str) -> (Thresholds, Vec<String>) {
    let mut t = Thresholds::default();
    let mut warnings = Vec::new();
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => doc,
        Err(e) => {
            warnings.push(format!("config.toml did not parse ({}); using default thresholds", e.message()));
            return (t, warnings);
        }
    };
    let Some(params) = doc.get("params").and_then(|p| p.as_table()) else { return (t, warnings) };
    let mut int = |key: &str| -> Option<i64> {
        let v = params.get(key)?;
        match v.as_integer() {
            Some(n) if n >= 0 => Some(n),
            _ => {
                warnings.push(format!(
                    "config.toml [params] {} = {} is not a non-negative integer; using the default",
                    key, v
                ));
                None
            }
        }
    };
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
    if let Some(n) = int("noteStatusStartedSpanDays") {
        t.started_span_days = n;
    }
    if let Some(n) = int("noteStatusTinyWords") {
        t.tiny_words = n as usize;
    }
    (t, warnings)
}

/// `parse_thresholds` without the warnings.
pub fn thresholds_from_toml(text: &str) -> Thresholds {
    parse_thresholds(text).0
}

/// Pull `garden/<level>` out of a note's tag list. Returns the remaining tags
/// (so Hugo never gets a `garden/...` taxonomy term), the level if one is
/// valid, and every `garden/*` tag that is not a level (dropped; the caller
/// warns with the note's name, which this function does not know).
pub fn split_garden_tag(tags: Vec<String>) -> (Vec<String>, Option<Level>, Vec<String>) {
    let mut rest = Vec::with_capacity(tags.len());
    let mut level = None;
    let mut rejected = Vec::new();
    for tag in tags {
        let lower = tag.to_ascii_lowercase();
        match lower.strip_prefix("garden/") {
            Some(value) => match Level::parse(value) {
                Some(l) => {
                    if level.is_none() {
                        level = Some(l);
                    }
                }
                None => rejected.push(tag),
            },
            None => rest.push(tag),
        }
    }
    (rest, level, rejected)
}

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::enrich_with_blog::{load_json, save_json_pretty};

const CONTENT_DIR: &str = "content";
const CONFIG_TOML: &str = "config.toml";
const RECENT_UPDATES: &str = "data/recent_updates.json";
const OUT_JSON: &str = "data/note_status.json";
const OUT_REPORT: &str = "note_status_report.md"; // repo root: Hugo loads every file under data/ and fails on .md

#[derive(Clone, Debug)]
pub struct Entry {
    /// Filename stem, exactly Hugo's `.File.BaseFileName` ("don't stop").
    pub stem: String,
    pub level: Level,
    pub source: Source,
    pub facts: NoteFacts,
    pub reason: String,
}

/// One Entry per `content/*.md` (top level only, `_index.md` skipped), sorted
/// by stem. `recent` is the parsed data/recent_updates.json object. Errors
/// when the directory is unreadable or holds no notes, so a broken checkout
/// can never ship an empty data/note_status.json (= no level word anywhere);
/// a single unreadable note is reported and skipped.
pub fn build_entries(content_dir: &Path, recent: &serde_json::Value, t: &Thresholds) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let dir = fs::read_dir(content_dir)
        .map_err(|e| format!("note-status: cannot read {} ({})", content_dir.display(), e))?;
    for entry in dir.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else {
            eprintln!("note-status: skipping {} (file name is not UTF-8)", path.display());
            continue;
        };
        if stem == "_index" {
            continue;
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("note-status: skipping {} ({})", path.display(), e);
                continue;
            }
        };
        let fm = frontmatter_fields(&text);
        let (sessions, mut span_days) = sessions_facts(recent.get(&stem));
        if sessions == 0 {
            if let (Some(c), Some(l)) = (fm.createddate, fm.lastmod) {
                span_days = (l - c).num_days().max(0);
            }
        }
        let facts = NoteFacts { words: word_count(&text), sessions, span_days, manual: fm.manual };
        let (level, source) = classify(&facts, t);
        let reason = reason(&facts, source);
        entries.push(Entry { stem, level, source, facts, reason });
    }
    if entries.is_empty() {
        return Err(format!("note-status: no notes found under {}", content_dir.display()));
    }
    entries.sort_by(|a, b| a.stem.cmp(&b.stem));
    Ok(entries)
}

pub fn entries_to_json(entries: &[Entry]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for e in entries {
        map.insert(
            e.stem.clone(),
            serde_json::json!({
                "status": e.level.as_str(),
                "source": e.source.as_str(),
                "words": e.facts.words,
                "sessions": e.facts.sessions,
                "span_days": e.facts.span_days,
                "reason": e.reason,
            }),
        );
    }
    serde_json::Value::Object(map)
}

/// The author's Markdown report: summary, manual-vs-git disagreements (biggest
/// first), then every note under its level, longest first.
pub fn render_report(entries: &[Entry], t: &Thresholds, today: NaiveDate) -> String {
    let levels = [Level::Evergreen, Level::Growing, Level::Started];
    let mut out = format!("# Note status report · {}\n\n", today.format("%Y-%m-%d"));
    out.push_str(&format!(
        "Thresholds: evergreen = {} words, {} sessions, {} days · started = < {} words with all edits within {} days, or < {} words\n\n",
        t.evergreen_words, t.evergreen_sessions, t.evergreen_span_days, t.started_words, t.started_span_days, t.tiny_words
    ));
    out.push_str("| level | git | manual | total |\n|---|---|---|---|\n");
    for level in levels {
        let git = entries.iter().filter(|e| e.level == level && e.source == Source::Git).count();
        let manual = entries.iter().filter(|e| e.level == level && e.source == Source::Manual).count();
        out.push_str(&format!("| {} | {} | {} | {} |\n", level.as_str(), git, manual, git + manual));
    }

    // Disagreements: manual notes re-run through the git rules.
    let mut dis: Vec<(&Entry, Level, u8)> = entries
        .iter()
        .filter(|e| e.source == Source::Manual)
        .filter_map(|e| {
            let git_facts = NoteFacts { manual: None, ..e.facts.clone() };
            let (git_level, _) = classify(&git_facts, t);
            (git_level != e.level).then(|| {
                let gap = (git_level.rank() as i8 - e.level.rank() as i8).unsigned_abs();
                (e, git_level, gap)
            })
        })
        .collect();
    dis.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.stem.cmp(&b.0.stem)));
    out.push_str(&format!("\n## Manual tag disagrees with heuristic ({})\n\n", dis.len()));
    for (e, git_level, _) in &dis {
        let git_facts = NoteFacts { manual: None, ..e.facts.clone() };
        out.push_str(&format!(
            "- **{}** — manual `{}`, heuristic `{}` — {}\n",
            e.stem,
            e.level.as_str(),
            git_level.as_str(),
            reason(&git_facts, Source::Git)
        ));
    }

    for level in levels {
        let mut rows: Vec<&Entry> = entries.iter().filter(|e| e.level == level).collect();
        rows.sort_by(|a, b| b.facts.words.cmp(&a.facts.words).then(a.stem.cmp(&b.stem)));
        out.push_str(&format!("\n## {} ({})\n\n", level.as_str(), rows.len()));
        for e in rows {
            out.push_str(&format!("- {} — {}\n", e.stem, e.reason));
        }
    }
    out
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let thresholds = match fs::read_to_string(CONFIG_TOML) {
        Ok(text) => {
            let (t, warnings) = parse_thresholds(&text);
            for w in warnings {
                eprintln!("note-status: {}", w);
            }
            t
        }
        Err(e) => {
            eprintln!("note-status: cannot read {} ({}); using default thresholds", CONFIG_TOML, e);
            Thresholds::default()
        }
    };
    let recent = match load_json(RECENT_UPDATES) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "note-status: {} not readable ({}), classifying on word counts only",
                RECENT_UPDATES, e
            );
            serde_json::json!({})
        }
    };
    let entries = build_entries(Path::new(CONTENT_DIR), &recent, &thresholds)?;
    save_json_pretty(OUT_JSON, &entries_to_json(&entries))?;
    let today = chrono::Utc::now().naive_utc().date();
    fs::write(OUT_REPORT, render_report(&entries, &thresholds, today))?;
    let count = |l: Level| entries.iter().filter(|e| e.level == l).count();
    println!(
        "note-status: {} notes -> {} evergreen, {} growing, {} started ({} manual) -> {} + {}",
        entries.len(),
        count(Level::Evergreen),
        count(Level::Growing),
        count(Level::Started),
        entries.iter().filter(|e| e.source == Source::Manual).count(),
        OUT_JSON,
        OUT_REPORT
    );
    Ok(())
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
    fn started_is_short_and_untended() {
        let t = Thresholds::default();
        assert_eq!(classify(&facts(95, 1, 0), &t), (Level::Started, Source::Git));
        assert_eq!(classify(&facts(95, 0, 0), &t), (Level::Started, Source::Git));
        // a same-week fix commit does not make a 3-word note "growing"
        assert_eq!(classify(&facts(3, 2, 1), &t), (Level::Started, Source::Git));
        assert_eq!(classify(&facts(95, 2, 29), &t), (Level::Started, Source::Git));
        // tended for a month, or enough words, makes it growing
        assert_eq!(classify(&facts(95, 2, 30), &t), (Level::Growing, Source::Git));
        assert_eq!(classify(&facts(120, 1, 0), &t), (Level::Growing, Source::Git));
        // a tiny note stays started however often it was touched
        assert_eq!(classify(&facts(9, 5, 700), &t), (Level::Started, Source::Git));
        assert_eq!(classify(&facts(49, 5, 700), &t), (Level::Started, Source::Git));
        assert_eq!(classify(&facts(50, 5, 700), &t), (Level::Growing, Source::Git));
    }

    #[test]
    fn thresholds_are_not_hardcoded() {
        let t = Thresholds { evergreen_words: 10, evergreen_sessions: 1, evergreen_span_days: 0, started_words: 1, started_span_days: 0, tiny_words: 0 };
        assert_eq!(classify(&facts(10, 1, 0), &t), (Level::Evergreen, Source::Git));
        assert_eq!(classify(&facts(5, 1, 0), &t), (Level::Growing, Source::Git));
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
        let cfg = "baseURL = \"x\"\n[params]\nauthor = \"S\"\nnoteStatusEvergreenWords = 700\nnoteStatusEvergreenSessions = 6\nnoteStatusEvergreenSpanDays = 200\nnoteStatusStartedWords = 100\nnoteStatusStartedSpanDays = 14\nnoteStatusTinyWords = 40\n";
        assert_eq!(
            thresholds_from_toml(cfg),
            Thresholds { evergreen_words: 700, evergreen_sessions: 6, evergreen_span_days: 200, started_words: 100, started_span_days: 14, tiny_words: 40 }
        );
        assert_eq!(thresholds_from_toml("[params]\nnoteStatusStartedWords = 50\n"),
            Thresholds { started_words: 50, ..Thresholds::default() });
        assert_eq!(thresholds_from_toml("not even = [toml"), Thresholds::default());
    }

    fn temp_content(files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("note-status-test-{}-{}", std::process::id(), files.len()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_img")).unwrap();
        for (name, body) in files {
            std::fs::write(dir.join(name), body).unwrap();
        }
        dir
    }

    #[test]
    fn build_entries_keys_by_stem_skips_index_and_joins_sessions() {
        let dir = temp_content(&[
            ("_index.md", "---\ntitle: x\n---\nhome"),
            ("don't stop.md", "---\ncreateddate: 2026-10-01\n---\nshort note"),
            ("big essay.md", &format!("---\ncreateddate: 2024-08-31\nlastmod: 2026-09-23 12:16:49\n---\n{}", "word ".repeat(700))),
            ("tagged.md", "---\nstatus: started\nstatus_source: manual\n---\nmanual override"),
            ("_img/pic.webp", "not a note"),
        ]);
        let recent = serde_json::json!({
            "big essay": {"status": "updated", "words": 10, "sessions": [
                {"iso": "2026-09-23", "date": "Sep 23", "rel": "", "added": 1, "removed": 0},
                {"iso": "2026-06-01", "date": "Jun 1", "rel": "", "added": 1, "removed": 0},
                {"iso": "2025-12-01", "date": "Dec 1", "rel": "", "added": 1, "removed": 0},
                {"iso": "2025-03-01", "date": "Mar 1", "rel": "", "added": 1, "removed": 0},
                {"iso": "2024-08-31", "date": "Aug 31", "rel": "", "kind": "published", "words": 10}
            ]},
            "don't stop": {"status": "new", "words": 2, "sessions": [
                {"iso": "2026-10-01", "date": "Oct 1", "rel": "today", "kind": "published", "words": 2}
            ]}
        });
        let entries = build_entries(&dir, &recent, &Thresholds::default()).unwrap();
        let stems: Vec<&str> = entries.iter().map(|e| e.stem.as_str()).collect();
        assert_eq!(stems, vec!["big essay", "don't stop", "tagged"]);

        let big = &entries[0];
        assert_eq!((big.level, big.source), (Level::Evergreen, Source::Git));
        assert_eq!(big.facts.sessions, 5);
        assert_eq!(big.facts.span_days, 753);
        assert_eq!(big.reason, "Estimated from edit history · 5 sessions over 2 years · 700 words");

        let short = &entries[1];
        assert_eq!((short.level, short.source), (Level::Started, Source::Git));
        assert_eq!(short.facts, NoteFacts { words: 2, sessions: 1, span_days: 0, manual: None });

        let tagged = &entries[2];
        assert_eq!((tagged.level, tagged.source), (Level::Started, Source::Manual));
        assert_eq!(tagged.reason, "Set by author");
        assert_eq!(tagged.facts.sessions, 0);

        let json = entries_to_json(&entries);
        assert_eq!(json["big essay"]["status"], "evergreen");
        assert_eq!(json["big essay"]["source"], "git");
        assert_eq!(json["big essay"]["words"], 700);
        assert_eq!(json["big essay"]["sessions"], 5);
        assert_eq!(json["big essay"]["span_days"], 753);
        assert_eq!(json["don't stop"]["reason"], "Estimated from edit history · 1 session · 2 words");
        assert!(json.get("_index").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_entries_span_falls_back_to_frontmatter_dates_without_sessions() {
        let dir = temp_content(&[
            ("old.md", "---\ncreateddate: 2023-01-01\nlastmod: 2023-07-01 10:00:00\n---\nold untouched note body"),
        ]);
        let entries = build_entries(&dir, &serde_json::json!({}), &Thresholds::default()).unwrap();
        assert_eq!(entries[0].facts, NoteFacts { words: 4, sessions: 0, span_days: 181, manual: None });
        assert_eq!(entries[0].level, Level::Started); // 4 words, 0 sessions
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_entries_fails_loudly_on_missing_dir_or_no_notes() {
        let missing = std::env::temp_dir().join("note-status-test-does-not-exist");
        let err = build_entries(&missing, &serde_json::json!({}), &Thresholds::default()).unwrap_err();
        assert!(err.contains("note-status-test-does-not-exist"), "names the dir: {err}");

        let empty = temp_content(&[("_index.md", "---\ntitle: x\n---\nhome"), ("_img/only.webp", "")]);
        let err = build_entries(&empty, &serde_json::json!({}), &Thresholds::default()).unwrap_err();
        assert!(err.contains("no notes"), "{err}");
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn date_prefix_never_panics_on_multibyte_input() {
        // a smart quote pasted into the vault lands verbatim in the frontmatter;
        // byte 10 is inside the 3-byte quote here, so a byte slice would panic
        let f = frontmatter_fields("---\ncreateddate: 2026-10-0\u{2019}1\nlastmod: \u{2019}2026-10-01\n---\n");
        assert_eq!(f.createddate, None);
        assert_eq!(f.lastmod, None);
    }

    #[test]
    fn parse_thresholds_warns_on_bad_values_and_unparsable_config() {
        let (t, warnings) = parse_thresholds("[params]\nnoteStatusStartedWords = \"100\"\nnoteStatusTinyWords = 100.0\nnoteStatusEvergreenSessions = -1\nnoteStatusEvergreenWords = 700\n");
        assert_eq!(t, Thresholds { evergreen_words: 700, ..Thresholds::default() });
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("noteStatusStartedWords")));
        assert!(warnings.iter().any(|w| w.contains("noteStatusTinyWords")));
        assert!(warnings.iter().any(|w| w.contains("noteStatusEvergreenSessions")));

        let (t, warnings) = parse_thresholds("not even = [toml");
        assert_eq!(t, Thresholds::default());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("config.toml"), "{warnings:?}");

        assert!(parse_thresholds("[params]\nnoteStatusTinyWords = 40\n").1.is_empty());
    }

    #[test]
    fn render_report_lists_disagreements_first_then_each_level() {
        let t = Thresholds::default();
        let mk = |stem: &str, words, sessions, span, manual: Option<Level>| {
            let facts = NoteFacts { words, sessions, span_days: span, manual };
            let (level, source) = classify(&facts, &t);
            let reason = reason(&facts, source);
            Entry { stem: stem.to_string(), level, source, facts, reason }
        };
        let entries = vec![
            mk("alpha", 3000, 20, 700, Some(Level::Started)), // manual started, git says evergreen
            mk("beta", 50, 1, 0, Some(Level::Growing)),       // manual growing, git says started
            mk("gamma", 3000, 20, 700, None),
            mk("delta", 50, 1, 0, None),
            mk("epsilon", 300, 2, 10, None),
            mk("zeta", 50, 1, 0, Some(Level::Started)),        // manual agrees
        ];
        let report = render_report(&entries, &t, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert!(report.starts_with("# Note status report · 2026-10-01\n"));
        assert!(report.contains("| evergreen | 1 | 0 | 1 |"), "summary row for evergreen:\n{report}");
        assert!(report.contains("| growing | 1 | 1 | 2 |"));
        assert!(report.contains("| started | 1 | 2 | 3 |"));
        assert!(report.contains("Thresholds: evergreen = 600 words, 5 sessions, 180 days · started = < 120 words with all edits within 30 days, or < 50 words"));

        let dis = report.find("## Manual tag disagrees with heuristic (2)").expect("disagreement section");
        let alpha = report[dis..].find("- **alpha**").unwrap();
        let beta = report[dis..].find("- **beta**").unwrap();
        assert!(alpha < beta, "bigger disagreement first");
        assert!(report.contains("- **alpha** — manual `started`, heuristic `evergreen` — Estimated from edit history · 20 sessions over 1 year · 3,000 words"));
        assert!(!report[dis..].contains("- **zeta**"), "agreeing manual tags are not disagreements");

        let ever = report.find("## evergreen (1)").unwrap();
        let grow = report.find("## growing (2)").unwrap();
        let start = report.find("## started (3)").unwrap();
        assert!(dis < ever && ever < grow && grow < start);
        assert!(report[start..].contains("- zeta — Set by author"));
        assert!(report[ever..grow].contains("- gamma — Estimated from edit history · 20 sessions over 1 year · 3,000 words"));
    }

    #[test]
    fn split_garden_tag_extracts_level_case_insensitively_and_drops_unknown() {
        let tags = vec!["cooking".to_string(), "garden/Growing".to_string(), "til".to_string()];
        let (rest, level, rejected) = split_garden_tag(tags);
        assert_eq!(rest, vec!["cooking", "til"]);
        assert_eq!(level, Some(Level::Growing));
        assert!(rejected.is_empty());

        let (rest, level, _) = split_garden_tag(vec!["Garden/EVERGREEN".to_string()]);
        assert!(rest.is_empty());
        assert_eq!(level, Some(Level::Evergreen));

        // unknown level: dropped from tags, no level, handed back so the caller
        // can warn with the note's name
        let (rest, level, rejected) = split_garden_tag(vec!["garden/tree".to_string(), "x".to_string()]);
        assert_eq!(rest, vec!["x"]);
        assert_eq!(level, None);
        assert_eq!(rejected, vec!["garden/tree"]);

        // no garden tag at all
        let (rest, level, rejected) = split_garden_tag(vec!["a".to_string()]);
        assert_eq!(rest, vec!["a"]);
        assert_eq!(level, None);
        assert!(rejected.is_empty());
    }
}
