.DEFAULT_GOAL := serve

help: ## Show all Makefile targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

update: ## Update Quartz to the latest version on Github
	go install github.com/jackyzha0/hugo-obsidian@latest
	@git remote show upstream || (echo "remote 'upstream' not present, setting 'upstream'" && git remote add upstream https://github.com/jackyzha0/quartz.git)
	git fetch upstream
	git log --oneline --decorate --graph ..upstream/hugo
	git checkout -p upstream/hugo -- layouts .github Makefile assets/js assets/styles/base.scss assets/styles/darkmode.scss config.toml data

update-force: ## Forcefully pull all changes and don't ask to patch
	go install github.com/jackyzha0/hugo-obsidian@latest
	@git remote show upstream || (echo "remote 'upstream' not present, setting 'upstream'" && git remote add upstream https://github.com/jackyzha0/quartz.git)
	git fetch upstream
	git checkout upstream/hugo -- layouts .github Makefile assets/js assets/styles/base.scss assets/styles/darkmode.scss config.toml data

prepare-python: ## prepare commands
	find /home/sspaeti/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -not -path "*/_img/*" -delete
	python utils/find-publish-notes.py #copy all notes from my secondbrain with hashtag #publish to quartz
	rm -rf public
	hugo-obsidian -input=content -output=/home/sspaeti/git/sspaeti.com/second-brain-public/assets/indices -index=true -root=.
	python utils/lower_case.py #change linkIndex to lowercase for proper linking
	-$(MAKE) -C ../sspaeti-hugo-blog prepare #refresh blog indices so cross-edges are current
	obsidian-quartz enrich-with-blog #merge blog<->brain cross-edges into brain indices
	obsidian-quartz merge-search-index #build merged v2 search index, copy into blog
	-$(MAKE) -C ../../book/dedp link-index #refresh book linkIndex.js so book->brain edges are current
	obsidian-quartz enrich-with-book #merge book->brain incoming edges into brain indices

# run with Rust: build with `cargo build --release`
prepare: ## prepare commands
	find /home/sspaeti/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -not -path "*/_img/*" -delete
	obsidian-quartz #copy all notes from my secondbrain with hashtag #publish to /content
	cp static/second-brain.jpeg static/feature #the one in feature is used for _index note
	rm -rf public
	hugo-obsidian -input=content -output=/home/sspaeti/git/sspaeti.com/second-brain-public/assets/indices -index=true -root=.
	# obsidian-quartz convert_to_lower_case #change linkIndex to lowercase for proper linking
	python utils/lower_case.py #change linkIndex to lowercase for proper linking
	-$(MAKE) -C ../sspaeti-hugo-blog prepare #refresh blog indices so cross-edges are current
	obsidian-quartz enrich-with-blog #merge blog<->brain cross-edges into brain indices
	obsidian-quartz merge-search-index #build merged v2 search index, copy into blog
	-$(MAKE) -C ../../book/dedp link-index #refresh book linkIndex.js so book->brain edges are current
	obsidian-quartz enrich-with-book #merge book->brain incoming edges into brain indices
	obsidian-quartz enrich-with-memories #merge memory->brain edges into brain indices (graph only, after search merge)

word-count:
	find content -type f -not -path '*/\.*' -name '*.md' -exec cat {} \; | wc -w

file-count:
	find content -name "*.md" -type f | wc -l

# --- SecondBrain vault (private, whole vault not just published) ---
SB         := $(HOME)/Simon/SecondBrain
SB_SCRIPTS := $(SB)/💡 Resources/🧮 Runbooks/Scripts

all-sb: ## SecondBrain vault full stats (delegates to vault Scripts `make all`)
	@$(MAKE) -C "$(SB_SCRIPTS)" all


updated-this-year:
	@year=$$(date +%Y); \
	count=$$(grep -l "^lastmod: $$year" content/*.md | wc -l); \
	echo "$$count notes updated in $$year"

#Test Backlinks where probelm occured. Fixed with latest `hugo-obsidian`
# UPDATE 2025-04-23; Fixed with update on https://github.com/sspaeti/hugo-obsidian
#somehow the index is not correctly shown: 
# - open source projects engineeing project does not show backlink to poeple od data engineering...if make prepare-python is used, it works.......[
# - "Continuous Notes" only has one backlink, eventhoug in Obsidian it has many more (feedback loop, digital garden)
# - semantic layer does not show backlinks, only one, although there are 10 of them, as e.g. data virtualization is one of them.
# Workaround: If I rename the file in Obsidian, and rename it back to its origin. It will correctly create backlinks again. Not sure where the problem is. The linkindex.json seems to look good already with all paths included.

stop-brain: ## kill any running hugo server to prevent localhost URLs in production (when dedp book is running)
	-pkill -f "hugo server"

run: ## run hugo from a clean state
	hugo --gc && hugo server --enableGitInfo --minify

hugo-generate: ## generate hugo from clean but don't run
	rm -rf resources/_gen/ #helps prevent localhost:1313 in deployed website if accidentally an old hugo process running or from my book
	hugo --gc && hugo

purge-cdn: ## Purge Bunny CDN cache for ssp.sh/brain only (all brain pages)
	@if [ -z "$$BUNNY_API_KEY" ] || [ -z "$$BUNNY_ZONE_SSP" ]; then \
		echo "Skipping CDN purge (set BUNNY_API_KEY and BUNNY_ZONE_SSP to enable)"; \
	else \
		curl -s -X POST "https://api.bunny.net/purge?url=https://www.ssp.sh/brain/*&async=true" \
			-H "AccessKey: $$BUNNY_API_KEY" && \
		echo "CDN cache purged for ssp.sh/brain/*"; \
	fi

purge-cdn-changed: ## Purge only brain notes changed since last purge (tracks timestamp locally)
	python utils/purge_cdn.py

purge-cdn-today: ## Purge all brain notes updated today (legacy, date-only granularity)
	@if [ -z "$$BUNNY_API_KEY" ]; then \
		echo "BUNNY_API_KEY not set"; exit 1; \
	fi; \
	today=$$(date +%Y-%m-%d); \
	count=0; \
	grep -rl "^lastmod: $$today" content/*.md 2>/dev/null | while IFS= read -r file; do \
		slug=$$(basename "$$file" .md | tr '[:upper:]' '[:lower:]' | sed 's/ /-/g; s/[()]//g'); \
		curl -s -X POST "https://api.bunny.net/purge?url=https://www.ssp.sh/brain/$$slug&async=true" \
			-H "AccessKey: $$BUNNY_API_KEY"; \
		curl -s -X POST "https://api.bunny.net/purge?url=https://www.ssp.sh/brain/$$slug/&async=true" \
			-H "AccessKey: $$BUNNY_API_KEY"; \
		echo "Purged: ssp.sh/brain/$$slug"; \
		count=$$((count + 1)); \
	done; \
	total=$$(grep -rl "^lastmod: $$today" content/*.md 2>/dev/null | wc -l); \
	curl -s -X POST "https://api.bunny.net/purge?url=https://www.ssp.sh/brain/&async=true" \
		-H "AccessKey: $$BUNNY_API_KEY"; \
	echo "Purged brain index + $$total notes updated $$today"

upload: ## upload to server (preserves old hashed /js/, /styles/, /indices/ and root styles.*.min.css so stale CDN/browser HTML keeps working)
	rsync -avz --delete \
		--exclude='/_img/' \
		--filter='P /js/***' \
		--filter='P /styles/***' \
		--filter='P /indices/***' \
		--filter='P /styles.*.min.css' \
		public/ sspaeti@sspaeti.com:~/www/ssp/brain
	rsync -av --delete --size-only \
		--skip-compress=jpg,jpeg,png,gif,webp,avif,ico,woff,woff2 \
		public/_img/ sspaeti@sspaeti.com:~/www/ssp/brain/_img

upload-clean: ## upload with full delete (removes orphaned hashed assets — pair with `make purge-cdn` or stale HTML will 404)
	rsync -avz --delete public/ sspaeti@sspaeti.com:~/www/ssp/brain

compress-gallery: ## Compress gallery images in-place (run once; only downsizes, never upscales)
	find content/_img/todays-office -type f \( -iname "*.jpg" -o -iname "*.jpeg" \) \
	  -exec magick {} -resize 1920x1920\> -quality 78 -strip {} \;

serve: prepare run
serve-old: prepare-python run


upload-only: hugo-generate upload
deploy: stop-brain prepare hugo-generate upload purge-cdn-changed
deploy-clean: stop-brain prepare hugo-generate upload-clean purge-cdn ## deploy and remove orphaned hashed assets (full purge required — may briefly break stale HTML until purge propagates)
