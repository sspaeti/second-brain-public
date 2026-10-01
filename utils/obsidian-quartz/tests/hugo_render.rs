//! Regression tests for the Hugo template `layouts/partials/textprocessing.html`
//! (embeds/transclusions, wikilinks, blockquotes and callouts).
//!
//! The template can only be exercised by Hugo itself, so this test builds the
//! fixture notes in `tests/hugo-render/content` with the site's real layouts
//! (`config.toml` + `tests/hugo-render/config.toml`, which only swaps the content
//! mount) and asserts on the generated HTML.
//!
//!     cargo test --test hugo_render        # or `make test` in the repo root
//!
//! When an embed/callout/wikilink rendering bug is fixed, add a fixture line and
//! an assertion here so it cannot silently come back.

use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn build() -> PathBuf {
    let root = repo_root();
    // Hugo silently builds the real content/ if the fixture mount is missing.
    assert!(
        root.join("utils/obsidian-quartz/tests/hugo-render/content/embedder.md").is_file(),
        "fixture notes missing under tests/hugo-render/content"
    );
    let out = std::env::temp_dir().join(format!("brain-hugo-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new("hugo")
        .current_dir(&root)
        .args(["--quiet", "--config", "config.toml,utils/obsidian-quartz/tests/hugo-render/config.toml", "-d"])
        .arg(&out)
        .status()
        .expect("hugo must be installed and on PATH");
    assert!(status.success(), "hugo build of the test fixtures failed");
    out
}

/// The note body as textprocessing.html emitted it (`<div class="e-content">` up
/// to the newsletter footer that single.html appends after it).
fn content(out: &Path, slug: &str) -> String {
    let html = std::fs::read_to_string(out.join(slug).join("index.html")).unwrap();
    let start = html.find(r#"<div class="e-content popover-hint">"#).expect("e-content div");
    let end = html[start..].find(r#"<div class="nl-footer">"#).expect("nl-footer div") + start;
    html[start..end].to_string()
}

/// The `<div class="transclusion">` that directly follows the paragraph `label`.
fn transclusion_for(html: &str, label: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"(?s){}</p><div class="transclusion">(.*?)</blockquote></div>"#,
        regex::escape(label)
    ))
    .unwrap();
    re.captures(html).map(|c| c[1].to_string())
}

fn heading_ids(html: &str) -> HashSet<String> {
    Regex::new(r#"<h[1-6] id="([^"]+)""#)
        .unwrap()
        .captures_iter(html)
        .map(|c| c[1].to_string())
        .collect()
}

/// The `#anchor` of the `<a href="/brain/source-note/#...">` whose text is `text`.
fn anchor_of_link(html: &str, text: &str) -> Option<String> {
    Regex::new(&format!(r#"<a href="/brain/source-note/#([^"]+)"[^>]*>{}</a>"#, regex::escape(text)))
        .unwrap()
        .captures(html)
        .map(|c| c[1].to_string())
}

fn is_match(html: &str, re: &str) -> bool {
    Regex::new(re).unwrap().is_match(html)
}

#[test]
fn textprocessing_renders_embeds_wikilinks_and_callouts() {
    let out = build();
    let page = content(&out, "embedder");
    let source = content(&out, "source-note");
    let source_ids = heading_ids(&source);

    // --- heading embeds ---------------------------------------------------
    let t = transclusion_for(&page, "Heading embed, plain:").expect("plain heading embed renders the section");
    assert!(t.contains("Plain section body."), "plain heading embed body");
    assert!(t.contains(r#"href="/brain/source-note/#plain-heading""#), "plain heading embed links to the heading anchor");

    // Apostrophes: goldmark's typographer turns ' into &rsquo; in the rendered
    // HTML the template reads, which used to make the heading lookup fail and
    // the embed collapse to a bare link (![[AI Writing#There's no way ...]]).
    let t = transclusion_for(&page, "Heading embed with comma and apostrophe:")
        .expect("heading with comma + apostrophe embeds the section (not a bare link)");
    assert!(t.contains("Apostrophe section body"), "apostrophe heading embed body");
    let anchor = Regex::new(r#"href="/brain/source-note/#([^"]+)""#).unwrap().captures(&t).map(|c| c[1].to_string());
    assert!(
        anchor.as_deref().map_or(false, |a| source_ids.contains(a)),
        "apostrophe heading embed anchor must be a real heading id, got {:?} (ids: {:?})",
        anchor, source_ids
    );

    let t = transclusion_for(&page, "Heading embed whose section is a callout:").expect("callout section embed renders");
    assert!(t.contains("Callout body text inside the source note."), "callout section embed body");
    assert!(!t.contains("[!note]"), "embedded callout: no literal [!note]");
    assert_eq!(t.matches("<blockquote").count(), 1, "embedded callout: no nested blockquote");

    // --- block + whole-note embeds ---------------------------------------
    let t = transclusion_for(&page, "Block embed:").expect("block reference embeds its paragraph");
    assert!(t.contains("Block paragraph text with a block id."), "block embed body");
    assert!(!t.contains("^blk123"), "block id is stripped from the embed");
    assert!(t.contains(r#"href="/brain/source-note/#last-heading""#), "block embed links to its enclosing heading");

    let t = transclusion_for(&page, "Whole note embed:").expect("whole-note embed renders the note");
    assert!(t.contains("Intro paragraph of the source note."), "whole-note embed body");
    assert!(!t.contains("Origin:") && !t.contains("Somewhere Else"), "whole-note embed drops the Origin footer");
    assert!(!t.contains("[!note]"), "whole-note embed flattens callouts");

    // --- wikilinks ----------------------------------------------------------
    assert!(
        is_match(&page, r#"<a href="/brain/dont-stop/"[^>]*>Don&rsquo;t Stop</a>"#),
        "wikilink to a note with an apostrophe in its title resolves"
    );
    assert!(
        is_match(&page, r#"<a href="/brain/dash-note/"[^>]*>Dash (–|&ndash;) Note</a>"#),
        "wikilink to a note whose file name has a real en dash resolves"
    );
    for (text, what) in [
        ("apostrophe heading", "apostrophe heading"),
        ("punctuated heading", "heading with colon/parens/&"),
        ("code heading", "heading with inline `code`"),
    ] {
        let a = anchor_of_link(&page, text);
        assert!(
            a.as_deref().map_or(false, |a| source_ids.contains(a)),
            "wikilink anchor to {} must be a real heading id, got {:?} (ids: {:?})",
            what, a, source_ids
        );
    }
    assert_eq!(anchor_of_link(&page, "block link").as_deref(), Some("blk123"), "block-id wikilink keeps its #blockid anchor");
    assert!(page.contains("<code>[[Not A Link]]</code>"), "wikilink inside inline code stays literal");
    assert!(
        page.contains(r#"<img src="/brain/pixel.png""#) && page.contains(r#"width="300""#),
        "image embed with width renders an <img>"
    );

    // --- blockquotes and callouts ------------------------------------------
    assert!(page.contains("<blockquote>\n<p>A plain blockquote, no callout.</p>"), "plain blockquote stays a plain <blockquote>");
    assert!(page.contains(r#"<blockquote class="note-callout">"#), "[!note] becomes a note-callout");
    assert!(
        page.contains(r#"<blockquote class="callout-collapsible callout-collapsed warning-callout">"#),
        "[!warning]- becomes a collapsed collapsible callout"
    );
    assert!(page.contains(r#"<blockquote class="callout-collapsible tip-callout">"#), "[!tip]+ becomes an expanded collapsible callout");
    assert!(!page.contains("[!"), "no literal [!type] markers survive");
    assert!(!page.contains("blockquote class=callout"), "no temporary class=callout marker survives");
    assert_eq!(page.matches("<blockquote").count(), page.matches("</blockquote>").count(), "blockquote tags are balanced");

    let _ = std::fs::remove_dir_all(&out);
}

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
    assert!(meta.contains("<strong>Growing.</strong> Worked on quite a bit, still rough. Expect bullets, gaps and views that update."));
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
    assert!(p.contains("<strong>Evergreen.</strong> Own words, refined over time. Still grows."));
    assert!(p.contains("Set by author"));
    assert!(!p.contains("Recent changes"), "no sessions, no recent-changes section");

    // note absent from note_status.json: no trigger at all
    let p = page(&out, "embedder");
    assert!(!p.contains(r#"class="note-changes""#), "no status entry, no trigger");

    let _ = std::fs::remove_dir_all(&out);
}
