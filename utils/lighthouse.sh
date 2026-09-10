#!/usr/bin/env bash
# Local Lighthouse run that mirrors production delivery.
#
# `hugo server` (make run / make serve) sends every file uncompressed, so a
# Lighthouse run against localhost:1313 sees the 190 KB home page and the
# 130 KB stylesheet at full size and reports mobile FCP/LCP roughly 1.5 s
# worse than the CDN, which serves brotli/gzip. This builds the site with a
# local baseURL, serves it through utils/serve_gzip.py (gzip for text assets)
# and runs Lighthouse mobile + desktop against it.
#
# usage: utils/lighthouse.sh [path]      e.g. utils/lighthouse.sh apache-airflow/
# needs: chromium (CHROME_PATH), node/npx (lighthouse@latest via npx)
set -euo pipefail
PATH_UNDER_BRAIN="${1:-}"
PORT="${LH_PORT:-8790}"
OUT="${LH_OUT:-/tmp/brain-lighthouse}"
export CHROME_PATH="${CHROME_PATH:-/usr/bin/chromium}"
rm -rf "$OUT"; mkdir -p "$OUT/site/brain"
hugo --gc --minify -b "http://127.0.0.1:$PORT/brain/" -d "$OUT/site/brain" --quiet
python3 utils/serve_gzip.py "$OUT/site" "$PORT" &
SRV=$!
trap 'kill $SRV 2>/dev/null || true' EXIT
sleep 1
URL="http://127.0.0.1:$PORT/brain/$PATH_UNDER_BRAIN"
for mode in mobile desktop; do
  extra=""; [ "$mode" = desktop ] && extra="--preset=desktop"
  npx -y lighthouse@latest "$URL" $extra --output=json --output=html \
    --output-path="$OUT/$mode" --chrome-flags="--headless=new --no-sandbox" --quiet >/dev/null 2>&1 || true
  python3 - "$OUT/$mode.report.json" "$mode" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); a = r['audits']
cats = ' '.join(f"{k}={round((v['score'] or 0)*100)}" for k, v in r['categories'].items())
mets = ' '.join(f"{k}={a[k]['displayValue']}" for k in ['first-contentful-paint','largest-contentful-paint','total-blocking-time','cumulative-layout-shift','speed-index'])
print(f"{sys.argv[2]:8s} {cats}\n         {mets}")
PY
done
echo "reports: $OUT/mobile.report.html  $OUT/desktop.report.html"
