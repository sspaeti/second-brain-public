"""Emit per-note change data from the `content/` git history.

One output, `data/recent_updates.json`, from a single git scan. Keyed by the
on-disk filename stem (== Hugo's `.File.BaseFileName`), each value carries both
the homepage badge fields and the per-note-page popover history:

    {
      "status": "new" | "updated",  # homepage recent-notes badge
      "words": N,                    # new -> total note words; updated -> gross
      "sessions": [ {"date","iso","rel","added","removed"}, ... ]  # newest first
    }

`sessions` is omitted for a note whose only edits touched zero words (e.g. an
image/frontmatter-only change): such a note still gets a badge but no popover.

An *editing session* is the run of commits ending at some commit, grouping
commits whose gap is <= SESSION_GAP_HOURS into one. This coalesces "I fixed it
three times that afternoon" into one entry that matches the single date shown.

The homepage list reads `.status` / `.words`; the note page reads `.sessions`.
The logic is ported (not imported) from ../listmonk-rss/newsletter.py so that
`make prepare` stays self-contained (stdlib only, no third-party deps).
"""

import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTENT = ROOT / "content"
OUTPUT = ROOT / "data" / "recent_updates.json"
CONFIG = ROOT / "config.toml"


def _config_params() -> dict:
    """`[params]` from config.toml, or empty on any read/parse error."""
    try:
        with CONFIG.open("rb") as f:
            return tomllib.load(f).get("params", {})
    except (OSError, tomllib.TOMLDecodeError):
        return {}


_params = _config_params()

# Tunables live in config.toml `[params]` (recentUpdates*); these are fallbacks.
# LOOKBACK_DAYS is the git word-diff scan window (~5y). No viewer cost: each popover
# is capped at MAX_SESSIONS rows and bounded by the note's lastmod; only the local
# build scan grows.
LOOKBACK_DAYS = int(_params.get("recentUpdatesLookbackDays", 1825))
SESSION_GAP_HOURS = int(_params.get("recentUpdatesSessionGapHours", 24))  # fold commits closer than this into one session
MAX_SESSIONS = int(_params.get("recentUpdatesMaxSessions", 7))            # sessions shown in a note's change popover


def _parse_git_date(raw: str) -> datetime:
    """git `%ai` looks like `2026-08-05 14:23:01 +0200`."""
    return datetime.strptime(raw.strip(), "%Y-%m-%d %H:%M:%S %z")


_HUNK_RE = re.compile(r"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@")


def _frontmatter_end_line(path: Path) -> int:
    """1-based line number of a note's closing `---` frontmatter delimiter, so
    lines 1..N are frontmatter and the body starts at N+1. 0 if no frontmatter.
    Approximated from the note's current version and reused for all its commits
    (frontmatter length barely changes), which is enough to keep metadata edits
    -- OG `description:`, `lastmod:`, moved `created:` lines -- out of the count."""
    try:
        lines = path.read_text(encoding="utf-8", errors="ignore").split("\n")
    except OSError:
        return 0
    if not lines or lines[0].strip() != "---":
        return 0
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            return i + 1
    return 0


