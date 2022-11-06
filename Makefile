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

prepare: ## prepare commands
	find /Users/sspaeti/Documents/git/sspaeti.com/second-brain-public/content -type f -not -name ".git" -delete
	python utils/find-publish-notes.py #copy all notes from my secondbrain with hashtag #publish to quartz
	rm -r public
	hugo-obsidian -input=content -output=assets/indices -index -root=. 
	python utils/lower_case.py #change linkIndex to lowercase for proper linking

run: ## run hugo from a clean state
	hugo --gc && hugo server --enableGitInfo --minify

hugo-generate: ## generate hugo from clean but don't run
	hugo --gc && hugo

upload: ## upload to server 
	rsync -avz --delete public/ sspaeti@sspaeti.com:~/www/brain

serve: prepare run

deploy: prepare hugo-generate upload
