#!/usr/bin/env bash
# check-trailing-slash.sh — list internal links in the built site that point at a
# directory URL without the trailing slash (/about, /brain/foo, https://www.ssp.sh/ai).
# Every such link costs the visitor and every crawler a 301 (Ahrefs: "page has links
# to redirect"). Wikilinks get their slash in layouts/partials/textprocessing.html;
# what this finds is template/config/data links. Exit 1 when anything is found.
#   usage: helper-scripts/check-trailing-slash.sh [public]
set -euo pipefail
dir="${1:-public}"
# brain/ is built here but served from the second-brain-public deploy; uploads/ is static HTML.
hits=$(grep -rhoE 'href="(https?://(www\.)?ssp\.sh)?/[A-Za-z0-9_./-]*[A-Za-z0-9_-]"' "$dir" --include='*.html' --exclude-dir=uploads 2>/dev/null \
  | sed -E 's#https?://(www\.)?ssp\.sh##' \
  | grep -vE '\.[A-Za-z0-9]{1,12}"$' \
  | sort | uniq -c | sort -rn || true)
if [ -z "$hits" ]; then
  echo "trailing-slash check: OK — no internal links without trailing slash in $dir"
  exit 0
fi
echo "trailing-slash check: internal links without trailing slash (count, href) in $dir:"
echo "$hits" | head -40
echo
echo "Fix the source (layouts/, config.toml) — grep -rn 'href=\"<path>\"' layouts config.toml."
exit 1
