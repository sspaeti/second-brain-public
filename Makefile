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
	find /Users/sspaeti/Documents/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -delete
	python utils/find-publish-notes.py #copy all notes from my secondbrain with hashtag #publish to quartz
	rm -rf public
	hugo-obsidian -input=content -output=/Users/sspaeti/Documents/git/sspaeti.com/second-brain-public/assets/indices -index=true -root=. 
	python utils/lower_case.py #change linkIndex to lowercase for proper linking

# run with Rust: build with `cargo build --release`
prepare: ## prepare commands
	find /Users/sspaeti/Documents/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -delete
	obsidian-quartz #copy all notes from my secondbrain with hashtag #publish to /content
	rm -rf public
	hugo-obsidian -input=content -output=/Users/sspaeti/Documents/git/sspaeti.com/second-brain-public/assets/indices -index=true -root=. 
	# obsidian-quartz convert_to_lower_case #change linkIndex to lowercase for proper linking
	python utils/lower_case.py #change linkIndex to lowercase for proper linking
	# TODO: somehow the index is not correctly shown. E.g open source projects engineeing project does not show backlink to poeple od data engineering...if make prepare-python is used, it works.......[
	# or "Continuous Notes" only has one backlink, eventhoug in Obsidian it has many more, even public ones
	# # Workaround: If I rename the file in Obsidian, and rename it back to its origin. It will correctly create backlinks again. Not sure where the problem is. The linkindex.json seems to look good already with all paths included.

run: ## run hugo from a clean state
	hugo --gc && hugo server --enableGitInfo --minify

hugo-generate: ## generate hugo from clean but don't run
	hugo --gc && hugo

upload: ## upload to server 
	rsync -avz --delete public/ sspaeti@sspaeti.com:~/www/ssp/brain

serve: prepare run
serve-old: prepare-python run

upload-only: hugo-generate upload
deploy: prepare hugo-generate upload
