# Note Status (started → growing → evergreen) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every public note shows a lowercase level word (`started`, `growing`, `evergreen`) at the end of its meta line; hovering it explains the level and lists the recent edit sessions, with the level derived from git history or a `#garden/*` tag in the vault.

**Architecture:** A new Rust subcommand `obsidian-quartz note-status` runs in `make prepare` right after `utils/recent_updates.py`, joins that script's `data/recent_updates.json` with each note's word count and frontmatter, and writes `data/note_status.json` (for Hugo) plus a gitignored Markdown report (for the author). The existing `.note-changes` trigger in `single.html` becomes the level word and its popover gains a status section above the sessions list. `file_utils.rs` maps a `#garden/<level>` tag on the vault note's `Tags:` line to `status:` / `status_source: manual` frontmatter.

**Tech Stack:** Rust 2021 (`utils/obsidian-quartz`: serde_json, serde_yaml, chrono, regex, glob; add `toml = "0.9"`), Hugo templates + SCSS, one-line vanilla JS change, Python 3 stdlib for the one-off Jev calibration script.

**Spec:** `docs/superpowers/specs/2026-10-01-note-status-design.md`

## Global Constraints

- Level words are exactly `started`, `growing`, `evergreen`; lowercase in data, pills, lists and tags. Title case only in the popover sentence ("Growing.").
- Reader-facing sentences, verbatim: started = "Quick capture, barely worked on. May change or vanish." growing = "Worked on, still rough. Expect bullets, gaps and views that move." evergreen = "Own words, tended over time. Still grows, never finished."
- Reason strings, verbatim shapes: manual = `Set by author`; git = `Estimated from edit history · 23 sessions over 2 years · 3,120 words` (one session: `1 session`; zero: `no edits recorded`; no "over …" when sessions < 2).
- Thresholds live in `config.toml` `[params]`: `noteStatusEvergreenWords = 600`, `noteStatusEvergreenSessions = 5`, `noteStatusEvergreenSpanDays = 180`, `noteStatusStartedWords = 120`.
- Rules in order: manual tag → `words ≥ 600 && sessions ≥ 5 && span_days ≥ 180` → evergreen → `words < 120 && sessions ≤ 1` → started → else growing.
- Vault override tag: `#garden/started|growing|evergreen` on the note's `Tags:` line, case-insensitive. Unknown `garden/*` values are dropped with a warning. Old emoji tags stay stripped and never map to a level.
- Nothing is written to the vault (`$secondbrain`). Ever.
- No new HTTP request, script file or stylesheet for visitors. The popover is baked HTML; the only JS edit is one selector in `assets/js/popover-v2.js`.
- `content/` is a git submodule with its own repo. All commits in this plan go to the main repo; never `git add` inside `content/`.
- `data/note_status.json` and `note_status_report.md` are gitignored build outputs, like `data/recent_updates.json`.
- Keep SCSS chunks ASCII-only at the top of a new block (Sass BOM trap, see comments in `custom.scss`).

## Review Focus

1. A vault note whose frontmatter already carries `status: foo` (typed by hand, no `#garden/*` tag) must not be treated as a manual override; only `status_source: manual` counts. Pinned in Task 2 (`frontmatter_fields` test).
2. `#garden/Growing` or `#Garden/EVERGREEN` on the `Tags:` line must map to `growing` / `evergreen`; `#garden/tree` must warn and vanish from the published tags. Pinned in Task 4.
3. A `recent_updates.json` entry whose rows mix `{"kind":"published","words":…}` and `{"added":…,"removed":…}` must count every row as a session and compute the span from newest to oldest `iso`. Pinned in Task 2 (`sessions_facts` test).
4. File stems with apostrophes or dashes (`don't stop.md`, `dash – note.md`) must be keyed exactly as Hugo's `.File.BaseFileName` sees them; `_index.md` must be skipped. Pinned in Task 3 (`run` integration test on a temp dir).
5. A note with a status entry but no `recent_updates.json` entry must render the trigger and the status section with no "Recent changes" heading; a note with no status entry renders no trigger at all. Pinned in Task 5 (Hugo render test).

---

### Task 0: Baseline Lighthouse numbers and a branch

**Files:**
- Read: `CHANGELOG.md` (top entry only, for the format)

- [ ] **Step 1: Create the branch in place (no worktree; `content/` is a submodule and `data/recent_updates.json` must exist)**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git status --short | head        # expect nothing outside content/ (submodule) 
git switch -c note-status
```

- [ ] **Step 2: Make sure the build inputs exist, then record the baseline**

```bash
test -f data/recent_updates.json || python utils/recent_updates.py
make lighthouse P=will-ai-replace-humans/ 2>&1 | tail -6
```

Copy the two printed lines (`mobile …` / `desktop …` with the category scores and the metric line under each) into the scratchpad file `lighthouse-before.txt`. They are compared in Task 7.

---

### Task 1: Pure classification core in `note_status.rs`

**Files:**
- Create: `utils/obsidian-quartz/src/note_status.rs`
- Modify: `utils/obsidian-quartz/src/main.rs:17-18` (add `mod note_status;`)

**Interfaces:**
- Produces:
  - `pub enum Level { Started, Growing, Evergreen }` with `pub fn as_str(&self) -> &'static str`, `pub fn parse(s: &str) -> Option<Level>` (case-insensitive), `pub fn rank(&self) -> u8` (0, 1, 2).
  - `pub enum Source { Git, Manual }` with `as_str` → `"git"` / `"manual"`.
  - `pub struct Thresholds { pub evergreen_words: usize, pub evergreen_sessions: usize, pub evergreen_span_days: i64, pub started_words: usize }` and `impl Default` with 600 / 5 / 180 / 120.
  - `pub struct NoteFacts { pub words: usize, pub sessions: usize, pub span_days: i64, pub manual: Option<Level> }`
  - `pub fn classify(f: &NoteFacts, t: &Thresholds) -> (Level, Source)`
  - `pub fn reason(f: &NoteFacts, source: Source) -> String`
  - `pub fn word_count(text: &str) -> usize` (frontmatter stripped, whitespace split, same as `recent_updates.py::_word_count`)
  - `pub fn with_commas(n: usize) -> String`, `pub fn humanize_span(days: i64) -> String`

