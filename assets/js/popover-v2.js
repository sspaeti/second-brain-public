// Popover v2: full-content fetch-based previews (Quartz v4 style).
// Triggered by initPopoverV2() when the enableLinkPreviewV2 flag is true.
// Internal links rendered with class="internal-link" and href.
// Branches on Content-Type: text/html -> extract .popover-hint elements;
// image/* -> render <img>; application/pdf -> embed <iframe>.

(function () {
  const popoverCache = new Map(); // pathname -> Promise<HTMLElement|null>
  let activeAnchor = null;

  function clearActivePopover() {
    activeAnchor = null;
    document.querySelectorAll(".popover-v2").forEach((el) => {
      el.classList.remove("active-popover");
    });
  }

  function setPosition(link, popoverElement, clientX, clientY) {
    const FUI = window.FloatingUIDOM;
    if (!FUI) return;
    FUI.computePosition(link, popoverElement, {
      strategy: "fixed",
      middleware: [FUI.inline({ x: clientX, y: clientY }), FUI.shift(), FUI.flip()],
    }).then(({ x, y }) => {
      Object.assign(popoverElement.style, {
        transform: `translate(${Math.round(x)}px, ${Math.round(y)}px)`,
      });
    });
  }

  function buildHtmlPopover(html, targetUrl) {
    // Prefix every id with "popover-internal-" so duplicates do not collide
    // with the host page's IDs.
    html.querySelectorAll("[id]").forEach((el) => {
      el.id = "popover-internal-" + el.id;
    });
    // Rewrite relative URLs so links/images inside the popover still resolve.
    html.querySelectorAll("[href], [src]").forEach((el) => {
      const attr = el.hasAttribute("href") ? "href" : "src";
      const val = el.getAttribute(attr);
      if (!val) return;
      if (val.startsWith("#") || /^[a-z]+:\/\//i.test(val) || val.startsWith("//")) return;
      try {
        el.setAttribute(attr, new URL(val, targetUrl).toString());
      } catch (_) {
        // ignore malformed URLs
      }
    });

    const hints = [...html.getElementsByClassName("popover-hint")];
    if (hints.length === 0) return null;

    const inner = document.createElement("div");
    inner.classList.add("popover-v2-inner");
    inner.dataset.contentType = "text/html";
    hints.forEach((h) => inner.appendChild(h));
    return inner;
  }

  function buildImagePopover(url) {
    const inner = document.createElement("div");
    inner.classList.add("popover-v2-inner");
    inner.dataset.contentType = "image/*";
    const img = document.createElement("img");
    img.src = url;
    img.alt = "";
    inner.appendChild(img);
    return inner;
  }

  function buildPdfPopover(url) {
    const inner = document.createElement("div");
    inner.classList.add("popover-v2-inner");
    inner.dataset.contentType = "application/pdf";
    const iframe = document.createElement("iframe");
    iframe.src = url;
    inner.appendChild(iframe);
    return inner;
  }

  async function fetchAndBuild(targetUrl) {
    let response;
    try {
      response = await fetch(targetUrl.toString(), { credentials: "same-origin" });
    } catch (e) {
      console.warn("[popover-v2] fetch failed", e);
      return null;
    }
    if (!response.ok) return null;

    const contentTypeHeader = response.headers.get("Content-Type") || "";
    const [contentType] = contentTypeHeader.split(";");
    const [category, subtype] = contentType.trim().split("/");

    let inner = null;
    if (category === "image") {
      inner = buildImagePopover(targetUrl.toString());
    } else if (category === "application" && subtype === "pdf") {
      inner = buildPdfPopover(targetUrl.toString());
    } else {
      const text = await response.text();
      const parsed = new DOMParser().parseFromString(text, "text/html");
      inner = buildHtmlPopover(parsed, targetUrl);
    }
    if (!inner) return null;

    const outer = document.createElement("div");
    outer.classList.add("popover-v2");
    outer.appendChild(inner);
    document.body.appendChild(outer);
    return outer;
  }

  function getCacheKey(link) {
    return link.pathname + link.search;
  }

  async function onMouseEnter(event) {
    const link = event.currentTarget;
    if (!link.href) return;
    if (link.dataset.noPopover === "true") return;

    activeAnchor = link;

    const targetUrl = new URL(link.href);
    const hash = decodeURIComponent(targetUrl.hash || "");
    targetUrl.hash = "";

    const key = getCacheKey(targetUrl);
    if (!popoverCache.has(key)) {
      popoverCache.set(key, fetchAndBuild(targetUrl));
    }

    const popoverElement = await popoverCache.get(key);
    if (!popoverElement) return;
    if (activeAnchor !== link) return;

    clearActivePopover();
    popoverElement.classList.add("active-popover");
    setPosition(link, popoverElement, event.clientX, event.clientY);

    if (hash) {
      const inner = popoverElement.querySelector(".popover-v2-inner");
      const target = inner && inner.querySelector("#popover-internal-" + CSS.escape(hash.slice(1)));
      if (target && inner) {
        inner.scroll({ top: target.offsetTop - 12, behavior: "instant" });
      }
    }
  }

  function onMouseLeave() {
    clearActivePopover();
  }

  window.initPopoverV2 = function initPopoverV2() {
    const links = document.querySelectorAll("a.internal-link[href]");
    links.forEach((link) => {
      link.addEventListener("mouseenter", onMouseEnter);
      link.addEventListener("mouseleave", onMouseLeave);
    });
  };
})();
