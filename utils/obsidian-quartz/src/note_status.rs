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