def _stats_from_diff(text: str) -> dict[str, list[tuple[datetime, int, int]]]:
    """Parse a `git log -p` / `git diff` stream (commits delimited by
    `__COMMIT__%ai` marker lines) into per-note `(commit_datetime, added_words,
    removed_words)`, counted from `+`/`-` diff lines. Frontmatter lines are
    excluded (tracked by file line number) so metadata-only edits don't read as
    content edits."""
    commits: dict[str, list[tuple[datetime, int, int]]] = defaultdict(list)
    cur_date: datetime | None = None
    cur_path: str | None = None
    cur_added = 0
    cur_removed = 0
    is_binary = False
    seen_hunk = False          # True once past the per-file diff header preamble
    fm_end = 0                 # last frontmatter line for the current file
    old_ln = new_ln = 0        # running line numbers within the current hunk

    def flush() -> None:
        nonlocal cur_path, cur_added, cur_removed
        if cur_path is not None and cur_date is not None:
            commits[cur_path].append((cur_date, cur_added, cur_removed))
        cur_path, cur_added, cur_removed = None, 0, 0

    for line in text.splitlines():
        if line.startswith("__COMMIT__"):
            flush()
            cur_date = _parse_git_date(line[len("__COMMIT__"):])
            continue
        if line.startswith("diff --git "):
            flush()
            _, _, b_path = line.partition(" b/")
            cur_path = b_path if b_path.endswith(".md") else None
            is_binary = False
            seen_hunk = False
            fm_end = _frontmatter_end_line(CONTENT / cur_path) if cur_path else 0
            old_ln = new_ln = 0
            continue
        if cur_path is None:
            continue
        if line.startswith("Binary files"):
            is_binary = True
            continue
        if is_binary:
            continue
        m = _HUNK_RE.match(line)
        if m:
            old_ln, new_ln = int(m.group(1)), int(m.group(2))
            seen_hunk = True
            continue
        # A file-creation hunk is `@@ -0,0 +1,N @@`, so `old_ln` stays 0 for the
        # whole hunk -- gate the preamble on "have we seen a hunk yet", not on
        # `old_ln == 0`, or every added line of a new file is dropped (0 words).
        if not seen_hunk:      # still in the diff preamble (index / ---/+++ headers)
            continue
        if line.startswith("+"):
            if new_ln > fm_end:            # body only
                cur_added += len(line[1:].split())
            new_ln += 1
        elif line.startswith("-"):
            if old_ln > fm_end:            # body only
                cur_removed += len(line[1:].split())
            old_ln += 1
        else:                              # context line: advances both sides
            old_ln += 1
            new_ln += 1
    flush()

    return commits


