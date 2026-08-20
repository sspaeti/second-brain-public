// Reveals the preloaded time buckets under Recent Notes (partials/recent.html).
// Delegated on document because Million's SPA router swaps .singlePage content
// without re-running inline scripts.
document.addEventListener("click", (e) => {
  const btn = e.target.closest("#show-more-btn")
  if (!btn) return
  const next = document.querySelector(".more-notes.notes-hidden")
  if (next) next.classList.remove("notes-hidden")
  const upcoming = document.querySelector(".more-notes.notes-hidden")
  if (upcoming) {
    const count = btn.querySelector(".count")
    if (count) count.innerHTML = "&middot; " + upcoming.dataset.label
  } else {
    btn.remove()
  }
})
