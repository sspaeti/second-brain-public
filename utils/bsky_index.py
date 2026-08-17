"""Map ssp.sh URLs to the Bluesky post that announced them.

One output, `data/bsky_posts.json`, from a walk of the account's author feed:

    {
      "meta":  {"did": "did:plc:...", "handle": "ssp.sh", "generated": "..."},
      "paths": {"brain/listmonk": {"rkey": "3lua...", "url": "https://bsky.app/..."}}
    }

Why this exists: brid.gy only sends a webmention when the link sits in the post
*text* (a link facet). Posts that carry the URL only in the embed card — which
is what you get when you paste a link, let Bluesky build the preview, then
delete the URL from the text — are invisible to it, so their likes and replies
never reach webmention.io. Roughly half of the posts in the feed are that shape.

This script does not fetch engagement, only identity: which post belongs to
which page. `static/webmention.js` takes the rkey from here and asks the public
Bluesky API for likes and replies at page load, so the counts stay live between
deploys. Only a post published *after* the last build is missed.

Both the embed URI and any link facets are considered, so posts of either shape
are indexed. When a page was announced more than once, the newest post wins
(the feed is returned newest-first).

Network failure is not fatal: any existing `data/bsky_posts.json` is left in
place and the build continues without fresh data. Stdlib only, like the other
utils here, so `make prepare` stays dependency-free.
"""

import json
import sys
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "data" / "bsky_posts.json"

HANDLE = "ssp.sh"
SITE_HOSTS = {"ssp.sh", "www.ssp.sh"}
API = "https://public.api.bsky.app/xrpc"
PAGE_SIZE = 100
MAX_PAGES = 40  # 4000 posts; a backstop, not an expected limit
TIMEOUT = 20


def _get(method: str, **params) -> dict:
    """One unauthenticated AppView call. Raises on transport/HTTP error."""
    url = f"{API}/{method}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"User-Agent": "ssp.sh-bsky-index"})
    with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
        return json.load(resp)


def _page_key(url: str) -> str | None:
    """Normalise an ssp.sh URL to the key the Hugo template looks up.

    `https://ssp.sh/brain/listmonk` and `https://www.ssp.sh/brain/listmonk/`
    both become `brain/listmonk`, matching `strings.Trim .RelPermalink "/"`.
    """
    try:
        parts = urllib.parse.urlsplit(url)
    except ValueError:
        return None
    if parts.hostname is None or parts.hostname.lower() not in SITE_HOSTS:
        return None
    key = parts.path.strip("/").lower()
    return key or None


def _post_urls(record: dict) -> list[str]:
    """Every outbound link in a post: the embed card plus any link facets."""
    urls = []

    embed = record.get("embed") or {}
    external = embed.get("external") or {}
    if external.get("uri"):
        urls.append(external["uri"])

    for facet in record.get("facets") or []:
        for feature in facet.get("features") or []:
            if feature.get("$type", "").endswith("#link") and feature.get("uri"):
                urls.append(feature["uri"])

    return urls


def _walk_feed(did: str):
    """Yield the account's own top-level posts, newest first."""
    cursor = None
    for _ in range(MAX_PAGES):
        params = {"actor": did, "limit": PAGE_SIZE, "filter": "posts_no_replies"}
        if cursor:
            params["cursor"] = cursor
        data = _get("app.bsky.feed.getAuthorFeed", **params)

        for item in data.get("feed") or []:
            # Skip reposts of other people — the announcement has to be ours.
            if item.get("reason"):
                continue
            post = item.get("post") or {}
            if (post.get("author") or {}).get("did") != did:
                continue
            yield post

        cursor = data.get("cursor")
        if not cursor:
            return


def build_index() -> dict:
    did = _get("com.atproto.identity.resolveHandle", handle=HANDLE)["did"]

    paths: dict[str, dict] = {}
    for post in _walk_feed(did):
        rkey = (post.get("uri") or "").rsplit("/", 1)[-1]
        if not rkey:
            continue
        for url in _post_urls(post.get("record") or {}):
            key = _page_key(url)
            # First hit wins: the feed is newest-first, so a page announced
            # twice keeps its most recent post.
            if key and key not in paths:
                paths[key] = {
                    "rkey": rkey,
                    "url": f"https://bsky.app/profile/{HANDLE}/post/{rkey}",
                }

    return {
        "meta": {
            "did": did,
            "handle": HANDLE,
            "generated": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        },
        "paths": dict(sorted(paths.items())),
    }


def main() -> int:
    try:
        index = build_index()
    except (urllib.error.URLError, OSError, KeyError, ValueError) as err:
        # Never fail the build over a third-party API. Yesterday's mapping is
        # still correct for every page except ones announced since.
        kept = "kept existing" if OUTPUT.exists() else "no data file"
        print(f"bsky_index: skipped ({err}); {kept}", file=sys.stderr)
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n")
    print(f"bsky_index: {len(index['paths'])} pages -> {OUTPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
