import {
  apply,
  navigate,
  prefetch,
  router,
} from "/brain/js/million-1.11.5/router.js"
// ^ self-hosted: million@1.11.5/dist/router.mjs + its two chunks, bundled to
// one ESM file with esbuild (static/js/million-1.11.5/router.js). Was loaded
// from unpkg.com, which has had outages and cost an extra origin on every load.

export const attachSPARouting = (init, rerender) => {
  // Million treats every same-origin <a> as a route: it prefetches it on
  // mouseover, fetches it on click, parses the response as HTML and runs
  // init() on it. Two kinds of links must never reach it:
  //  - cross-section links (brain <-> blog): a different page shell, needs a
  //    full load.
  //  - links to files (gallery lightbox .webp, PDFs, .gif …): the "page" is
  //    binary, DOMParser turns it into garbage text and KaTeX then spends
  //    seconds warning about every byte that looks like a $ formula.
  const brainPath = '/brain/'
  const isFileLink = (url) => /\.(?!html?$)[a-z0-9]{1,5}$/i.test(url.pathname)
  const bypassRouter = (link) => {
    try {
      const targetUrl = new URL(link.href)
      if (targetUrl.origin !== window.location.origin) return false // Million ignores these anyway
      const currentInBrain = window.location.pathname.startsWith(brainPath)
      const targetInBrain = targetUrl.pathname.startsWith(brainPath)
      return currentInBrain !== targetInBrain || isFileLink(targetUrl)
    } catch (e) {
      return false // Invalid URL, let the browser decide
    }
  }

  // Million registers its click/mouseover listeners on window (bubble phase)
  // inside router() on DOMContentLoaded; this module runs earlier, so our
  // bubble-phase listeners come first and stopImmediatePropagation() skips
  // Million's. Bubble, not capture: every handler below window (lightbox2's
  // delegated click on body, popover's click/mouseenter on the link) must
  // still run — a capture-phase stopPropagation on window would swallow the
  // event before it ever reached them and gallery clicks would open the
  // raw image instead of the lightbox.
  const interceptForMillion = (event) => {
    const link = event.target.closest && event.target.closest("a")
    if (link && bypassRouter(link)) event.stopImmediatePropagation()
  }
  window.addEventListener("click", interceptForMillion)
  window.addEventListener("mouseover", interceptForMillion)

  // Intercept form submits BEFORE Million.js — let cross-origin forms submit natively
  const interceptCrossOriginSubmit = (event) => {
    const form = event.target.closest("form")
    if (!form) return
    try {
      const actionUrl = new URL(form.action)
      if (actionUrl.origin !== window.location.origin) {
        event.stopImmediatePropagation()
        // Don't call preventDefault — native form submission proceeds normally
      }
    } catch (e) {
      // Invalid URL, ignore
    }
  }
  window.addEventListener("submit", interceptCrossOriginSubmit, true)

  // Custom navigate wrapper for programmatic navigation
  const customNavigate = (url, selector) => {
    const targetUrl = typeof url === 'string' ? new URL(url, window.location.origin) : url
    const currentInBrain = window.location.pathname.startsWith(brainPath)
    const targetInBrain = targetUrl.pathname.startsWith(brainPath)

    // If navigating between brain and non-brain sections, force full page load
    if (currentInBrain !== targetInBrain) {
      window.location.href = targetUrl.href
      return
    }

    // Otherwise use SPA navigation
    navigate(targetUrl, selector)
  }

  // Attach SPA functions to the global Million namespace
  window.Million = {
    apply,
    navigate: customNavigate,
    prefetch,
    router,
  }

  const render = () => requestAnimationFrame(rerender)

  window.addEventListener("DOMContentLoaded", () => {
    apply((doc) => init(doc))
    init()
    router(".singlePage")
    render()
  })
  window.addEventListener("million:navigate", render)
}