def commit_word_stats(since: datetime) -> dict[str, list[tuple[datetime, int, int]]]:
    """Per note, a list of `(commit_datetime, added_words, removed_words)` for
    committed changes since `since`."""
    result = subprocess.run(
        [
            "git", "-c", "core.quotePath=false", "-C", str(CONTENT), "log",
            f"--since={since.strftime('%Y-%m-%d %H:%M:%S')}",
            "-p", "--src-prefix=a/", "--dst-prefix=b/",
            "--format=__COMMIT__%ai", "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    )
    return _stats_from_diff(result.stdout)


def worktree_word_stats(now: datetime) -> tuple[dict[str, list[tuple[datetime, int, int]]], set[str]]:
    """Uncommitted `content/` changes folded into a single `now` session, so a
    freshly prepared note gets a badge/popover before the content submodule is
    committed (the generator otherwise sees only committed history).

    Returns `(stats, new_paths)` where `stats` mirrors `commit_word_stats` (dated
    `now`) and `new_paths` is the set of untracked -- brand-new -- note paths, so
    the caller can treat them as creations rather than edits."""
    marker = "__COMMIT__" + now.strftime("%Y-%m-%d %H:%M:%S %z")

    # Tracked-but-modified notes: reuse the diff parser on the working-tree diff.
    # `--src-prefix/--dst-prefix` pin the `a/`..`b/` headers the parser keys on:
    # with the user's `diff.mnemonicPrefix=true`, `git diff HEAD` emits `c/`..`w/`
    # instead and every uncommitted edit parses to nothing (note reads NEW again).
    diff = subprocess.run(
        [
            "git", "-c", "core.quotePath=false", "-C", str(CONTENT),
            "diff", "HEAD", "--src-prefix=a/", "--dst-prefix=b/", "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    ).stdout
    stats = _stats_from_diff(marker + "\n" + diff) if diff.strip() else defaultdict(list)

    # Untracked notes have no diff base -- the whole body is "added".
    others = subprocess.run(
        [
            "git", "-C", str(CONTENT), "ls-files", "--others", "--exclude-standard",
            "-z", "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    ).stdout
    new_paths = {p for p in others.split("\0") if p.endswith(".md")}
    for p in new_paths:
        full = CONTENT / p
        if full.exists():
            stats.setdefault(p, []).append((now, _word_count(full), 0))
    return stats, new_paths


def first_commit_dates() -> dict[str, datetime]:
    """Each note's first-ever commit datetime (full history, no --since), so a
    fresh add is never mistaken for an edit to an old note."""
    result = subprocess.run(
        [
            "git", "-c", "core.quotePath=false", "-C", str(CONTENT), "log",
            "--reverse", "--diff-filter=A", "--name-only", "--format=__COMMIT__%ai",
            "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    )
    first: dict[str, datetime] = {}
    cur_date: datetime | None = None
    for line in result.stdout.splitlines():
        if line.startswith("__COMMIT__"):
            cur_date = _parse_git_date(line[len("__COMMIT__"):])
        elif line.endswith(".md") and line not in first and cur_date is not None:
            first[line] = cur_date
    return first


def group_sessions(dated: list[tuple[datetime, int, int]]) -> list[dict]:
    """Fold a note's commits into sessions, newest first. Each session:
    `{"start", "end", "added", "removed"}` where `end` is the newest commit in
    the session and `start` the oldest. A gap larger than SESSION_GAP_HOURS
    between consecutive commits opens a new session."""
    dated = sorted(dated, key=lambda t: t[0], reverse=True)
    gap = timedelta(hours=SESSION_GAP_HOURS)
    sessions: list[dict] = []
    cur: dict | None = None
    prev: datetime | None = None
    for dt, added, removed in dated:
        if cur is None or prev - dt > gap:
            cur = {"start": dt, "end": dt, "added": added, "removed": removed}
            sessions.append(cur)
        else:
            cur["added"] += added
            cur["removed"] += removed
            cur["start"] = dt
        prev = dt
    return sessions


def _fmt_date(dt: datetime, now: datetime) -> str:
    """`Aug 4` for the current year, `Aug 4, 2025` otherwise."""
    if dt.year == now.year:
        return dt.strftime("%b ") + str(dt.day)
    return dt.strftime("%b ") + str(dt.day) + dt.strftime(", %Y")


def _relative(dt: datetime, now: datetime) -> str:
    """Coarse humanized age: today / yesterday / N days / weeks / months / years."""
    days = (now - dt).days
    if days <= 0:
        return "today"
    if days == 1:
        return "yesterday"
    if days < 7:
        return f"{days} days ago"
    if days < 14:
        return "last week"
    if days < 60:
        return f"{days // 7} weeks ago"
    if days < 365:
        months = max(1, round(days / 30))
        return "last month" if months == 1 else f"{months} months ago"
    years = max(1, round(days / 365))
    return "last year" if years == 1 else f"{years} years ago"


def _word_count(path: Path) -> int:
    """Total words of a note, frontmatter stripped."""
    text = path.read_text(encoding="utf-8", errors="ignore")
    if text.startswith("---"):
        end = text.find("\n---", 4)
        if end >= 0:
            text = text[end + 4:]
    return len(re.split(r"\s+", text.strip())) if text.strip() else 0


def _frontmatter_date(path: Path, field: str):
    """A `YYYY-MM-DD` frontmatter field as a `date`, or None.

    * `lastmod`     -- the site's authoritative "last real edit" date; tooling
                       commits (OG images, body restructuring) don't bump it, so it
                       is the ceiling for the popover (never show a change after it).
    * `createddate` -- the note's true creation date in the private vault, which
                       can predate the first *git* commit (= when it was published),
                       used to label that first row `published` instead of `new`."""
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return None
    if not text.startswith("---"):
        return None
    end = text.find("\n---", 4)
    front = text[: end if end >= 0 else len(text)]
    m = re.search(rf'^{field}:\s*"?(\d{{4}}-\d{{2}}-\d{{2}})', front, re.MULTILINE)
    if not m:
        return None
    try:
        return datetime.strptime(m.group(1), "%Y-%m-%d").date()
    except ValueError:
        return None


def build() -> dict[str, dict]:
    if not CONTENT.exists():
        print(f"recent_updates: {CONTENT} not found, writing empty index", file=sys.stderr)
        return {}

    now = datetime.now().astimezone()
    since = now - timedelta(days=LOOKBACK_DAYS)
    commits = commit_word_stats(since)
    first = first_commit_dates()

    # Fold in uncommitted `content/` changes as a "now" session so freshly
    # prepared notes (submodule not yet committed) still get badges/popovers.
    wt_stats, new_paths = worktree_word_stats(now)
    wt_paths = set(wt_stats)          # notes with uncommitted (live) edits
    for path, dated in wt_stats.items():
        commits.setdefault(path, []).extend(dated)
    for path in new_paths:
        first.setdefault(path, now)   # untracked note: its creation is "now"

    out: dict[str, dict] = {}
    for path, dated in commits.items():
        if not dated:
            continue
        sessions = group_sessions(dated)
        stem = Path(path).stem
        first_dt = first.get(path)
        full = CONTENT / path
        total_words = _word_count(full) if full.exists() else None

        # Ceiling the history at the note's `lastmod`: git can carry later
        # tooling/restructuring commits that never bumped lastmod, and those
        # would otherwise show as changes newer than the "Last updated" date.
        # A note with a live uncommitted edit is genuinely changing *now*, before
        # lastmod is bumped, so raise its ceiling to today or that session drops.
        # The creation session is always kept, though: a note must show when it
        # was created (and keep its NEW badge) even if a later commit that never
        # bumped lastmod folded into it -- otherwise the whole note disappears.
        lastmod = _frontmatter_date(full, "lastmod") if full.exists() else None
        if lastmod is not None:
            ceiling = max(lastmod, now.date()) if path in wt_paths else lastmod
            sessions = [
                s for s in sessions
                if s["end"].date() <= ceiling
                or (first_dt is not None and s["start"] <= first_dt <= s["end"])
            ]
        if not sessions:
            continue

        # If the note carries a `createddate`, its true creation is already shown
        # in the meta line, so the first *git* commit is the publish, not the
        # creation -- label that row "published" to avoid two dates both reading
        # as the origin (holds even when created and published fall on one day).
        # Only a note with no createddate (born straight on git) reads "new".
        created = _frontmatter_date(full, "createddate") if full.exists() else None
        creation_kind = "published" if created is not None else "new"

        def _is_creation(s: dict) -> bool:
            return first_dt is not None and s["start"] <= first_dt <= s["end"]

        # --- homepage badge ---
        # Use the most recent session that actually changed content (or is the
        # note's creation), so a trailing metadata-only commit doesn't make the
        # badge read "0 words".
        recent = next(
            (s for s in sessions if _is_creation(s) or s["added"] + s["removed"] > 0),
            sessions[0],
        )
        gross = recent["added"] + recent["removed"]
        is_new = _is_creation(recent)
        if is_new:
            entry: dict = {"status": "new", "words": total_words if total_words is not None else gross}
        else:
            entry = {"status": "updated", "words": gross}

        # --- per-note change popover (list of sessions) ---
        # The session that contains the note's first-ever commit is its creation:
        # show it as "new -> total words" (matching the badge) instead of churn,
        # so a fresh note doesn't read as a big +added/-removed edit.
        rows = []
        for s in sessions:
            is_creation = (
                first_dt is not None and s["start"] <= first_dt <= s["end"]
            )
            row = {
                "date": _fmt_date(s["end"], now),
                "iso": s["end"].strftime("%Y-%m-%d"),
                "rel": _relative(s["end"], now),
            }
            if is_creation:
                row["kind"] = creation_kind
                # size at creation/publish (words in the first commit), NOT the
                # current total -- a note published small and grown since would
                # otherwise read "published · <today's word count>".
                row["words"] = s["added"]
            elif s["added"] + s["removed"] > 0:
                row["added"] = s["added"]
                row["removed"] = s["removed"]
            else:
                continue  # zero-word non-creation session: skip as noise
            rows.append(row)
        rows = rows[:MAX_SESSIONS]
        if rows:
            entry["sessions"] = rows

        out[stem] = entry

    return out


def main() -> None:
    index = build()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    new = sum(1 for v in index.values() if v["status"] == "new")
    changes = sum(1 for v in index.values() if "sessions" in v)
    print(f"recent_updates: {len(index)} notes ({new} new, {len(index) - new} updated, "
          f"{changes} with popover history) -> {OUTPUT}")


if __name__ == "__main__":
    main()