- [ ] **Step 1: Create the module with the tests first**

Write `utils/obsidian-quartz/src/note_status.rs`:

```rust
//! Per-note maturity level ("started" / "growing" / "evergreen") for the
//! public second brain, derived from git edit history and word count, or
//! overridden by a `#garden/<level>` tag in the vault.
//!
//! Spec: docs/superpowers/specs/2026-10-01-note-status-design.md
//!
//! Runs as `obsidian-quartz note-status` in `make prepare`, right after
//! `utils/recent_updates.py` (whose data/recent_updates.json it consumes).
//! Writes data/note_status.json (read by Hugo) and
//! note_status_report.md (read by the author, gitignored).

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
    let body = strip_frontmatter(text);
    body.split_whitespace().count()
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
}
```

- [ ] **Step 2: Register the module and run the tests to see them fail to compile, then pass**

In `utils/obsidian-quartz/src/main.rs`, after `mod merge_search_index;` add:

```rust
mod note_status;
```

Run:

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test note_status
```

Expected: 9 tests in `note_status::tests` PASS (the module is self-contained, so the test file and implementation land together; the "fail first" moment is the compile error you get if you register the module before writing the file, which is fine to skip here). Warnings about unused functions are expected until Task 3 wires them up.

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add utils/obsidian-quartz/src/note_status.rs utils/obsidian-quartz/src/main.rs
git commit -m "note-status: classification core (levels, thresholds, reason strings)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: Parsing inputs: frontmatter fields, sessions, thresholds from config.toml

**Files:**
- Modify: `utils/obsidian-quartz/src/note_status.rs` (append)
- Modify: `utils/obsidian-quartz/Cargo.toml:6-13` (add `toml = "0.9"`)

**Interfaces:**
- Consumes: `Level`, `Thresholds` from Task 1.
- Produces:
  - `pub struct FrontmatterFields { pub manual: Option<Level>, pub createddate: Option<chrono::NaiveDate>, pub lastmod: Option<chrono::NaiveDate> }`
  - `pub fn frontmatter_fields(text: &str) -> FrontmatterFields`
  - `pub fn sessions_facts(entry: Option<&serde_json::Value>) -> (usize, i64)` → (session count, span days newest−oldest)
  - `pub fn thresholds_from_toml(text: &str) -> Thresholds` (missing keys fall back to `Default`)

- [ ] **Step 1: Add the dependency**

In `utils/obsidian-quartz/Cargo.toml` under `[dependencies]` add:

```toml
toml = "0.9"
```

Run `cargo build` once from `utils/obsidian-quartz` to resolve it (the crate is in the local registry cache; if resolution needs the network and fails, use `toml = "0.9.8"`, which is cached).

- [ ] **Step 2: Append the tests**

Inside the existing `mod tests` block in `note_status.rs` add:

```rust
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
```

- [ ] **Step 3: Run to verify they fail**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test note_status 2>&1 | head -20
```

Expected: compile errors `cannot find function frontmatter_fields`, `sessions_facts`, `thresholds_from_toml`, `FrontmatterFields`.

- [ ] **Step 4: Implement, above `#[cfg(test)]`**

```rust
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
    s.strip_prefix('\'').and_then(|x| x.strip_suffix('\''))
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
    if let Some(n) = int("noteStatusEvergreenWords") { t.evergreen_words = n as usize; }
    if let Some(n) = int("noteStatusEvergreenSessions") { t.evergreen_sessions = n as usize; }
    if let Some(n) = int("noteStatusEvergreenSpanDays") { t.evergreen_span_days = n; }
    if let Some(n) = int("noteStatusStartedWords") { t.started_words = n as usize; }
    t
}
```

- [ ] **Step 5: Run to verify they pass**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test note_status
```

Expected: 12 tests PASS.

- [ ] **Step 6: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add utils/obsidian-quartz/src/note_status.rs utils/obsidian-quartz/Cargo.toml utils/obsidian-quartz/Cargo.lock
git commit -m "note-status: parse frontmatter, sessions and config.toml thresholds

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: `obsidian-quartz note-status` subcommand, report, Makefile wiring

**Files:**
- Modify: `utils/obsidian-quartz/src/note_status.rs` (append `run`, `build_entries`, `render_report`)
- Modify: `utils/obsidian-quartz/src/main.rs:37-45` (dispatch)
- Modify: `Makefile` (`prepare` target, after the `recent_updates.py` line)
- Modify: `config.toml` `[params]` (four thresholds, next to `recentUpdates*`)
- Modify: `.gitignore` (two lines after `data/recent_updates.json`)

**Interfaces:**
- Consumes: everything from Tasks 1–2; `crate::enrich_with_blog::{load_json, save_json_pretty}`.
- Produces:
  - `pub struct Entry { pub stem: String, pub level: Level, pub source: Source, pub facts: NoteFacts, pub reason: String }`
  - `pub fn build_entries(content_dir: &Path, recent: &serde_json::Value, t: &Thresholds) -> Vec<Entry>` (sorted by stem)
  - `pub fn entries_to_json(entries: &[Entry]) -> serde_json::Value`
  - `pub fn render_report(entries: &[Entry], t: &Thresholds, today: NaiveDate) -> String`
  - `pub fn run() -> Result<(), Box<dyn Error>>`
  - `data/note_status.json`: `{ "<stem>": { "status", "source", "words", "sessions", "span_days", "reason" } }`

- [ ] **Step 1: Append the tests (temp-dir integration for `build_entries`, string test for `render_report`)**

Inside `mod tests`:

```rust
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
        let entries = build_entries(&dir, &recent, &Thresholds::default());
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
        // no recent_updates entry: span falls back to lastmod - createddate (both missing -> 0)
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
        let entries = build_entries(&dir, &serde_json::json!({}), &Thresholds::default());
        assert_eq!(entries[0].facts, NoteFacts { words: 4, sessions: 0, span_days: 181, manual: None });
        assert_eq!(entries[0].level, Level::Started); // 4 words, 0 sessions
        let _ = std::fs::remove_dir_all(&dir);
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
        assert!(report.contains("Thresholds: evergreen = 600 words, 5 sessions, 180 days · started = < 120 words, <= 1 session"));

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
```

- [ ] **Step 2: Run to verify they fail**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test note_status 2>&1 | grep -E "^error" | head
```

