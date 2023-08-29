.DEFAULT_GOAL := serve


serve: ## generate hugo from clean but don't run
	npx quartz build --serve

