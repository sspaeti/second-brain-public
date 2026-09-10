// @license magnet:?xt=urn:btih:d3d9a9a6595521f9666a5e94cc830dab83b65699&dn=expat.txt
(function() {
  "use strict";
  
  // i18n support
  window.i18next = window.i18next || { t: function(n) { return n } };
  const t = window.i18next.t.bind(window.i18next);
  
  // Get configuration from data attributes
  function getAttribute(name, defaultValue) {
    return document.currentScript.getAttribute("data-" + name) || defaultValue;
  }
  
  // `hugo server` serves from localhost, so webmention.io would be asked for
  // mentions of http://localhost:1313/brain/<note>/ and always answer with
  // nothing. Rewrite local origins to production so comments are visible while
  // developing — the note's path is the same either way.
  const PROD_ORIGIN = "https://www.ssp.sh";
  function toProductionUrl(url) {
    return url.replace(/^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?/, PROD_ORIGIN);
  }

  // Configuration
  const pageUrl = toProductionUrl(getAttribute("page-url", window.location.href.replace(/#.*$/, "")));
  const additionalUrls = getAttribute("add-urls", undefined);
  const containerID = getAttribute("id", "webmentions");
  const wordCount = getAttribute("wordcount");
  const maxWebmentions = getAttribute("max-webmentions", 30);
  const preventSpoofingField = getAttribute("prevent-spoofing") ? "wm-source" : "url";
  const sortBy = getAttribute("sort-by", "published");
  const sortDir = getAttribute("sort-dir", "up");
  const commentsAreReactions = getAttribute("comments-are-reactions", false);
  const queryWwwRedirects = getAttribute("query-www-redirects", false);

  // Where "Join the conversation" points when this page has no Bluesky
  // webmention to link to. Hugo passes the same search URL the footer
  // "Discuss" link uses; the literal below is only a safety net for pages
  // rendered without the data attribute.
  const discussUrl = getAttribute(
    "discuss-url",
    "https://bsky.app/search?q=domain%3A%20" + pageUrl
  );

  // Owner's Bluesky DID — used to filter out self-webmentions (bridgy-fed
  // bridges your own bsky posts back as webmentions; we don't want to show
  // them as comments on our own articles).
  const OWN_BSKY_DID = "did:plc:edglm4muiyzty2snc55ysuqx";
  const OWN_BSKY_HANDLE = "ssp.sh";

  // Bridgy drops the `did:` prefix when it builds like URLs
  // (`...#liked_by_did:plc:xxx`), so matching on the full DID never fired on
  // reactions — our own likes on our own posts showed up as reactions. The
  // bare form is a substring of the full one, so it covers both shapes.
  const OWN_BSKY_DID_BARE = OWN_BSKY_DID.replace(/^did:/, "");

  // The Bluesky post announcing this page, baked in by utils/bsky_index.py.
  // Absent on pages that were never posted about.
  const bskyRkey = getAttribute("bsky-post", "");
  const bskyDid = getAttribute("bsky-did", "");

  // Extract the actor (DID or handle) from a bsky.app profile URL.
  function getBskyActor(url) {
    if (!url) return null;

    // A like is `.../profile/<post author>/post/<rkey>#liked_by_did:<liker>`,
    // so the handle in the path is *ours*, not the reactor's. Reading the path
    // credited every like that arrived without author data to the post owner.
    const liked = url.match(/#liked_by_did:(.+)$/);
    if (liked) {
      const did = decodeURIComponent(liked[1]);
      return did.startsWith("did:") ? did : "did:" + did;
    }

    const m = url.match(/bsky\.app\/profile\/([^/?#]+)/);
    return m ? decodeURIComponent(m[1]) : null;
  }

  function isOwnBskyPost(webmention) {
    const url = webmention[preventSpoofingField] || webmention.url || "";
    if (url.includes(OWN_BSKY_DID_BARE)) return true;

    // Bridgy reports replies as bsky.app/profile/<handle>/post/<rkey> — the DID
    // never appears, so the check above never actually matched our own replies
    // and they rendered as comments on our own notes.
    // Likes must be excluded from this rule: they arrive as
    // <our post URL>#liked_by_did:<liker>, so the handle in the path is ours
    // even though the reaction belongs to somebody else.
    const property = webmention["wm-property"];
    if (property !== "in-reply-to" && property !== "mention-of") return false;
    return url.includes("bsky.app/profile/" + OWN_BSKY_HANDLE + "/");
  }

  // Fetch a Bluesky profile from the public AppView (no auth required).
  async function fetchBskyProfile(actor) {
    try {
      const resp = await window.fetch(
        `https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor=${encodeURIComponent(actor)}`
      );
      if (!resp.ok) return null;
      return await resp.json();
    } catch (_) {
      return null;
    }
  }

  // Webmention.io returns empty author fields for bridgy-fed Bluesky entries;
  // backfill name/photo/url from the public Bluesky API in one batched pass.
  async function enrichBskyAuthors(webmentions) {
    const needs = webmentions.filter(function(wm) {
      const url = wm.url || "";
      if (!url.includes("bsky.app/profile/")) return false;
      const a = wm.author || {};
      return !a.name || !a.photo;
    });
    if (needs.length === 0) return;

    const actors = Array.from(new Set(
      needs.map(function(wm) { return getBskyActor(wm.url); }).filter(Boolean)
    ));

    const profiles = {};
    await Promise.all(actors.map(async function(actor) {
      const p = await fetchBskyProfile(actor);
      if (p) profiles[actor] = p;
    }));

    needs.forEach(function(wm) {
      const actor = getBskyActor(wm.url);
      const p = profiles[actor];
      if (!p) return;
      wm.author = wm.author || {};
      if (!wm.author.name) wm.author.name = p.displayName || p.handle || actor;
      if (!wm.author.photo) wm.author.photo = p.avatar || "";
      if (!wm.author.url) wm.author.url = "https://bsky.app/profile/" + (p.handle || actor);
    });
  }
  
  // One unauthenticated AppView call; null on any failure, since none of this
  // is essential to the page.
  async function bskyGet(method, params) {
    try {
      const query = new URLSearchParams(params).toString();
      const resp = await window.fetch(`https://public.api.bsky.app/xrpc/${method}?${query}`);
      if (!resp.ok) return null;
      return await resp.json();
    } catch (_) {
      return null;
    }
  }

  // Likes and replies straight from Bluesky for the post that announced this
  // page. brid.gy only reports posts whose link is in the text, so anything
  // posted with the URL in the embed card alone never produces a webmention —
  // this fills that gap. Both endpoints are public, so the counts are live at
  // page load rather than frozen at the last build.
  //
  // The results are shaped like webmention.io entries so the existing
  // renderers and removeDuplicates() work on them unchanged; in particular the
  // like URLs copy Bridgy's `#liked_by_did:` form, so a like reported by both
  // sources collapses into one.
  async function fetchBskyEngagement() {
    const empty = { comments: [], reactions: [] };
    if (!bskyRkey || !bskyDid) return empty;

    const atUri = `at://${bskyDid}/app.bsky.feed.post/${bskyRkey}`;
    const postUrl = `https://bsky.app/profile/${OWN_BSKY_HANDLE}/post/${bskyRkey}`;

    const [thread, likes] = await Promise.all([
      bskyGet("app.bsky.feed.getPostThread", { uri: atUri, depth: 1 }),
      bskyGet("app.bsky.feed.getLikes", { uri: atUri, limit: 100 })
    ]);

    const comments = [];
    (((thread || {}).thread || {}).replies || []).forEach(function(node) {
      const post = node.post;
      if (!post || !post.author) return;
      if (post.author.did === bskyDid) return; // our own replies in the thread
      const record = post.record || {};
      comments.push({
        "wm-property": "in-reply-to",
        url: `https://bsky.app/profile/${post.author.handle}/post/${(post.uri || "").split("/").pop()}`,
        published: record.createdAt || post.indexedAt || null,
        author: {
          name: post.author.displayName || post.author.handle,
          photo: post.author.avatar || "",
          url: `https://bsky.app/profile/${post.author.handle}`
        },
        content: { text: record.text || "" }
      });
    });

    const reactions = [];
    (((likes || {}).likes) || []).forEach(function(like) {
      const actor = like.actor;
      if (!actor || actor.did === bskyDid) return;
      reactions.push({
        "wm-property": "like-of",
        // Bridgy's exact shape — `did:` stripped — so the same like arriving
        // from both sources collapses in removeDuplicates().
        url: `${postUrl}#liked_by_did:${actor.did.replace(/^did:/, "")}`,
        published: like.createdAt || null,
        author: {
          name: actor.displayName || actor.handle,
          photo: actor.avatar || "",
          url: `https://bsky.app/profile/${actor.handle}`
        }
      });
    });

    return { comments, reactions };
  }

  // Translation mappings
  const propertyText = {
    "in-reply-to": t("replied"),
    "like-of": t("liked"),
    "repost-of": t("reposted"),
    "bookmark-of": t("bookmarked"),
    "mention-of": t("mentioned"),
    "rsvp": t("RSVPed"),
    "follow-of": t("followed")
  };
  
  // Emoji mappings
  const propertyEmoji = {
    "in-reply-to": "💬",
    "like-of": "❤️",
    "repost-of": "🔄",
    "bookmark-of": "⭐️",
    "mention-of": "💬",
    "rsvp": "📅",
    "follow-of": "🐜"
  };
  
  const rsvpEmoji = {
    "yes": "✅",
    "no": "❌",
    "interested": "💡",
    "maybe": "💭"
  };
  
  // Escape HTML
  function escapeHTML(text) {
    return text.replace(/[&<>"]/g, function(char) {
      return {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;"
      }[char] || char;
    });
  }
  
  // "Join the conversation on Bluesky" link — used by the comments block, the
  // reactions block, and the empty state, so it lives in one place.
  function renderJoinLink(url) {
    return `
      <a href="${url}" target="_blank" rel="noopener" class="join-conversation">
        <span>Join the conversation on Bluesky</span>
        <svg class="bluesky-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 -3.268 64 68.414" width="20" height="20">
          <path d="M13.873 3.805C21.21 9.332 29.103 20.537 32 26.55v15.882c0-.338-.13.044-.41.867-1.512 4.456-7.418 21.847-20.923 7.944-7.111-7.32-3.819-14.64 9.125-16.85-7.405 1.264-15.73-.825-18.014-9.015C1.12 23.022 0 8.51 0 6.55 0-3.268 8.579-.182 13.873 3.805zm36.254 0C42.79 9.332 34.897 20.537 32 26.55v15.882c0-.338.13.044.41.867 1.512 4.456 7.418 21.847 20.923 7.944 7.111-7.32 3.819-14.64-9.125-16.85 7.405 1.264 15.73-.825 18.014-9.015C62.88 23.022 64 8.51 64 6.55c0-9.818-8.578-6.732-13.873-2.745z" class="bluesky-icon-path"/>
        </svg>
      </a>
    `;
  }

  // Most notes have no webmentions, and the API can fail. In both cases the
  // block used to render nothing at all, leaving the page with no way to
  // reply — show the header and the Bluesky link anyway, minus the list.
  function renderEmptyState(container) {
    container.innerHTML = `
      <div class="webmentions-container">
        <div class="webmentions-header-container">
          <h3 class="webmentions-header">${t("Comments & Replies")}</h3>
          ${renderJoinLink(discussUrl)}
        </div>
        <p class="webmentions-empty">${t("No replies yet — start the conversation.")}</p>
      </div>
    `;
  }

  // Render a reaction (like, bookmark, etc.)
  function renderReaction(webmention) {
    // Clean up broken emoji characters in author names
    let author = "";
    if (webmention.author && webmention.author.name) {
      author = cleanUpEmojis(webmention.author.name);
    } else {
      author = webmention.url.split("/")[2];
    }
    
    const actionText = propertyText[webmention["wm-property"]] || t("reacted");
    
    let authorImg = "";
    if (webmention.author && webmention.author.photo) {
      authorImg = `
        <img
          src="${webmention.author.photo}"
          loading="lazy"
          decoding="async"
          alt="${author}"
        >
      `;
    } else {
      authorImg = `
        <img
          class="missing"
          src="data:image/webp;base64,UklGRkoCAABXRUJQVlA4TD4CAAAvP8APAIV0WduUOLr/m/iqY6SokDJSMD5xYX23SQizRsVdZmIj/f6goYUbiOj/BED7MOPReuBNT3vBesSzIex+SeqMFFkjebFmzH3S7POxDSJ1yaCbCmMnS2R46cRMPyQLw4GBK4esdK60pYwsZakecUCl5zsHv/5cPH08nx9/7i6rEEVCg2hR8VSd30PxMZpVoJZQO6Dixgg6X5oKFCmlVHIDmmMFShWumAXgCuyqVN8hHff/k+9fj8+ei7BVjpxBmZCUJv+6DhWGZwWvs+UoLHFCKsPYpfJtIcEXBTopEEsKwedZUv4ku1FZErKULLyQwFGgnmTs2vBD5qu44xwnG9uyjgrFOd+KRVlXyQfwQlauydaU6AVI7OjKXLUEqNtxJBmQegNDZgV7lxxqYMOMrDyC1NdAGbdiH9Ij0skjG+oTyfO0lmjdgvoH8iIgreuBMRYLSH+R3sAztXgL+XfS7E2bmfo6gnS0TrpnzHT7kL+skj7PgHuBwv/zpN8LDLQg7zfJZLBubMKnyeh6ZGyfDEfc2LYpnlUtG7JqsSHq1WoASbUS4KVaLwB8be5mfsGMDwBcm5VxbuxWxx3nkFanB6lYqsqSkOGkKicoDvXsneR7BkKU7DtaEuT7+pxBGVwx+9gVyqf2pVA9sC2CsmjZ1RJqEJHS4Tj/pCcS0JoyBYOsB91Xjh3OFfQPQhvCAYyeLJlaOoFp0XNNuD0BC8exr8uPx7D1JWkwFdZIXmD3MOPReuDNzHjBesSzIbQD"
          alt="${author}"
        >
      `;
    }
    
    let rsvpMark = "";
    if (webmention.rsvp && rsvpEmoji[webmention.rsvp]) {
      rsvpMark = `<sub>${rsvpEmoji[webmention.rsvp]}</sub>`;
    }
    
    return `
      <a
        class="reaction"
        rel="nofollow ugc"
        title="${author} ${actionText}"
        href="${webmention[preventSpoofingField]}"
      >
        ${authorImg}
        <span>${propertyEmoji[webmention["wm-property"]] || "💥"}</span>
        ${rsvpMark}
      </a>
    `;
  }
  
  // Truncate text to specified word count - preserving emoji characters but removing "???"
  function truncateText(text) {
    // Only escape HTML special characters, not emojis, and clean up broken emoji characters
    let result = escapeHTML(text);
    
    // Remove broken emoji characters (???)
    result = cleanUpEmojis(result);
    
    if (wordCount) {
      let words = result.replace(/\s+/g, " ").split(" ", parseInt(wordCount) + 1);
      if (words.length > parseInt(wordCount)) {
        words[parseInt(wordCount) - 1] += "&hellip;";
        words = words.slice(0, parseInt(wordCount));
        result = words.join(" ");
      }
    }
    
    return result;
  }
  
  // Get domain from URL
  function getDomain(url) {
    return url.substr(url.indexOf("//"));
  }
  
  // Remove duplicate webmentions based on URL
  function removeDuplicates(webmentions) {
    const result = [];
    const seen = {};
    
    webmentions.forEach(function(webmention) {
      const domain = getDomain(webmention.url);
      if (!seen[domain]) {
        result.push(webmention);
        seen[domain] = true;
      }
    });
    
    return result;
  }
  
  // Helper function to clean up text containing broken emoji characters
  function cleanUpEmojis(text) {
    // Handle null/undefined values
    if (!text) return "";
    
    // Replace the broken emoji characters (??? and also single ?️) with an empty string
    return text.replace(/\?\?\?/g, "").replace(/\?️/g, "").replace(/\?/g, "");
  }
  
  // Render a comment
  function renderComment(webmention) {
    const actionText = propertyText[webmention["wm-property"]] || t("reacted");
    
    let authorName = "";
    let sourceDomain = webmention[preventSpoofingField].split("/")[2];
    
    if (webmention.author && webmention.author.name) {
      // Remove broken emoji characters (???) from author names
      authorName = cleanUpEmojis(webmention.author.name);
    } else {
      authorName = webmention.url.split("/")[2];
    }
    
    // Skip your own comments if desired
    if (authorName.includes("Your Name")) {
      return "";
    }
    
    let authorLink = `<a class="webmention-author" rel="nofollow ugc" href="${webmention[preventSpoofingField]}">${authorName}</a>`;
    
    let sourceInfo = "";
    if (webmention.url.includes("news.ycombinator.com")) {
      sourceInfo = "via Hacker News";
      if (webmention.published) {
        sourceInfo += " (" + escapeHTML(webmention.published.split("T")[0]) + ")";
      }
    } else if (webmention.url.includes("reddit.com")) {
      sourceInfo = "via Reddit";
      if (webmention.published) {
        sourceInfo += " (" + escapeHTML(webmention.published.split("T")[0]) + ")";
      }
    } else {
      sourceInfo = sourceDomain !== authorName ? `at ${sourceDomain}` : "";
    }
    
    let publishedDate = "";
    if (webmention.published) {
      const date = new Date(webmention.published);
      publishedDate = `<time datetime="${webmention.published}">${date.toLocaleDateString()}</time>`;
    }
    
    let contentText = "";
    let contentType = "meta";
    
    if (webmention.content && webmention.content.text) {
      contentType = "content";
      // Clean up broken emoji characters in the content text
      contentText = truncateText(cleanUpEmojis(webmention.content.text));
    }
    
    // Custom render for avatar without emoji icon
    let authorImg = "";
    if (webmention.author && webmention.author.photo) {
      authorImg = `
        <div class="comment-avatar">
          <img
            src="${webmention.author.photo}"
            loading="lazy"
            decoding="async"
            alt="${authorName}"
          >
        </div>
      `;
    } else {
      authorImg = `
        <div class="comment-avatar">
          <img
            class="missing"
            src="data:image/webp;base64,UklGRkoCAABXRUJQVlA4TD4CAAAvP8APAIV0WduUOLr/m/iqY6SokDJSMD5xYX23SQizRsVdZmIj/f6goYUbiOj/BED7MOPReuBNT3vBesSzIex+SeqMFFkjebFmzH3S7POxDSJ1yaCbCmMnS2R46cRMPyQLw4GBK4esdK60pYwsZakecUCl5zsHv/5cPH08nx9/7i6rEEVCg2hR8VSd30PxMZpVoJZQO6Dixgg6X5oKFCmlVHIDmmMFShWumAXgCuyqVN8hHff/k+9fj8+ei7BVjpxBmZCUJv+6DhWGZwWvs+UoLHFCKsPYpfJtIcEXBTopEEsKwedZUv4ku1FZErKULLyQwFGgnmTs2vBD5qu44xwnG9uyjgrFOd+KRVlXyQfwQlauydaU6AVI7OjKXLUEqNtxJBmQegNDZgV7lxxqYMOMrDyC1NdAGbdiH9Ij0skjG+oTyfO0lmjdgvoH8iIgreuBMRYLSH+R3sAztXgL+XfS7E2bmfo6gnS0TrpnzHT7kL+skj7PgHuBwv/zpN8LDLQg7zfJZLBubMKnyeh6ZGyfDEfc2LYpnlUtG7JqsSHq1WoASbUS4KVaLwB8be5mfsGMDwBcm5VxbuxWxx3nkFanB6lYqsqSkOGkKicoDvXsneR7BkKU7DtaEuT7+pxBGVwx+9gVyqf2pVA9sC2CsmjZ1RJqEJHS4Tj/pCcS0JoyBYOsB91Xjh3OFfQPQhvCAYyeLJlaOoFp0XNNuD0BC8exr8uPx7D1JWkwFdZIXmD3MOPReuDNzHjBesSzIbQD"
            alt="${authorName}"
          >
        </div>
      `;
    }
    
    return `
      <li>
        <div class="webmention-comment">
          ${authorImg}
          <div class="comment-content">
            ${authorLink}
            <div class="webmention-meta">
              ${publishedDate}
              ${sourceInfo ? `<span>${sourceInfo}</span>` : ""}
            </div>
            ${contentText ? `<div class="webmention-content">${contentText}</div>` : ""}
          </div>
        </div>
      </li>
    `;
  }
  
  // Main function to fetch and render webmentions
  window.addEventListener("load", async function() {
    const container = document.getElementById(containerID);
    if (!container) return;
    
    // Collect target URLs
    const targetUrls = [getDomain(pageUrl)];
    if (additionalUrls) {
      additionalUrls.split("|").forEach(function(url) {
        targetUrls.push(getDomain(url));
      });
    }
    
    // Add www-less versions if needed
    if (queryWwwRedirects) {
      targetUrls.forEach(function(url) {
        if (url.indexOf("www.") > 0) {
          targetUrls.push(url.replace("www.", ""));
        }
      });
    }
    
    // Build API URL
    let apiUrl = `https://webmention.io/api/mentions.jf2?per-page=${maxWebmentions}&sort-by=${sortBy}&sort-dir=${sortDir}`;
    targetUrls.forEach(function(url) {
      apiUrl += `&target[]=${encodeURIComponent("http:" + url)}&target[]=${encodeURIComponent("https:" + url)}`;
    });
    
    // Start the Bluesky lookup alongside the webmention request; neither
    // depends on the other.
    const bskyPromise = fetchBskyEngagement();

    // Fetch webmentions
    let webmentionsData = {};
    try {
      const response = await window.fetch(apiUrl);
      if (response.status >= 200 && response.status < 300) {
        webmentionsData = await response.json();
      } else {
        console.error("Could not parse response");
        throw new Error(response.statusText);
      }
    } catch (error) {
      // Carry on with whatever Bluesky returns rather than dropping the block.
      console.error("Request failed", error);
      webmentionsData = { children: [] };
    }

    // Drop self-webmentions (bridgy-fed bridges our own bsky posts back).
    webmentionsData.children = (webmentionsData.children || []).filter(function(wm) {
      return !isOwnBskyPost(wm);
    });

    // Backfill author info for Bluesky entries that came back without it.
    await enrichBskyAuthors(webmentionsData.children);

    // Organize webmentions
    let reactions = [];
    let comments = [];
    
    if (commentsAreReactions) {
      reactions = comments;
    }
    
    const buckets = {
      "in-reply-to": comments,
      "like-of": reactions,
      "bookmark-of": reactions,
      "mention-of": comments
    };
    
    webmentionsData.children.forEach(function(webmention) {
      let bucket = buckets[webmention["wm-property"]];
      
      // Special handling for some platforms
      if (webmention.url.includes("news.ycombinator.com") || webmention.url.includes("reddit.com")) {
        bucket = comments;
      }
      
      if (bucket) {
        bucket.push(webmention);
      }
    });

    // Merge in the Bluesky-native engagement brid.gy never reported. Duplicates
    // with webmention.io collapse later in removeDuplicates(), which compares
    // full URLs — hence the matching URL shapes in fetchBskyEngagement().
    const bsky = await bskyPromise;
    comments.push(...bsky.comments);
    reactions.push(...bsky.reactions);

    // Render HTML
    let commentsHTML = "";
    let reactionsHTML = "";
    
    if (comments.length > 0 && comments !== reactions) {
      // Sort comments by published date (newest first)
      const sortedComments = [...comments].sort((a, b) => {
        const dateA = a.published ? new Date(a.published) : new Date(0);
        const dateB = b.published ? new Date(b.published) : new Date(0);
        return dateB - dateA;
      });
      
      const uniqueComments = removeDuplicates(sortedComments);

      commentsHTML = `
        <div class="webmentions-comments">
          <div class="webmentions-header-container">
            <h3 class="webmentions-header">${t("Comments & Replies")}</h3>
            ${renderJoinLink(discussUrl)}
          </div>
          <ul>${uniqueComments.map(renderComment).join("")}</ul>
        </div>
      `;
    }
    
    if (reactions.length > 0) {
      // Sort reactions by published date (newest first)
      const sortedReactions = [...reactions].sort((a, b) => {
        const dateA = a.published ? new Date(a.published) : new Date(0);
        const dateB = b.published ? new Date(b.published) : new Date(0);
        return dateB - dateA;
      });
      
      const uniqueReactions = removeDuplicates(sortedReactions);

      reactionsHTML = `
        <div>
          <div class="webmentions-header-container">
            <h3 class="webmentions-header">${t("Reactions")}</h3>
            ${comments.length === 0 || comments === reactions ? renderJoinLink(discussUrl) : ''}
          </div>
          <ul class="webmentions-list">${uniqueReactions.map(renderReaction).join("")}</ul>
        </div>
      `;
    }
    
    if (!commentsHTML && !reactionsHTML) {
      renderEmptyState(container);
      return;
    }

    container.innerHTML = `
      <div class="webmentions-container">
        ${reactionsHTML}
        ${commentsHTML}
      </div>
    `;
  });
})();
// @license-end