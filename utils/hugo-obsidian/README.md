# Obsidian Link Scraper
Used by [Quartz](https://github.com/jackyzha0/quartz/hugo) -> My latest Quartz v3+my addtions are on [sspaeti/second-brain-public](https://github.com/sspaeti/second-brain-public).

This repository comes to you in two parts.

1. GitHub Action (scrapes links into a `.json` file)
2. Hugo Partial (turns `.json` file into graphs and tables)

## GitHub Action
GitHub action and binary to scrape [Obsidian](http://obsidian.md/) vault for links and exposes them as a `.json` file for easy consumption by [Hugo](https://gohugo.io/).
### Example Usage (Binary)
Read Markdown from the `/content` folder and place the resulting `linkIndex.json` (and `contentIndex.yaml` if the `index` flag is enabled) into `/data`

```shell
# Installation
go install github.com/sspaeti/hugo-obsidian@latest

# Run
hugo-obsidian -input=content -output=data -index=true
```

### Example Usage (GitHub Action)

Add 'Build Link Index' as a build step in your workflow file (e.g. `.github/workflows/deploy.yaml`)
```yaml
...

jobs:
  deploy:
    runs-on: ubuntu-18.04
    steps:
      - uses: actions/checkout@v2
      - name: Build Link Index
        uses: sspaeti/hugo-obsidian@latest
        with:
          input: content # input folder
          output: data   # output folder
          index: true    # whether to index content
      ...
```


## Changelog

### 2026-06-05

- Added enrichment of Blogs and Book articles, see
    - [enrich_with_blog.rs](https://github.com/sspaeti/second-brain-public/blob/hugo/utils/obsidian-quartz/src/enrich_with_blog.rs)
    - [enrich_with_book.rs](https://github.com/sspaeti/second-brain-public/blob/hugo/utils/obsidian-quartz/src/enrich_with_book.rs)

### 2025-04-23

  1. Fixed case sensitivity: Both processTarget and processSource now convert links to lowercase, ensuring that capitalization differences (like "Semantic Layer" vs "semantic layer") don't cause missed links.
  2. Improved block reference handling: The code now properly extracts the base path from links that include block references (like [[Semantic Layer^SOMEHASH]]), ensuring the link is properly registered.
  3. Added special character handling: Added specific handling for titles with slashes, replacing " / " with "-" to ensure consistency between file paths and link references.
  4. Optimized for large vaults: Pre-allocated maps with larger capacities and simplified the mapping logic to better handle large numbers of links.

