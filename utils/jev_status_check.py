#!/usr/bin/env python3
"""One-off: ask Jev for a level per public note, list where it disagrees with
data/note_status.json. Throwaway; not part of make. Run from anywhere.

  python3 jev_status_check.py            # first 20 notes, prints token usage
  python3 jev_status_check.py --all      # every note
"""
import json, os, sys, time, urllib.request, urllib.error, concurrent.futures as cf
from pathlib import Path

ROOT = Path("/home/sspaeti/git/sspaeti.com/second-brain-public")
KEY = os.environ.get("JEV_API_KEY") or os.environ.get("JEV_TYPESAVE_AI")
ENDPOINTS = [
    os.environ.get("JEV_ENDPOINT") or "https://jevtypesafeai.com/api/v1/decide",
    "https://api.typesafe.ai/v1/systemone",
]
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

ENDPOINT = ENDPOINTS[0]

def post(endpoint: str, data: bytes):
    r = urllib.request.Request(endpoint, data=data, headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    with urllib.request.urlopen(r, timeout=60) as resp:
        return json.load(resp)

def ask(stem: str, text: str) -> dict:
    global ENDPOINT
    req = {
        "state": f"Note title: {stem}\n\n{text}",
        "questions": {
            "level": {"type": "choice", "instructions": "How mature is this public note for a reader?", "criteria": SENTENCES},
            "polish": {"type": "score", "instructions": "How polished is the writing?",
                       "criteria": ["scratch/bullets", "rough prose", "readable", "clean prose", "publication quality"]},
        },
    }
    data = json.dumps(req).encode()
    for attempt in range(3):
        try:
            return post(ENDPOINT, data)
        except urllib.error.HTTPError as e:
            if e.code in (401, 403) and ENDPOINT == ENDPOINTS[0]:
                ENDPOINT = ENDPOINTS[1]   # key is for the official API, not the proxy
                continue
            if attempt == 2:
                return {"error": f"HTTP {e.code}: {e.read()[:200]!r}"}
        except Exception as e:  # noqa: BLE001
            if attempt == 2:
                return {"error": str(e)}
        time.sleep(2 * (attempt + 1))
    return {"error": "gave up"}

def main() -> None:
    if not KEY:
        sys.exit("set JEV_API_KEY (or JEV_TYPESAVE_AI)")
    status = json.loads((ROOT / "data/note_status.json").read_text())
    stems = sorted(status)
    if "--all" not in sys.argv:
        stems = stems[:20]
    # one sequential probe first so the endpoint fallback settles before the pool starts
    first = ask(stems[0], body(ROOT / "content" / f"{stems[0]}.md"))
    if "error" in first:
        sys.exit(f"probe failed: {first['error']}")
    rows, tokens, cost = [], 0, 0.0
    def handle(s, res):
        nonlocal tokens, cost
        if "error" in res:
            rows.append((s, status[s]["status"], "error", 0.0, 0.0, res["error"])); return
        a = res["answers"]; u = res.get("usage", {})
        tokens += u.get("input_tokens", 0); cost += u.get("cost_usd", 0.0)
        rows.append((s, status[s]["status"], a["level"]["choice"], a["level"].get("confidence", 0.0), a["polish"].get("score", 0.0), status[s]["reason"]))
    handle(stems[0], first)
    with cf.ThreadPoolExecutor(max_workers=4) as pool:
        futs = {pool.submit(ask, s, body(ROOT / "content" / f"{s}.md")): s for s in stems[1:]}
        for fut in cf.as_completed(futs):
            handle(futs[fut], fut.result())
    rows.sort(key=lambda r: (-(r[3] if r[1] != r[2] else -1), r[0]))
    dis = [r for r in rows if r[1] != r[2] and r[3] >= MIN_CONF]
    lines = [f"# Jev vs heuristic · {len(rows)} notes · {len(dis)} disagreements at confidence >= {MIN_CONF}",
             f"input tokens {tokens:,} · cost ${cost:.2f} · endpoint {ENDPOINT}", "",
             "| note | heuristic | jev | conf | polish | heuristic reason |", "|---|---|---|---|---|---|"]
    for s, h, j, c, p, reason in dis:
        lines.append(f"| {s} | {h} | {j} | {c:.2f} | {p:.1f} | {reason} |")
    lines += ["", "## All notes", "", "| note | heuristic | jev | conf | polish |", "|---|---|---|---|---|"]
    for s, h, j, c, p, _ in rows:
        lines.append(f"| {s} | {h} | {j} | {c:.2f} | {p:.1f} |")
    OUT.write_text("\n".join(lines) + "\n")
    errors = sum(1 for r in rows if r[2] == "error")
    print(f"{len(rows)} notes, {len(dis)} disagreements, {errors} errors, {tokens:,} input tokens, ${cost:.2f} -> {OUT}")

if __name__ == "__main__":
    main()
