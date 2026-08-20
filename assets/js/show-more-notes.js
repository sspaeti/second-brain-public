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

// NEW/UPD filter chips next to the Recent Notes heading. Filtering hides
// non-matching rows within the revealed groups (CSS :has rules in custom.scss);
// the "Show more" time steps keep working while a filter is active.
document.addEventListener("click", (e) => {
  const chip = e.target.closest(".filter-btn")
  if (!chip) return
  const list = chip.closest(".content-list")
  if (!list) return
  list.classList.toggle("filter-new", chip.dataset.filter === "new")
  list.classList.toggle("filter-upd", chip.dataset.filter === "upd")
  list.querySelectorAll(".filter-btn").forEach((b) => {
    b.classList.toggle("active", b === chip)
    b.setAttribute("aria-pressed", b === chip ? "true" : "false")
  })
})