Expected: `cannot find function build_entries`, `entries_to_json`, `render_report`, `struct Entry`.

- [ ] **Step 3: Implement, above `#[cfg(test)]`**

```rust
use std::error::Error;
use std::fs;
use std::path::Path;

use crate::enrich_with_blog::{load_json, save_json_pretty};

const CONTENT_DIR: &str = "content";
const CONFIG_TOML: &str = "config.toml";
const RECENT_UPDATES: &str = "data/recent_updates.json";
const OUT_JSON: &str = "data/note_status.json";
const OUT_REPORT: &str = "note_status_report.md";

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
/// by stem. `recent` is the parsed data/recent_updates.json object.
pub fn build_entries(content_dir: &Path, recent: &serde_json::Value, t: &Thresholds) -> Vec<Entry> {
    let mut entries = Vec::new();
    let Ok(dir) = fs::read_dir(content_dir) else { return entries };
    for entry in dir.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
        if stem == "_index" {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
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
    entries.sort_by(|a, b| a.stem.cmp(&b.stem));
    entries
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
        "Thresholds: evergreen = {} words, {} sessions, {} days · started = < {} words, <= 1 session\n\n",
        t.evergreen_words, t.evergreen_sessions, t.evergreen_span_days, t.started_words
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
            e.stem, e.level.as_str(), git_level.as_str(), reason(&git_facts, Source::Git)
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
        Ok(text) => thresholds_from_toml(&text),
        Err(_) => Thresholds::default(),
    };
    let recent = match load_json(RECENT_UPDATES) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("note-status: {} not readable ({}), classifying on word counts only", RECENT_UPDATES, e);
            serde_json::json!({})
        }
    };
    let entries = build_entries(Path::new(CONTENT_DIR), &recent, &thresholds);
    save_json_pretty(OUT_JSON, &entries_to_json(&entries))?;
    let today = chrono::Utc::now().naive_utc().date(); // works on every chrono 0.4.x
    fs::write(OUT_REPORT, render_report(&entries, &thresholds, today))?;
    let count = |l: Level| entries.iter().filter(|e| e.level == l).count();
    println!(
        "note-status: {} notes -> {} evergreen, {} growing, {} started ({} manual) -> {} + {}",
        entries.len(), count(Level::Evergreen), count(Level::Growing), count(Level::Started),
        entries.iter().filter(|e| e.source == Source::Manual).count(), OUT_JSON, OUT_REPORT
    );
    Ok(())
}
```

Then in `main.rs`, in the dispatch chain after the `merge-search-index` arm:

```rust
    } else if args.len() > 1 && args[1] == "note-status" {
        note_status::run()?;
```

- [ ] **Step 4: Run tests**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test note_status
```

Expected: 15 tests PASS, no warnings about unused items in `note_status.rs`.

- [ ] **Step 5: Wire config, Makefile, gitignore**

`config.toml`, inside `[params]` directly after the `recentUpdatesNewGraceDays` line:

```toml
  # note status (read by `obsidian-quartz note-status`, build-time only; spec docs/superpowers/specs/2026-10-01-note-status-design.md)
  noteStatusEvergreenWords    = 600   # evergreen needs all three: words, sessions, span
  noteStatusEvergreenSessions = 5
  noteStatusEvergreenSpanDays = 180
  noteStatusStartedWords      = 120   # started when fewer words AND <= 1 editing session
```

`Makefile`, in the `prepare` target, directly after the `python utils/recent_updates.py` line:

```make
	obsidian-quartz note-status #per-note level started/growing/evergreen -> data/note_status.json (+ note_status_report.md for the author)
```

`.gitignore`, directly after `data/recent_updates.json`:

```
data/note_status.json
note_status_report.md
```

- [ ] **Step 6: Build, install, run end to end**

First see how the binary on PATH is provided (`which obsidian-quartz; readlink -f "$(which obsidian-quartz)"`). If it resolves into `utils/obsidian-quartz/target/release/`, `cargo build --release` alone refreshes it and the `cargo install` below is unnecessary; if it lives under `~/.cargo/bin`, run both as written.

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo build --release && cargo install --path . --force 2>&1 | tail -2
cd /home/sspaeti/git/sspaeti.com/second-brain-public && obsidian-quartz note-status && head -c 600 data/note_status.json && sed -n 1,25p note_status_report.md
```

Expected: a summary line like `note-status: 1533 notes -> N evergreen, N growing, N started (0 manual)`, JSON keyed by stems, a report whose disagreement section is `(0)`. Check `will ai replace humans` is `evergreen` and `format usb drive on linux` is `started` (95 words, 1 session) in the JSON:

```bash
python3 -c "import json;d=json.load(open('data/note_status.json'));print(d['will ai replace humans']);print(d['format usb drive on linux'])"
```

Note for the author in the final summary: `format usb drive on linux` lands on `started`; the calibration run in Task 8 is where thresholds get tuned.

- [ ] **Step 7: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add utils/obsidian-quartz/src/note_status.rs utils/obsidian-quartz/src/main.rs config.toml Makefile .gitignore
git commit -m "note-status: subcommand writing data/note_status.json and the author report

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: `#garden/<level>` tag → `status` frontmatter in `file_utils.rs`

**Files:**
- Modify: `utils/obsidian-quartz/src/file_utils.rs:331-341` (Tags line), `:556-565` (new-frontmatter branch), `:664-700` (merge branch, next to the tags handling)
- Modify: `utils/obsidian-quartz/src/note_status.rs` (add `split_garden_tag`)

**Interfaces:**
- Produces: `pub fn split_garden_tag(tags: Vec<String>) -> (Vec<String>, Option<Level>)` in `note_status.rs`: removes every `garden/*` tag (case-insensitive prefix), returns the first valid level; prints a warning for an unknown level.
- Published frontmatter gains `status: <level>` and `status_source: manual` when a tag was present.

- [ ] **Step 1: Test in `note_status.rs`**

