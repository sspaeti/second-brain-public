# My Public Second Brain.

See on [ssp.sh/brain](https://ssp.sh/brain).

This is a fork of the [Quartz](https://github.com/jackyzha0/quartz) repo ([v3](https://github.com/jackyzha0/quartz/tree/hugo) with Hugo). I added some additional features such as:
* tagging with #publish will automatically copy the note from my private second brain in [Obsidian](https://obsidian.md) to this public second brain. You find the python scrips in [utils](utils). 
* It also converts the first header (`# my title`) into the frontmatter (metadata of each note) and removes it so that Quartz needs it.

The content/notes itself are not published in this repo, only on [ssp.sh/brain](https://ssp.sh/brain).

> **[Explore with RAG → explore.ssp.sh](https://explore.ssp.sh)**
>
> Semantic search, hidden connections, and graph traversal powered by [obsidian-note-taking-assistant](https://github.com/sspaeti/obsidian-note-taking-assistant).

## Utils used
## Backlink and graph creation
The tool used is `hugo-obsidian`, a small go programm written by Jacky. Here's the [source](https://github.com/jackyzha0/hugo-obsidian). But it is not maintained anymore (as there is now a [v4](https://github.com/jackyzha0/quartz/tree/v4) without it) and it had bugs and didn't show all my backlinks. That's why I forked it and fixed the backlinks. You can find it here: [sspaeti/hugo-obsidian](https://github.com/sspaeti/hugo-obsidian).

### Move file from my Obsidian to this Quartz folder

I use the utils in [obsidian-quartz](./utils/obsidian-quartz/README.md) script, written in Rust.

## Configs
### Redirects of renamed files
Find these in [.htaccess](static/.htaccess)



## ChangeLog

### 2025-10-16: Fix cross-section navigation (brain ↔ blog)
- Modified [`assets/js/router.js`](assets/js/router.js) to intercept clicks between `/brain/` and other sections, forcing full page loads instead of SPA navigation
- Prevents "null" page errors when navigating from brain to main blog
- Compatible with [`assets/js/external-links.js`](assets/js/external-links.js) which handles truly external links
