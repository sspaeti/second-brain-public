import {
  apply,
  navigate,
  prefetch,
  router,
} from "https://unpkg.com/million@1.11.5/dist/router.mjs"

export const attachSPARouting = (init, rerender) => {
  // Intercept clicks BEFORE Million.js to handle cross-section navigation
  const interceptCrossSectionClicks = (event) => {
    const link = event.target.closest("a")
    if (!link) return

    try {
      const targetUrl = new URL(link.href)
      // Only handle same-origin links
      if (targetUrl.origin !== window.location.origin) return

      const brainPath = '/brain/'
      const currentInBrain = window.location.pathname.startsWith(brainPath)
      const targetInBrain = targetUrl.pathname.startsWith(brainPath)

      // If navigating between brain and non-brain sections, allow default behavior
      // and stop propagation so Million.js doesn't intercept it
      if (currentInBrain !== targetInBrain) {
        event.stopPropagation()
        // Let browser handle the navigation normally
        return
      }
    } catch (e) {
      // Invalid URL, ignore
    }
  }

  // Add our click handler in capture phase (runs before Million's)
  window.addEventListener("click", interceptCrossSectionClicks, true)

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
    const brainPath = '/brain/'
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