```rust
    #[test]
    fn split_garden_tag_extracts_level_case_insensitively_and_drops_unknown() {
        let tags = vec!["cooking".to_string(), "garden/Growing".to_string(), "til".to_string()];
        let (rest, level) = split_garden_tag(tags);
        assert_eq!(rest, vec!["cooking", "til"]);
        assert_eq!(level, Some(Level::Growing));

        let (rest, level) = split_garden_tag(vec!["Garden/EVERGREEN".to_string()]);
        assert!(rest.is_empty());
        assert_eq!(level, Some(Level::Evergreen));

        // unknown level: dropped from tags, no level (a warning is printed)
        let (rest, level) = split_garden_tag(vec!["garden/tree".to_string(), "x".to_string()]);
        assert_eq!(rest, vec!["x"]);
        assert_eq!(level, None);

        // no garden tag at all
        let (rest, level) = split_garden_tag(vec!["a".to_string()]);
        assert_eq!(rest, vec!["a"]);
        assert_eq!(level, None);
    }
```

Run `cargo test split_garden` → compile error `cannot find function split_garden_tag`.

- [ ] **Step 2: Implement in `note_status.rs` (above the tests)**

```rust
/// Pull `garden/<level>` out of a note's tag list. Returns the remaining tags
/// (so Hugo never gets a `garden/...` taxonomy term) and the level, if valid.
pub fn split_garden_tag(tags: Vec<String>) -> (Vec<String>, Option<Level>) {
    let mut rest = Vec::with_capacity(tags.len());
    let mut level = None;
    for tag in tags {
        let lower = tag.to_ascii_lowercase();
        match lower.strip_prefix("garden/") {
            Some(value) => match Level::parse(value) {
                Some(l) => {
                    if level.is_none() {
                        level = Some(l);
                    }
                }
                None => eprintln!(
                    "note-status: unknown garden level \"{}\" (use started, growing or evergreen); tag dropped",
                    tag
                ),
            },
            None => rest.push(tag),
        }
    }
    (rest, level)
}
```

`cargo test split_garden` → PASS.

- [ ] **Step 3: Use it in `process_file`**

In `file_utils.rs`, add at the top with the other `use` lines:

```rust
use crate::note_status::{split_garden_tag, Level};
```

Next to `let mut tags: Vec<String> = Vec::new();` (around line 213) add:

```rust
    let mut garden_level: Option<Level> = None;
```

In the `Tags:` branch (around line 331), after the `.collect();` that fills `tags`, replace the `lines.pop(); continue;` pair with:

```rust
            let (rest, level) = split_garden_tag(tags);
            tags = rest;
            garden_level = level;
            lines.pop();
            continue;
```

In the new-frontmatter branch (the `if existing_frontmatter.is_empty()` block, around line 556), after the `createddate` insert and before `frontmatter = format!(...)`:

```rust
            if let Some(level) = garden_level {
                frontmatter_parts.push(format!("status: {}", level.as_str()));
                frontmatter_parts.push("status_source: manual".to_string());
            }
```

In the merge branch, right after the block that ends with `println!("Added {} non-empty tags", tag_count); }` (around line 700):

```rust
            // #garden/<level> on the Tags line -> status frontmatter (spec:
            // docs/superpowers/specs/2026-10-01-note-status-design.md). A vault
            // `status:` key without the tag is dropped so a stale hand-typed
            // value can't masquerade as an author override.
            existing_frontmatter.remove("status");
            existing_frontmatter.remove("status_source");
            if let Some(level) = garden_level {
                existing_frontmatter.insert(
                    "status".to_string(),
                    serde_yaml::Value::String(level.as_str().to_string()),
                );
                existing_frontmatter.insert(
                    "status_source".to_string(),
                    serde_yaml::Value::String("manual".to_string()),
                );
            }
```

