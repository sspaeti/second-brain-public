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
from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTENT = ROOT / "content"
OUTPUT = ROOT / "data" / "recent_updates.json"

LOOKBACK_DAYS = 1825 #5 years          # window for word-diff scan (popover history reaches ~1y back)
SESSION_GAP_HOURS = 24       # commits closer than this fold into one session
MAX_SESSIONS = 7             # sessions shown in a note's change popover


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


def commit_word_stats(since: datetime) -> dict[str, list[tuple[datetime, int, int]]]:
    """Per note, a list of `(commit_datetime, added_words, removed_words)` for
    commits since `since`, counted from `+`/`-` diff lines. Frontmatter lines are
    excluded (tracked by file line number) so metadata-only commits don't read as
    content edits."""
    result = subprocess.run(
        [
            "git", "-c", "core.quotePath=false", "-C", str(CONTENT), "log",
            f"--since={since.strftime('%Y-%m-%d %H:%M:%S')}",
            "-p", "--format=__COMMIT__%ai", "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    )

    commits: dict[str, list[tuple[datetime, int, int]]] = defaultdict(list)
    cur_date: datetime | None = None
    cur_path: str | None = None
    cur_added = 0
    cur_removed = 0
    is_binary = False
    fm_end = 0                 # last frontmatter line for the current file
    old_ln = new_ln = 0        # running line numbers within the current hunk

    def flush() -> None:
        nonlocal cur_path, cur_added, cur_removed
        if cur_path is not None and cur_date is not None:
            commits[cur_path].append((cur_date, cur_added, cur_removed))
        cur_path, cur_added, cur_removed = None, 0, 0

    for line in result.stdout.splitlines():
        if line.startswith("__COMMIT__"):
            flush()
            cur_date = _parse_git_date(line[len("__COMMIT__"):])
            continue
        if line.startswith("diff --git "):
            flush()
            _, _, b_path = line.partition(" b/")
            cur_path = b_path if b_path.endswith(".md") else None
            is_binary = False
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
            continue
        if old_ln == 0:        # still in the diff preamble (index / ---/+++ headers)
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


_LASTMOD_RE = re.compile(r'^lastmod:\s*"?(\d{4}-\d{2}-\d{2})', re.MULTILINE)


def _frontmatter_lastmod(path: Path):
    """The note's frontmatter `lastmod` as a `date`, or None. This is the site's
    authoritative "last real edit" date -- tooling/metadata commits (OG images,
    body restructuring) often don't bump it -- so it's the ceiling for the popover
    history: we never show a change dated after it."""
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return None
    if not text.startswith("---"):
        return None
    end = text.find("\n---", 4)
    front = text[: end if end >= 0 else len(text)]
    m = _LASTMOD_RE.search(front)
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
        lastmod = _frontmatter_lastmod(full) if full.exists() else None
        if lastmod is not None:
            sessions = [s for s in sessions if s["end"].date() <= lastmod]
        if not sessions:
            continue

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
                row["kind"] = "new"
                row["words"] = total_words if total_words is not None else s["added"]
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
