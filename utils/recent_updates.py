"""Emit per-note change badges for the homepage recent-notes list.

Reads the `content/` git history and, for every markdown note, computes its most
recent *editing session* — the run of commits ending at the note's last commit,
grouping commits whose gap is <= SESSION_GAP_HOURS into one session. This
coalesces "I fixed it three times that afternoon" into a single number that
matches the single date Hugo shows.

Output: `data/recent_updates.json`, keyed by the on-disk filename stem (which
equals Hugo's `.File.BaseFileName`), each value `{"status": ..., "words": N}`:

  * status "new"     -> note created in that session; words = total note words.
  * status "updated" -> words = gross words touched (added + deleted) in the
                        session's diffs.

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

LOOKBACK_DAYS = 180          # window for word-diff scan; covers the 30 recent notes
SESSION_GAP_HOURS = 24       # commits closer than this fold into one session


def _parse_git_date(raw: str) -> datetime:
    """git `%ai` looks like `2026-08-05 14:23:01 +0200`."""
    return datetime.strptime(raw.strip(), "%Y-%m-%d %H:%M:%S %z")


def commit_word_stats(since: datetime) -> dict[str, list[tuple[datetime, int]]]:
    """Per note, a list of `(commit_datetime, gross_words)` for commits since
    `since`, where gross = added + deleted words on the note's diff lines."""
    result = subprocess.run(
        [
            "git", "-c", "core.quotePath=false", "-C", str(CONTENT), "log",
            f"--since={since.strftime('%Y-%m-%d %H:%M:%S')}",
            "-p", "--format=__COMMIT__%ai", "--", "*.md",
        ],
        capture_output=True, text=True, check=True,
    )

    commits: dict[str, list[tuple[datetime, int]]] = defaultdict(list)
    cur_date: datetime | None = None
    cur_path: str | None = None
    cur_gross = 0
    is_binary = False

    def flush() -> None:
        nonlocal cur_path, cur_gross
        if cur_path is not None and cur_date is not None:
            commits[cur_path].append((cur_date, cur_gross))
        cur_path, cur_gross = None, 0

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
            continue
        if cur_path is None:
            continue
        if line.startswith("Binary files"):
            is_binary = True
            continue
        if is_binary or line.startswith(("+++", "---", "@@")):
            continue
        if line.startswith("+"):
            cur_gross += len(line[1:].split())
        elif line.startswith("-"):
            cur_gross += len(line[1:].split())
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


def most_recent_session(dated: list[tuple[datetime, int]]) -> tuple[datetime, int]:
    """From a note's commits, return `(session_start, gross_words)` for the most
    recent session: newest commit plus every older commit within SESSION_GAP_HOURS
    of the previous kept one, until the first larger gap."""
    dated = sorted(dated, key=lambda t: t[0], reverse=True)
    gap = timedelta(hours=SESSION_GAP_HOURS)
    session_start = dated[0][0]
    gross = dated[0][1]
    prev = dated[0][0]
    for dt, g in dated[1:]:
        if prev - dt > gap:
            break
        gross += g
        session_start = dt
        prev = dt
    return session_start, gross


def _word_count(path: Path) -> int:
    """Total words of a note, frontmatter stripped."""
    text = path.read_text(encoding="utf-8", errors="ignore")
    if text.startswith("---"):
        end = text.find("\n---", 4)
        if end >= 0:
            text = text[end + 4:]
    return len(re.split(r"\s+", text.strip())) if text.strip() else 0


def build() -> dict[str, dict]:
    if not CONTENT.exists():
        print(f"recent_updates: {CONTENT} not found, writing empty index", file=sys.stderr)
        return {}

    since = datetime.now().astimezone() - timedelta(days=LOOKBACK_DAYS)
    commits = commit_word_stats(since)
    first = first_commit_dates()

    out: dict[str, dict] = {}
    for path, dated in commits.items():
        if not dated:
            continue
        session_start, gross = most_recent_session(dated)
        stem = Path(path).stem
        first_dt = first.get(path)
        is_new = first_dt is not None and first_dt >= session_start
        if is_new:
            full = CONTENT / path
            words = _word_count(full) if full.exists() else gross
            out[stem] = {"status": "new", "words": words}
        else:
            out[stem] = {"status": "updated", "words": gross}
    return out


def main() -> None:
    index = build()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    new = sum(1 for v in index.values() if v["status"] == "new")
    print(f"recent_updates: {len(index)} notes ({new} new, {len(index) - new} updated) -> {OUTPUT}")


if __name__ == "__main__":
    main()