- [ ] **Step 4: Prove it on a throwaway vault note (read-only on the real vault)**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo build --release && cargo install --path . --force 2>&1 | tail -1   # install only if PATH uses ~/.cargo/bin, see Task 3 step 6
SCRATCH=/tmp/claude-1000/-home-sspaeti-git-sspaeti-com-second-brain-public-content/8284d12b-be5e-4c89-8c48-e2cd559992f0/scratchpad/garden-tag-check
rm -rf "$SCRATCH"; mkdir -p "$SCRATCH/vault" "$SCRATCH/public"
printf '# Garden Tag Check\n\nSome body text.\n\nTags: #garden/Growing #cooking #publish\n' > "$SCRATCH/vault/garden tag check.md"
printf -- '---\ncreateddate: 2026-10-01\nstatus: evergreen\n---\n# Garden Stale\n\nBody.\n\nTags: #publish\n' > "$SCRATCH/vault/garden stale.md"
printf '# Garden Bad\n\nBody.\n\nTags: #garden/tree #publish\n' > "$SCRATCH/vault/garden bad.md"
cd "$SCRATCH" && secondbrain="$SCRATCH/vault" public_secondbrain="$SCRATCH/public" obsidian-quartz 2>&1 | grep -i "garden" ; head -12 "$SCRATCH/public/garden tag check.md"; echo ---; head -8 "$SCRATCH/public/garden stale.md"; echo ---; head -8 "$SCRATCH/public/garden bad.md"
```

Expected:
- `garden tag check.md` frontmatter has `status: growing`, `status_source: manual`, `tags: [cooking]`, no `garden/` anywhere.
- `garden stale.md` has no `status` line (the hand-typed one was dropped).
- `garden bad.md` prints `note-status: unknown garden level "garden/tree" … tag dropped`, has no `status`, no `tags` line.

(`obsidian-quartz` with no arguments reads `$secondbrain` and writes `$public_secondbrain`; pointing both at the scratch dir keeps the real vault untouched.)

- [ ] **Step 5: Run the whole Rust suite and commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test 2>&1 | grep -E "test result|FAILED"
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add utils/obsidian-quartz/src/file_utils.rs utils/obsidian-quartz/src/note_status.rs
git commit -m "note-status: #garden/<level> tag becomes status frontmatter

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

Expected: every `test result:` line says `0 failed` (the `hugo_render` test needs `hugo` on PATH; it runs in Task 5 anyway).

---

### Task 5: Hugo: level word, merged popover, list meta, link card; render test

**Files:**
- Modify: `layouts/_default/single.html:25-43` (the `.note-changes` block)
- Modify: `layouts/partials/page-list.html:5-7` (the `<p class="meta">`)
- Modify: `assets/js/popover-v2.js:32-36`
- Modify: `assets/styles/custom.scss:2084-2100` (`article > .meta`, `.note-changes`) and append `.nc-status*` rules after `.nc-del`
- Modify: `docs/superpowers/specs/2026-10-01-note-status-design.md` (popover anchoring sentence)
- Create: `utils/obsidian-quartz/tests/hugo-render/data/note_status.json`, `utils/obsidian-quartz/tests/hugo-render/data/recent_updates.json`, `utils/obsidian-quartz/tests/hugo-render/content/status note.md`, `utils/obsidian-quartz/tests/hugo-render/content/manual note.md`
- Modify: `utils/obsidian-quartz/tests/hugo-render/config.toml` (mount the fixture `data` dir before the real one)
- Modify: `utils/obsidian-quartz/tests/hugo_render.rs` (new test)

**Interfaces:**
- Consumes: `data/note_status.json` shape from Task 3.
- Produces: HTML `<span class="note-changes" …><span class="nc-label">growing</span><span class="nc-popover">…</span></span>` on every note with a status entry.

- [ ] **Step 1: Fixtures**

`utils/obsidian-quartz/tests/hugo-render/data/note_status.json`:

```json
{
  "status note": {"status": "growing", "source": "git", "words": 95, "sessions": 2, "span_days": 3, "reason": "Estimated from edit history · 2 sessions over 3 days · 95 words"},
  "manual note": {"status": "evergreen", "source": "manual", "words": 40, "sessions": 0, "span_days": 0, "reason": "Set by author"}
}
```

`utils/obsidian-quartz/tests/hugo-render/data/recent_updates.json`:

```json
{
  "status note": {"status": "updated", "words": 12, "sessions": [
    {"date": "Oct 1", "iso": "2026-10-01", "rel": "today", "added": 12, "removed": 3},
    {"date": "Sep 28", "iso": "2026-09-28", "rel": "3 days ago", "kind": "published", "words": 83}
  ]}
}
```

`utils/obsidian-quartz/tests/hugo-render/content/status note.md`:

```markdown
---
createddate: 2026-09-28
lastmod: 2026-10-01 09:00:00
title: "Status Note"
---
A growing note with two sessions.
```

`utils/obsidian-quartz/tests/hugo-render/content/manual note.md`:

```markdown
---
createddate: 2026-09-28
lastmod: 2026-10-01 09:00:00
status: evergreen
status_source: manual
title: "Manual Note"
---
An author-tagged note with no git history yet.
```

In `utils/obsidian-quartz/tests/hugo-render/config.toml`, insert **before** the existing `source = "data"` mount (earlier mounts win on path clashes, so the fixture JSON shadows the real build output):

```toml
  [[module.mounts]]
    source = "utils/obsidian-quartz/tests/hugo-render/data"
    target = "data"
```

- [ ] **Step 2: The failing render test**

Append to `utils/obsidian-quartz/tests/hugo_render.rs`:

```rust
/// The whole generated page for a slug (the meta line sits outside e-content).
fn page(out: &Path, slug: &str) -> String {
    std::fs::read_to_string(out.join(slug).join("index.html")).unwrap()
}

