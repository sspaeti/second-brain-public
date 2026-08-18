//! Canonical brain slug rules — the single Rust copy.
//!
//! KEEP IN SYNC WITH: ../../hugo-obsidian/util.go::UnicodeSanitize
//!
//! `UnicodeSanitize` is what Hugo applies to a filename to get its URL, and
//! what hugo-obsidian applies when building the link indices. Everything in
//! this crate that needs a brain URL (blog/book/memory enrichment, the search
//! index, frontmatter aliases) goes through here so the rules cannot drift.

/// Rust port of hugo-obsidian's `UnicodeSanitize`.
///
/// Drops parens, apostrophes, en-dashes, commas, `&`, `@` …; collapses runs of
/// `-`/whitespace into a single `-`. Does *not* lowercase — callers that start
/// from human-cased text should use [`brain_slug`].
pub fn unicode_sanitize(input: &str) -> String {
    let source: Vec<char> = input.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(source.len());
    let mut prepend_hyphen = false;
    for (i, &r) in source.iter().enumerate() {
        let is_punct_allowed =
            r == '.' || r == '/' || r == '\\' || r == '_' || r == '#' || r == '+' || r == '~';
        let is_pct_escape = r == '%'
            && i + 2 < source.len()
            && source[i + 1].is_ascii_hexdigit()
            && source[i + 2].is_ascii_hexdigit();
        // Note: Go's unicode.IsMark has no stable stdlib equivalent.
        // is_alphanumeric() handles precomposed Unicode letters (ö, é, …)
        // which is what brain filenames use in practice.
        let is_allowed = is_punct_allowed || r.is_alphanumeric() || is_pct_escape;
        if is_allowed {
            if prepend_hyphen {
                out.push('-');
                prepend_hyphen = false;
            }
            out.push(r);
        } else if !out.is_empty() && (r == '-' || r.is_whitespace()) {
            prepend_hyphen = true;
        }
    }
    out.into_iter().collect()
}

/// Brain URL slug for human-cased text (a note filename stem, a frontmatter
/// alias). Mirrors the two steps a note goes through: `file_utils` writes the
/// filename lowercased into `content/`, then Hugo sanitizes it.
///
/// `"Directed Acyclic Graphs (DAGs)"` -> `"directed-acyclic-graphs-dags"`
pub fn brain_slug(input: &str) -> String {
    // `unicode_sanitize` keeps '/' and '\\' so it can sanitize whole paths for the
    // link indices. A slug is a single URL segment, so fold separators into '-'
    // first — otherwise the alias "No-Code / Less-Code vs Code" would publish a
    // nested directory instead of one page. Matches hugo-obsidian's
    // processSource, which replaces " / " before sanitizing.
    let flattened: String = input
        .to_lowercase()
        .chars()
        .map(|c| if c == '/' || c == '\\' { '-' } else { c })
        .collect();
    unicode_sanitize(&flattened)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_hugo_urls_for_real_notes() {
        // filename stem -> the directory Hugo actually publishes
        assert_eq!(brain_slug("personal knowledge management (pkm)"), "personal-knowledge-management-pkm");
        assert_eq!(brain_slug("change data capture (cdc)"), "change-data-capture-cdc");
        assert_eq!(brain_slug("modern olap systems"), "modern-olap-systems");
        assert_eq!(brain_slug("no meetings (async)"), "no-meetings-async");
    }

    #[test]
    fn slugs_aliases() {
        assert_eq!(brain_slug("Directed Acyclic Graphs"), "directed-acyclic-graphs");
        assert_eq!(brain_slug("Directed Acyclic Graphs (DAGs)"), "directed-acyclic-graphs-dags");
        assert_eq!(brain_slug("OLAP Cubes"), "olap-cubes");
        assert_eq!(brain_slug("OLAP"), "olap");
        assert_eq!(brain_slug("No-Code / Less-Code vs Code"), "no-code-less-code-vs-code");
        assert_eq!(brain_slug("  spaced  out  "), "spaced-out");
        assert_eq!(brain_slug("()"), "");
    }
}
