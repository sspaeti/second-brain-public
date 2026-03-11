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

# run with Rust: build with `cargo build --release`
prepare: ## prepare commands
	find /home/sspaeti/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -not -path "*/_img/*" -delete
	obsidian-quartz #copy all notes from my secondbrain with hashtag #publish to /content
	cp static/second-brain.jpeg static/feature #the one in feature is used for _index note
	rm -rf public
	hugo-obsidian -input=content -output=/home/sspaeti/git/sspaeti.com/second-brain-public/assets/indices -index=true -root=. 
	# obsidian-quartz convert_to_lower_case #change linkIndex to lowercase for proper linking
	python utils/lower_case.py #change linkIndex to lowercase for proper linking

word-count:
	find content -type f -not -path '*/\.*' -name '*.md' -exec cat {} \; | wc -w

file-count:
	find content -name "*.md" -type f | wc -l

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

upload: ## upload to server 
	rsync -avz --delete public/ sspaeti@sspaeti.com:~/www/ssp/brain

serve: prepare run
serve-old: prepare-python run


upload-only: hugo-generate upload
deploy: stop-brain prepare hugo-generate upload
