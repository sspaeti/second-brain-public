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
  
  // Configuration
  const pageUrl = getAttribute("page-url", window.location.href.replace(/#.*$/, ""));
  const additionalUrls = getAttribute("add-urls", undefined);
  const containerID = getAttribute("id", "webmentions");
  const wordCount = getAttribute("wordcount");
  const maxWebmentions = getAttribute("max-webmentions", 30);
  const preventSpoofingField = getAttribute("prevent-spoofing") ? "wm-source" : "url";
  const sortBy = getAttribute("sort-by", "published");
  const sortDir = getAttribute("sort-dir", "up");
  const commentsAreReactions = getAttribute("comments-are-reactions", false);
  const queryWwwRedirects = getAttribute("query-www-redirects", false);
  
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
      console.error("Request failed", error);
      return;
    }
    
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
      
      // Find the most recent comment URL to link to for "Join the conversation"
      let conversationUrl = "https://bsky.app/profile/ssp.sh";
      
      // Try to find a comment with a URL to Bluesky
      if (uniqueComments.length > 0) {
        // Find the first comment from Bluesky (should already be sorted newest first)
        const blueskyComment = uniqueComments.find(comment => 
          comment.url && (comment.url.includes("bsky.app") || comment[preventSpoofingField].includes("bsky.app"))
        );
        
        // Use the URL from that comment if found
        if (blueskyComment) {
          conversationUrl = blueskyComment[preventSpoofingField] || blueskyComment.url;
        }
      }
      
      commentsHTML = `
        <div class="webmentions-comments">
          <div class="webmentions-header-container">
            <h3 class="webmentions-header">${t("Comments & Replies")}</h3>
            <a href="${conversationUrl}" target="_blank" rel="noopener" class="join-conversation">
              <span>Join the conversation on Bluesky</span>
              <svg class="bluesky-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 -3.268 64 68.414" width="20" height="20">
                <path d="M13.873 3.805C21.21 9.332 29.103 20.537 32 26.55v15.882c0-.338-.13.044-.41.867-1.512 4.456-7.418 21.847-20.923 7.944-7.111-7.32-3.819-14.64 9.125-16.85-7.405 1.264-15.73-.825-18.014-9.015C1.12 23.022 0 8.51 0 6.55 0-3.268 8.579-.182 13.873 3.805zm36.254 0C42.79 9.332 34.897 20.537 32 26.55v15.882c0-.338.13.044.41.867 1.512 4.456 7.418 21.847 20.923 7.944 7.111-7.32 3.819-14.64-9.125-16.85 7.405 1.264 15.73-.825 18.014-9.015C62.88 23.022 64 8.51 64 6.55c0-9.818-8.578-6.732-13.873-2.745z" class="bluesky-icon-path"/>
              </svg>
            </a>
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
      
      // If there are no comments, find a reaction URL to link to
      let conversationUrl = "https://bsky.app/profile/ssp.sh";
      
      // Only look for reaction URL if we don't already have a comment URL
      if (comments.length === 0 || comments === reactions) {
        // Try to find a reaction with a URL to Bluesky
        if (uniqueReactions.length > 0) {
          // Find the first reaction from Bluesky (should already be sorted newest first)
          const blueskyReaction = uniqueReactions.find(reaction => 
            reaction.url && (reaction.url.includes("bsky.app") || reaction[preventSpoofingField].includes("bsky.app"))
          );
          
          // Use the URL from that reaction if found
          if (blueskyReaction) {
            conversationUrl = blueskyReaction[preventSpoofingField] || blueskyReaction.url;
          }
        }
      }
      
      reactionsHTML = `
        <div>
          <div class="webmentions-header-container">
            <h3 class="webmentions-header">${t("Reactions")}</h3>
            ${comments.length === 0 || comments === reactions ? `
              <a href="${conversationUrl}" target="_blank" rel="noopener" class="join-conversation">
                <span>Join the conversation on Bluesky</span>
                <svg class="bluesky-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 -3.268 64 68.414" width="20" height="20">
                  <path d="M13.873 3.805C21.21 9.332 29.103 20.537 32 26.55v15.882c0-.338-.13.044-.41.867-1.512 4.456-7.418 21.847-20.923 7.944-7.111-7.32-3.819-14.64 9.125-16.85-7.405 1.264-15.73-.825-18.014-9.015C1.12 23.022 0 8.51 0 6.55 0-3.268 8.579-.182 13.873 3.805zm36.254 0C42.79 9.332 34.897 20.537 32 26.55v15.882c0-.338.13.044.41.867 1.512 4.456 7.418 21.847 20.923 7.944 7.111-7.32 3.819-14.64-9.125-16.85 7.405 1.264 15.73-.825 18.014-9.015C62.88 23.022 64 8.51 64 6.55c0-9.818-8.578-6.732-13.873-2.745z" class="bluesky-icon-path"/>
                </svg>
              </a>
            ` : ''}
          </div>
          <ul class="webmentions-list">${uniqueReactions.map(renderReaction).join("")}</ul>
        </div>
      `;
    }
    
    if (commentsHTML || reactionsHTML) {
      container.innerHTML = `
        <div class="webmentions-container">
          ${reactionsHTML}
          ${commentsHTML}
        </div>
      `;
    }
  });
})();
// @license-end