#[test]
fn note_status_word_and_merged_popover() {
    let out = build();

    // git-classified note: level word at the END of the meta line, popover
    // with the status section first and the sessions list under it.
    let p = page(&out, "status-note");
    let meta_start = p.find(r#"<p class="meta popover-hint">"#).expect("meta line");
    let meta = &p[meta_start..p[meta_start..].find("</p>").unwrap() + meta_start];
    assert!(meta.contains(r#"<span class="nc-label">growing</span>"#), "level word is the trigger label:\n{meta}");
    assert!(!meta.contains("recently updated"), "the old label is gone");
    let label = meta.find(r#"<span class="nc-label">growing</span>"#).unwrap();
    assert!(label > meta.find("min read").expect("min read"), "level word comes after min read");
    assert!(meta.contains(r#"<span class="nc-title">Note status</span>"#));
    assert!(meta.contains("<strong>Growing.</strong> Worked on, still rough. Expect bullets, gaps and views that move."));
    assert!(meta.contains("Estimated from edit history · 2 sessions over 3 days · 95 words"));
    assert!(meta.contains(r#"href="/brain/taxonomy-of-note-types/""#), "link to the taxonomy note");
    assert!(meta.contains("How my notes grow"));
    let status_pos = meta.find("Note status").unwrap();
    let changes_pos = meta.find(r#"<span class="nc-title">Recent changes</span>"#).expect("sessions section");
    assert!(status_pos < changes_pos, "status section above recent changes");
    assert!(meta.contains(r#"<span class="nc-add">+12</span>"#), "session rows still render");
    assert!(meta.contains("published"), "creation row still renders");

    // manual note without sessions: status section only
    let p = page(&out, "manual-note");
    assert!(p.contains(r#"<span class="nc-label">evergreen</span>"#));
    assert!(p.contains("<strong>Evergreen.</strong> Own words, tended over time. Still grows, never finished."));
    assert!(p.contains("Set by author"));
    assert!(!p.contains("Recent changes"), "no sessions, no recent-changes section");

    // note absent from note_status.json: no trigger at all
    let p = page(&out, "embedder");
    assert!(!p.contains(r#"class="note-changes""#), "no status entry, no trigger");

    let _ = std::fs::remove_dir_all(&out);
}
```

Run:

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test --test hugo_render note_status 2>&1 | tail -15
```

Expected: FAIL at `level word is the trigger label` (the template still prints "recently updated").

- [ ] **Step 3: `single.html`**

Replace lines 25–43 (from `{{ with (index hugo.Data.recent_updates .File.BaseFileName) }}{{ with .sessions }} ·` through the matching `{{ end }}{{ end }}`) with:

```html
          {{- /* Note status + recent changes, one popover. The level word
                 (data/note_status.json, `obsidian-quartz note-status`) is the
                 trigger; the sessions list (data/recent_updates.json) follows
                 when there is one. Spec: docs/superpowers/specs/2026-10-01-note-status-design.md */ -}}
          {{ $stem := .File.BaseFileName }}
          {{ $ns := "" }}{{ with hugo.Data.note_status }}{{ $ns = index . $stem }}{{ end }}
          {{ $ru := "" }}{{ with hugo.Data.recent_updates }}{{ $ru = index . $stem }}{{ end }}
          {{ with $ns }} ·
          <span class="note-changes" tabindex="0" role="button" aria-label="Note status: {{ .status }}. Show what that means and the recent changes">
            <span class="nc-label">{{ .status }}</span>
            <span class="nc-popover" role="tooltip">
              <span class="nc-title">Note status</span>
              {{- /* one line on purpose: the test asserts "</strong> Worked on…" with a single space */ -}}
              <span class="nc-status"><strong>{{ .status | title }}.</strong> {{ if eq .status "started" }}Quick capture, barely worked on. May change or vanish.{{ else if eq .status "growing" }}Worked on, still rough. Expect bullets, gaps and views that move.{{ else }}Own words, tended over time. Still grows, never finished.{{ end }}</span>
              <span class="nc-status-src">{{ .reason }} · <a href="{{ "taxonomy-of-note-types/" | relURL }}">How my notes grow</a></span>
              {{ with $ru }}{{ with .sessions }}
              <span class="nc-sep"></span>
              <span class="nc-title">Recent changes</span>
              {{ range . }}
              <span class="nc-row">
                <span class="nc-when"><time datetime="{{ .iso }}">{{ .date }}</time><span class="nc-rel">{{ .rel }}</span></span>
                {{ if .kind }}
                <span class="nc-delta"><span class="nc-add">{{ .kind }}</span> · {{ .words }} words</span>
                {{ else }}
                <span class="nc-delta"><span class="nc-add">+{{ .added }}</span> / <span class="nc-del">−{{ .removed }}</span> words</span>
                {{ end }}
              </span>
              {{ end }}
              {{ end }}{{ end }}
            </span>
          </span>
          {{ end }}
```

The tap-toggle `<script>` below it stays as is (it targets `.note-changes`, which is unchanged).

- [ ] **Step 4: `page-list.html`**

Replace the `<p class="meta">` block with (inside `with hugo.Data.note_status` the dot is the map, so the stem is captured in a variable first; `$` would be the partial's whole collection, not the row):

```html
            {{- $stem := "" }}{{ if .File }}{{ $stem = .File.BaseFileName }}{{ end }}
            <p class="meta">
                {{partial "date-fmt.html" .}}{{ with hugo.Data.note_status }}{{ with index . $stem }} · {{ .status }}{{ end }}{{ end }}
            </p>
```

- [ ] **Step 5: `popover-v2.js`**

Replace the comment and selector at lines 32–36 with:

```js
    // Strip the per-note status/edit-history popover body, keep its trigger
    // word: in a hover card the level ("growing") is useful, the nested
    // popover is not, and sites without the brain's CSS (e.g. the blog) would
    // render the whole history as stray grey text.
    html.querySelectorAll(".nc-popover").forEach((el) => el.remove());
```

- [ ] **Step 6: SCSS**

In `custom.scss`, the `.nc-popover` rule (around line 2100) keeps anchoring to the word (`.note-changes { position: relative }` is unchanged) but opens to the **left** of it, since the word is now the last item on the line: change `left: 0;` to

```scss
  left: auto;
  right: 0;  // the word ends the meta line; the box grows leftwards and stays inside the card
```

Hovering straight down from the word still lands on the box (its top-right corner sits under the word), so the existing `::before` hover bridge keeps working. On narrow screens the meta line wraps and the word can sit near the left edge, where a 250–360px box anchored right would overflow the card; there the tap-opened popover becomes a bottom sheet. Append after the `.nc-popover::before { … }` rule:

```scss
@media (max-width: 640px) {
  .nc-popover {
    position: fixed;
    left: 1rem;
    right: 1rem;
    top: auto;
    bottom: 1rem;
    max-width: none;
    max-height: 70vh;
  }
}
```

`position: fixed` is relative to the viewport only if no ancestor has `transform`/`filter`/`contain`; check in Step 9 with the window narrowed (DevTools device mode). If the sheet opens somewhere odd, replace the media block with `left: 0; right: auto; max-width: calc(100vw - 2rem);` and say so in the summary.

Update the block comment above `:root { --badge-del … }` so its first sentence reads: `// The note's level word ("started" / "growing" / "evergreen") after "min read" on a note page, separated by the same " · " as the other meta items (single.html). Hover (desktop) or tap/focus (mobile) reveals a popover with the level's one-line meaning and reason (data/note_status.json), then the recent editing sessions with per-session added/removed word counts (data/recent_updates.json).` Keep the rest.

Append after the `.nc-del { … }` rule:

```scss
.nc-status {
  display: block;
  margin-bottom: 0.35em;
  line-height: 1.4;
}
.nc-status-src {
  display: block;
  font-size: 0.86em;
  color: var(--global-font-secondary-color);
  a {
    color: inherit;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
}
.nc-sep {
  display: block;
  height: 1px;
  margin: 0.6em 0 0.55em;
  background: color-mix(in srgb, var(--outlinegray) 55%, transparent);
}
```

- [ ] **Step 7: Spec touch-up**

In the spec's "What it looks like" list, append to the bullet beginning "The popover is anchored to the right edge of the trigger":

```markdown
  Below 640px, where the wrapped meta line can put the word near the left edge, the tap-opened
  popover is a fixed bottom sheet (`left/right: 1rem; bottom: 1rem`) instead.
```

- [ ] **Step 8: Run the render test, then the full suite**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public/utils/obsidian-quartz && cargo test --test hugo_render 2>&1 | tail -5
cd /home/sspaeti/git/sspaeti.com/second-brain-public && make test 2>&1 | grep -E "test result|OK|FAILED|Error"
```

Expected: both `hugo_render` tests PASS; `make test` all green.

- [ ] **Step 9: Look at it**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public && make run
```

Open `http://localhost:1313/brain/format-usb-drive-on-linux/` and `/brain/will-ai-replace-humans/`: the meta line ends with `· started` / `· evergreen` in dotted underline; hover shows "Note status" then "Recent changes"; tap on a narrow window toggles it; dark mode via the header toggle; the home page Recent Notes rows read `Oct 1, 2026 · started`; hovering an internal link to either note shows the word with no nested popover. Stop the server.

- [ ] **Step 10: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add layouts/_default/single.html layouts/partials/page-list.html assets/js/popover-v2.js assets/styles/custom.scss utils/obsidian-quartz/tests docs/superpowers/specs/2026-10-01-note-status-design.md
git commit -m "note-status: level word on the meta line with a merged status + recent-changes popover

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: Docs: CHANGELOG, CLAUDE.md, README utils list

**Files:**
- Modify: `CHANGELOG.md` (new top entry)
- Modify: `CLAUDE.md` (Architecture bullet + Content Processing Workflow step)
- Modify: `README.md` `## Utils` section (one line, if the section lists the other `obsidian-quartz` subcommands; otherwise skip)

- [ ] **Step 1: CHANGELOG entry at the top, in the existing style**

```markdown
### 2026-10-01: Note status on every note: started / growing / evergreen

- **Meta line**: the "recently updated" label is now the note's level word
  (`started`, `growing`, `evergreen`), same dotted underline, on every note.
  Hover/tap opens one popover: what the level means, how it was decided
  ("Estimated from edit history · 23 sessions over 2 years · 3,120 words" or
  "Set by author"), a link to the taxonomy note, then the recent-changes list
  that was already there. Recent Notes rows and link hover cards show the word.
- **Build**: new `obsidian-quartz note-status` (Rust, `utils/obsidian-quartz/src/note_status.rs`)
  runs in `make prepare` after `recent_updates.py`, joins its sessions with
  word counts, writes `data/note_status.json` for Hugo and a gitignored
  `note_status_report.md` for me (manual-vs-git disagreements first).
  Thresholds in `config.toml` (`noteStatus*`).
- **Vault override**: `#garden/started|growing|evergreen` on a note's `Tags:`
  line becomes `status:` / `status_source: manual` in the published frontmatter
  and wins over the heuristic. Old emoji status tags stay ignored.
- **Performance**: no new request or asset; Lighthouse before/after on
  `will-ai-replace-humans/`: <fill in from Task 7>.
- Spec: `docs/superpowers/specs/2026-10-01-note-status-design.md`.
```

- [ ] **Step 2: CLAUDE.md**

In `## Architecture` add a bullet after the **Margin sidenotes** one:

```markdown
- **Note status**: `obsidian-quartz note-status` (`utils/obsidian-quartz/src/note_status.rs`) classifies every note as `started` / `growing` / `evergreen` from `data/recent_updates.json` sessions + word count, or a `#garden/<level>` vault tag (→ `status` frontmatter via `file_utils.rs`). Writes `data/note_status.json` (rendered by `single.html` as the level word + popover, and by `page-list.html`) and `note_status_report.md` (author-only, gitignored). Thresholds: `config.toml` `noteStatus*`. Spec: `docs/superpowers/specs/2026-10-01-note-status-design.md`.
```

In `## Content Processing Workflow`, between steps 2 and 3 insert:

```markdown
3. `recent_updates.py` then `obsidian-quartz note-status` derive per-note change badges and the level word from git history
```

and renumber the following steps.

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add CHANGELOG.md CLAUDE.md README.md
git commit -m "docs: note status (changelog, CLAUDE.md)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 7: Lighthouse after, full `make prepare`, final check

**Files:**
- Modify: `CHANGELOG.md` (fill the Lighthouse line)

- [ ] **Step 1: Full pipeline**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public && make prepare 2>&1 | grep -E "note-status|recent_updates|Error|error" | head
```

Expected: the `note-status: … notes -> …` summary line, no errors. `git status --short` shows nothing new under `data/` (both outputs gitignored).

- [ ] **Step 2: Lighthouse after**

```bash
make lighthouse P=will-ai-replace-humans/ 2>&1 | tail -6
```

Compare with `lighthouse-before.txt` from Task 0: mobile and desktop performance scores within ±2 and no new request in the HTML report's network tab (`/tmp/brain-lighthouse/mobile.report.html`). Put both score lines into the CHANGELOG entry's **Performance** bullet. If the score dropped by more than 2, the only candidate is page weight from the popover HTML; check the note's `index.html` size before/after (`wc -c public/will-ai-replace-humans/index.html`) and report it rather than guessing.

- [ ] **Step 3: Commit**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
git add CHANGELOG.md
git commit -m "docs: lighthouse before/after for note status

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 8: One-off Jev calibration (throwaway, needs `JEV_API_KEY` or `JEV_TYPESAVE_AI`)

**Files:**
- Create: `<scratchpad>/jev_status_check.py` (never committed, not in `utils/`)
- Output: `<scratchpad>/jev_status_check.md`

**Interfaces:**
- Consumes: `data/note_status.json` (Task 3), `content/*.md`.
- Produces: a Markdown list of notes where Jev's level ≠ heuristic level with confidence ≥ 0.7, for the author to tag in the vault.

- [ ] **Step 1: Write the script**

```python
#!/usr/bin/env python3
"""One-off: ask Jev for a level per public note, list where it disagrees with
data/note_status.json. Throwaway; not part of make. Run from the repo root.

  python3 jev_status_check.py            # first 20 notes, prints token usage
  python3 jev_status_check.py --all      # every note
"""
import json, os, sys, time, urllib.request, concurrent.futures as cf
from pathlib import Path

ROOT = Path("/home/sspaeti/git/sspaeti.com/second-brain-public")
KEY = os.environ.get("JEV_API_KEY") or os.environ.get("JEV_TYPESAVE_AI")
ENDPOINT = os.environ.get("JEV_ENDPOINT", "https://jevtypesafeai.com/api/v1/decide")
OUT = Path(__file__).with_suffix(".md")
MAX_WORDS = 1200          # trim long notes; the first screen decides the level
MIN_CONF = 0.7

SENTENCES = {
    "started": "Quick capture, barely worked on. A few lines, a snippet or a lookup; may change or vanish.",
    "growing": "Worked on, still rough. Bullets, gaps, references, views that still move; useful but not polished.",
    "evergreen": "Own words, tended over time, coherent prose with structure. Still grows, but a reader can rely on it.",
}

def body(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="ignore")
    if text.startswith("---"):
        end = text.find("\n---", 4)
        if end >= 0:
            text = text[end + 4:]
    words = text.split()
    return " ".join(words[:MAX_WORDS])

def ask(stem: str, text: str) -> dict:
    req = {
        "state": f"Note title: {stem}\n\n{text}",
        "questions": {
            "level": {"type": "choice", "instructions": "How mature is this public note for a reader?", "criteria": SENTENCES},
            "polish": {"type": "score", "instructions": "How polished is the writing?",
                       "criteria": ["scratch/bullets", "rough prose", "readable", "clean prose", "publication quality"]},
        },
    }
    data = json.dumps(req).encode()
    r = urllib.request.Request(ENDPOINT, data=data, headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(r, timeout=60) as resp:
                return json.load(resp)
        except Exception as e:  # noqa: BLE001
            if attempt == 2:
                return {"error": str(e)}
            time.sleep(2 * (attempt + 1))

def main() -> None:
    if not KEY:
        sys.exit("set JEV_API_KEY (or JEV_TYPESAVE_AI)")
    status = json.loads((ROOT / "data/note_status.json").read_text())
    stems = sorted(status)
    if "--all" not in sys.argv:
        stems = stems[:20]
    rows, tokens, cost = [], 0, 0.0
    with cf.ThreadPoolExecutor(max_workers=4) as pool:
        futs = {pool.submit(ask, s, body(ROOT / "content" / f"{s}.md")): s for s in stems}
        for fut in cf.as_completed(futs):
            s = futs[fut]; res = fut.result()
            if "error" in res:
                rows.append((s, status[s]["status"], "error", 0.0, 0.0, res["error"])); continue
            a = res["answers"]; u = res.get("usage", {})
            tokens += u.get("input_tokens", 0); cost += u.get("cost_usd", 0.0)
            rows.append((s, status[s]["status"], a["level"]["choice"], a["level"].get("confidence", 0.0), a["polish"].get("score", 0.0), status[s]["reason"]))
    rows.sort(key=lambda r: (-(r[3] if r[1] != r[2] else -1), r[0]))
    dis = [r for r in rows if r[1] != r[2] and r[3] >= MIN_CONF]
    lines = [f"# Jev vs heuristic · {len(rows)} notes · {len(dis)} disagreements at confidence >= {MIN_CONF}",
             f"input tokens {tokens:,} · cost ${cost:.2f}", "",
             "| note | heuristic | jev | conf | polish | heuristic reason |", "|---|---|---|---|---|---|"]
    for s, h, j, c, p, reason in dis:
        lines.append(f"| {s} | {h} | {j} | {c:.2f} | {p:.1f} | {reason} |")
    lines += ["", "## All notes", "", "| note | heuristic | jev | conf | polish |", "|---|---|---|---|---|"]
    for s, h, j, c, p, _ in rows:
        lines.append(f"| {s} | {h} | {j} | {c:.2f} | {p:.1f} |")
    OUT.write_text("\n".join(lines) + "\n")
    print(f"{len(rows)} notes, {len(dis)} disagreements, {tokens:,} input tokens, ${cost:.2f} -> {OUT}")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Dry run on 20 notes, read the cost, then the full run**

```bash
cd /home/sspaeti/git/sspaeti.com/second-brain-public
python3 /tmp/claude-1000/-home-sspaeti-git-sspaeti-com-second-brain-public-content/8284d12b-be5e-4c89-8c48-e2cd559992f0/scratchpad/jev_status_check.py
```

Expected: `20 notes, N disagreements, ~20,000 input tokens, $0.0x`. Multiply by ~77 for the full run; if that is above a few dollars, stop and report the estimate to the author instead of running `--all`. Otherwise:

```bash
python3 /tmp/claude-1000/-home-sspaeti-git-sspaeti-com-second-brain-public-content/8284d12b-be5e-4c89-8c48-e2cd559992f0/scratchpad/jev_status_check.py --all
```

If the key is missing (`set JEV_API_KEY`), report that and skip; the feature does not depend on this task.

- [ ] **Step 3: Hand the list to the author**

In the final summary, give the disagreement table (or its top 30 if longer) and the two notes the author named (`format usb drive on linux`, `will ai replace humans`) with both verdicts. Suggest threshold changes only if a whole band is off (for example most 100–120-word single-session notes come back `growing` → lower `noteStatusStartedWords`). The author adds `#garden/*` tags in the vault; nothing here writes there.

---

## Self-review notes

- Spec coverage: vocabulary + sentences (Task 5 template, Task 1 tests), meta line + merged popover (Task 5), list + link card (Task 5), manual tag mapping (Task 4), heuristic + thresholds + JSON (Tasks 1–3), report (Task 3), Jev one-off (Task 8), Hugo wiring list incl. config/Makefile/.gitignore/README changelog (Tasks 3, 6), performance budget (Tasks 0, 7), verification list (Tasks 3–5, 7), edge cases: malformed level (Task 4), renames (upstream, keyed by current stem: Task 3), `_index`/hidden meta (Task 3 skip + Task 5 `with`), zero-word sessions (Task 2), code blocks counted (Task 1 word_count test), missing dates (Task 3 fallback test).
- Interfaces: `Level::as_str/parse/rank`, `Source::as_str`, `Thresholds`, `NoteFacts`, `classify`, `reason`, `word_count`, `FrontmatterFields`, `frontmatter_fields`, `sessions_facts`, `thresholds_from_toml`, `Entry`, `build_entries`, `entries_to_json`, `render_report`, `run`, `split_garden_tag` are named identically in every task.
- Known judgment call surfaced to the author: with the default thresholds `format usb drive on linux` is `started`; Task 8's calibration decides whether `noteStatusStartedWords` moves.
