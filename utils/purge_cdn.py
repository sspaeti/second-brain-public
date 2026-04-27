#!/usr/bin/env python3
"""Purge Bunny CDN for notes modified since last purge.

Tracks last purge timestamp in .last_purge_time. Only purges notes
whose frontmatter `lastmod` is newer than that timestamp.
Handles rate limiting with automatic retry.
"""

import glob
import os
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import requests

CONTENT_DIR = Path(__file__).resolve().parent.parent / "content"
TIMESTAMP_FILE = Path(__file__).resolve().parent.parent / ".last_purge_time"
BASE_URL = "https://www.ssp.sh/brain"
PURGE_URL = "https://api.bunny.net/purge"
LASTMOD_RE = re.compile(r"^lastmod:\s*(.+)$", re.MULTILINE)


def get_last_purge_time() -> datetime:
    if TIMESTAMP_FILE.exists():
        ts = TIMESTAMP_FILE.read_text().strip()
        return datetime.fromisoformat(ts)
    # Fallback: start of today (purges all notes modified today)
    return datetime.now().replace(hour=0, minute=0, second=0, microsecond=0)


def save_purge_time(dt: datetime):
    TIMESTAMP_FILE.write_text(dt.isoformat())


def parse_lastmod(text: str) -> datetime | None:
    m = LASTMOD_RE.search(text)
    if not m:
        return None
    raw = m.group(1).strip()
    for fmt in ("%Y-%m-%d %H:%M:%S", "%Y-%m-%d"):
        try:
            return datetime.strptime(raw, fmt)
        except ValueError:
            continue
    return None


def slug_from_path(path: Path) -> str:
    name = path.stem.lower().replace(" ", "-")
    name = re.sub(r"[()]", "", name)
    return name


def purge_url(slug: str, api_key: str) -> bool:
    """Purge a slug (with and without trailing slash). Returns True on success."""
    headers = {"AccessKey": api_key}
    for url in [f"{BASE_URL}/{slug}", f"{BASE_URL}/{slug}/"]:
        while True:
            resp = requests.post(PURGE_URL, params={"url": url, "async": "true"}, headers=headers)
            if resp.status_code == 200:
                try:
                    data = resp.json()
                    if data.get("reason") == "rate_limited":
                        wait = data.get("retry_after_seconds", 2)
                        print(f"  Rate limited, waiting {wait}s...")
                        time.sleep(wait + 0.5)
                        continue
                except (ValueError, AttributeError):
                    pass
                break
            elif resp.status_code == 429:
                wait = int(resp.headers.get("Retry-After", 2))
                print(f"  Rate limited (429), waiting {wait}s...")
                time.sleep(wait + 0.5)
                continue
            else:
                print(f"  Warning: {resp.status_code} for {url}")
                break
    return True


def main():
    api_key = os.environ.get("BUNNY_API_KEY")
    if not api_key:
        print("BUNNY_API_KEY not set")
        sys.exit(1)

    last_purge = get_last_purge_time()
    purge_start = datetime.now()
    print(f"Last purge: {last_purge.isoformat()}")

    changed = []
    for md_file in sorted(CONTENT_DIR.glob("*.md")):
        text = md_file.read_text(errors="replace")
        lastmod = parse_lastmod(text)
        if lastmod and lastmod > last_purge:
            changed.append(md_file)

    if not changed:
        print("No notes changed since last purge.")
        save_purge_time(purge_start)
        return

    print(f"Found {len(changed)} note(s) to purge:")
    for f in changed:
        slug = slug_from_path(f)
        purge_url(slug, api_key)
        print(f"  Purged: ssp.sh/brain/{slug}")

    # Always purge the index page
    purge_url("", api_key)
    print("Purged brain index")

    save_purge_time(purge_start)
    print(f"Done. Updated .last_purge_time to {purge_start.isoformat()}")


if __name__ == "__main__":
    main()
