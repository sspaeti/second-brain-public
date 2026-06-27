;(async function () {
  // ── Utility shims (brain already has these from util.js; blog needs them) ──

  const _removeMarkdown =
    typeof removeMarkdown !== 'undefined'
      ? removeMarkdown
      : (s) => (s || '').replace(/<[^>]*>/g, '').replace(/[#*`_\[\]]/g, '')

  const _highlight =
    typeof highlight !== 'undefined'
      ? highlight
      : (content, term) => {
          if (!term || !content) return content || ''
          const idx = content.toLowerCase().indexOf(term.toLowerCase())
          if (idx === -1) return content.slice(0, 200) + (content.length > 200 ? '…' : '')
          const start = Math.max(0, idx - 60)
          const end   = Math.min(content.length, idx + term.length + 100)
          return (
            (start > 0 ? '…' : '') +
            content.slice(start, idx) +
            `<span class="search-highlight">${content.slice(idx, idx + term.length)}</span>` +
            content.slice(idx + term.length, end) +
            (end < content.length ? '…' : '')
          )
        }

  function _openSearch() {
    const el  = document.getElementById('search-container')
    const bar = document.getElementById('search-bar')
    const res = document.getElementById('results-container')
    if (!el) return
    if (el.style.display === 'none' || el.style.display === '') {
      if (res) res.innerHTML = ''
      el.style.display = 'block'
      if (bar) { bar.value = ''; bar.focus() }
    } else {
      el.style.display = 'none'
    }
  }

  function _closeSearch() {
    const el = document.getElementById('search-container')
    if (el) el.style.display = 'none'
  }

  // Expose globally so search-modal.html inline script can call openSearch()
  window.openSearch  = _openSearch
  window.closeSearch = _closeSearch

  // ── State ──────────────────────────────────────────────────────────────────

  const encoder = (str) => str.toLowerCase().split(/([^a-z]|[^\x00-\x7F])/)
  const idx = new FlexSearch.Document({
    cache:    true,
    charset:  'latin:extra',
    optimize: true,
    index: [
      { field: 'content', tokenize: 'reverse', encode: encoder },
      { field: 'title',   tokenize: 'forward',  encode: encoder },
    ],
  })

  let allData      = {}
  let activeSource = 'all'
  let activeDate   = 'any'
  let term         = ''

  // ── Load index ─────────────────────────────────────────────────────────────

  const res = await fetch(window.SEARCH_V2_URL)
  allData = await res.json()

  for (const [key, val] of Object.entries(allData)) {
    idx.add({
      id:      key,
      title:   val.title   ?? '',
      content: _removeMarkdown(val.content ?? ''),
    })
  }

  // Update placeholder with live count
  ;(function () {
    const _bar = document.getElementById('search-bar')
    if (!_bar) return
    const total = Object.keys(allData).length
    if (total > 0) {
      _bar.placeholder = `Search ${total.toLocaleString()} notes (blog + brain)… · Ctrl+K or /`
    }
  })()

  // ── Filter buttons ─────────────────────────────────────────────────────────

  document.querySelectorAll('#search-filters .filter-source button').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#search-filters .filter-source button')
        .forEach(b => b.classList.remove('active'))
      btn.classList.add('active')
      activeSource = btn.dataset.source
      doSearch()
    })
  })

  document.querySelectorAll('#search-filters .filter-date button').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#search-filters .filter-date button')
        .forEach(b => b.classList.remove('active'))
      btn.classList.add('active')
      activeDate = btn.dataset.date
      doSearch()
    })
  })

  // ── Keyboard shortcuts ─────────────────────────────────────────────────────

  document.addEventListener('keydown', (e) => {
    if (e.key === 'k' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      _openSearch()
    }
    // '/' opens search when not focused on an input (Linux-friendly fallback for Ctrl+K)
    if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
      e.preventDefault()
      _openSearch()
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      _closeSearch()
    }
  })

  // Brain search icon (id="search-icon")
  const brainIcon = document.getElementById('search-icon')
  if (brainIcon) {
    brainIcon.addEventListener('click', _openSearch)
  }

  // Close when clicking outside the search space
  const container = document.getElementById('search-container')
  if (container) {
    container.addEventListener('click', _closeSearch)
  }
  const space = document.getElementById('search-space')
  if (space) {
    space.addEventListener('click', (e) => e.stopPropagation())
  }

  // ── Date filter ────────────────────────────────────────────────────────────

  const now = new Date()
  function passesDate(entry) {
    if (activeDate === 'any') return true
    const d = new Date(entry.updated || entry.created || '')
    if (isNaN(d)) return true
    const diffYears = (now - d) / (1000 * 60 * 60 * 24 * 365.25)
    return activeDate === 'year' ? diffYears <= 1 : diffYears <= 2
  }

  // ── Result card ────────────────────────────────────────────────────────────

  function resultCard(id, entry) {
    const badge = `<span class="source-badge source-${entry.source}">${entry.source}</span>`
    const date  = entry.updated
      ? `<span class="result-date">${entry.updated}</span>`
      : ''
    return `<button class="result-card" id="${id}">
      <h3>${_highlight(entry.title, term)}${badge}${date}</h3>
      <p>${_highlight(_removeMarkdown(entry.content ?? ''), term)}</p>
    </button>`
  }

  // ── Search ─────────────────────────────────────────────────────────────────

  function doSearch() {
    const resultsEl = document.getElementById('results-container')
    if (!resultsEl) return
    if (!term) {
      resultsEl.innerHTML = ''
      return
    }

    const raw = idx.search(term, [
      { field: 'content', limit: 15 },
      { field: 'title',   limit: 8  },
    ])

    const seen = new Set()
    const ids  = []
    for (const r of raw) {
      for (const id of r.result) {
        if (!seen.has(id)) { seen.add(id); ids.push(id) }
      }
    }

    // Blog entries float first (blog-boost)
    const blog  = []
    const brain = []
    for (const id of ids) {
      const e = allData[id]
      if (!e) continue
      if (!passesDate(e)) continue
      if (activeSource !== 'all' && e.source !== activeSource) continue
      e.source === 'blog' ? blog.push(id) : brain.push(id)
    }

    const ordered = [...blog, ...brain]
    if (ordered.length === 0) {
      resultsEl.innerHTML = `<button class="result-card">
        <h3>No results.</h3>
        <p>Try another search term?</p>
      </button>`
      return
    }

    resultsEl.innerHTML = ordered.map(id => resultCard(id, allData[id])).join('\n')

    // Navigation: SPA on brain, plain href on blog
    const baseUrl = typeof BASE_URL !== 'undefined' ? BASE_URL.replace(/\/$/, '') : ''
    resultsEl.querySelectorAll('.result-card[id]').forEach(card => {
      card.addEventListener('click', () => {
        // card.id is the absolute path e.g. /brain/zettelkasten or /blog/post
        const textFrag = '#:~:text=' + encodeURIComponent(term)
        const source = (allData[card.id] || {}).source
        if (source === 'brain' && window.Million?.navigate) {
          // SPA navigation: BASE_URL already includes /brain/, strip it from card.id
          const brainRelPath = card.id.replace(/^\/brain/, '') + textFrag
          window.Million.navigate(new URL(baseUrl + brainRelPath), '.singlePage')
          _closeSearch()
        } else {
          // Blog entries (and fallback): plain navigation
          window.location.href = card.id + textFrag
        }
      })
    })
  }

  // Wire search input
  const searchBar = document.getElementById('search-bar')
  if (searchBar) {
    searchBar.addEventListener('input', (e) => {
      term = e.target.value
      doSearch()
    })
    searchBar.addEventListener('keyup', (e) => {
      if (e.key === 'Enter') {
        const first = document.querySelector('.result-card[id]')
        if (first) {
          const target = first.id + '#:~:text=' + encodeURIComponent(term)
          window.location.href = target
        }
      }
    })
  }
})()
