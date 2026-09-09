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
  // Page-context default ('blog'/'all' on the blog, 'brain' on the brain). Read
  // it up front so the very first keystroke filters correctly, even before the
  // index has finished loading — otherwise early queries leak the other source.
  let activeSource = (typeof window !== 'undefined' && window.SEARCH_DEFAULT_SOURCE) || 'all'
  let activeDate   = 'any'
  let term         = ''

  // ── Deferred index load ────────────────────────────────────────────────────
  // Starts after page load in an idle slot — invisible to Core Web Vitals.
  // _openSearch awaits _startLoad(), which is a no-op if already done.

  let _loadPromise = null

  function _startLoad() {
    if (_loadPromise) return _loadPromise
    _loadPromise = fetch(window.SEARCH_V2_URL)
      .then((r) => r.json())
      .then((data) => new Promise((resolve) => {
        allData = data
        const entries = Object.entries(data)
        let i = 0

        // Process items in idle slots so no single task exceeds ~50ms (TBT threshold).
        // Uses deadline.timeRemaining() when available, fixed 30-item batches as fallback.
        function processChunk(deadline) {
          let count = 0
          while (i < entries.length) {
            if (deadline ? deadline.timeRemaining() < 2 : count >= 30) break
            const [key, val] = entries[i++]
            idx.add({ id: key, title: val.title ?? '', content: _removeMarkdown(val.content ?? '') })
            count++
          }
          if (i < entries.length) {
            if (window.requestIdleCallback) requestIdleCallback(processChunk, { timeout: 10000 })
            else setTimeout(() => processChunk(null), 0)
          } else {
            const _bar = document.getElementById('search-bar')
            if (_bar) {
              const total = Object.keys(allData).length
              if (total > 0) _bar.placeholder = `Search ${total.toLocaleString()} notes (blog + brain) · "quotes" for exact phrase · Ctrl+K or /`
            }
            resolve()
          }
        }

        if (window.requestIdleCallback) requestIdleCallback(processChunk, { timeout: 10000 })
        else setTimeout(() => processChunk(null), 0)
      }))
    return _loadPromise
  }

  function _scheduleLoad() {
    ;(window.requestIdleCallback || ((fn) => setTimeout(fn, 200)))(() => _startLoad(), { timeout: 2000 })
  }
  if (document.readyState === 'complete') {
    _scheduleLoad()
  } else {
    window.addEventListener('load', _scheduleLoad, { once: true })
  }

  async function _openSearch() {
    const el  = document.getElementById('search-container')
    const bar = document.getElementById('search-bar')
    const res = document.getElementById('results-container')
    if (!el) return
    if (el.style.display === 'none' || el.style.display === '') {
      if (res) res.innerHTML = ''
      el.style.display = 'block'
      if (bar) { bar.value = ''; bar.focus() }
      await _startLoad() // no-op if already loaded; awaits if still in progress
    } else {
      el.style.display = 'none'
    }
  }

  function _closeSearch() {
    const el = document.getElementById('search-container')
    if (el) el.style.display = 'none'
  }

  // Expose globally so search-modal.html inline script can call openSearch()
  window.__searchStartLoad = _startLoad // for tests/console
  window.openSearch  = _openSearch
  window.closeSearch = _closeSearch

  // ── Filter buttons ─────────────────────────────────────────────────────────

  // Mark the page-context default active immediately (the markup hardcodes
  // "All" active). Keeps the highlighted button in sync with activeSource from
  // first paint, so there's no flip once the index finishes loading.
  if (activeSource !== 'all') {
    document.querySelectorAll('#search-filters .filter-source button').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.source === activeSource)
    })
  }

  document.querySelectorAll('#search-filters .filter-source button').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#search-filters .filter-source button')
        .forEach(b => b.classList.remove('active'))
      btn.classList.add('active')
      activeSource = btn.dataset.source
      doSearch(term, activeSource, activeDate, document.getElementById('results-container'))
    })
  })

  document.querySelectorAll('#search-filters .filter-date button').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#search-filters .filter-date button')
        .forEach(b => b.classList.remove('active'))
      btn.classList.add('active')
      activeDate = btn.dataset.date
      doSearch(term, activeSource, activeDate, document.getElementById('results-container'))
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
  function passesDate(entry, date) {
    if (date === 'any') return true
    const d = new Date(entry.updated || entry.created || '')
    if (isNaN(d)) return true
    const diffYears = (now - d) / (1000 * 60 * 60 * 24 * 365.25)
    return date === 'year' ? diffYears <= 1 : diffYears <= 2
  }

  // ── Result card ────────────────────────────────────────────────────────────

  function resultCard(id, entry, term) {
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

  // Returns ordered ids for a term. Multi-word terms rank exact-phrase hits
  // (title, then content) above FlexSearch's loose word-AND hits, which are
  // also capped by the index limit and so miss phrase matches. Wrapping the
  // term in double quotes returns only the phrase tier.
  function rankResults(term, source, date) {
    const strict = /^\s*"[^"]+"\s*$/.test(term)
    const phrase = (strict ? term.trim().slice(1, -1) : term).trim().toLowerCase()
    if (!phrase) return []

    const passes = (e) => e && passesDate(e, date) && (source === 'all' || e.source === source)

    // Punctuation-insensitive phrase match: "note taking" hits "Note-Taking".
    const norm = (s) => (s || '').toLowerCase().replace(/[^\p{L}\p{N}]+/gu, ' ').trim()
    const needle = norm(phrase)
    const titleHits = [], contentHits = []
    if (needle && (strict || needle.includes(' '))) {
      for (const [id, e] of Object.entries(allData)) {
        if (!passes(e)) continue
        if (norm(e.title).includes(needle)) titleHits.push(id)
        else if (norm(_removeMarkdown(e.content)).includes(needle)) contentHits.push(id)
      }
    }

    const loose = []
    if (!strict) {
      // Title first: FlexSearch returns hits in insertion order, not by score,
      // and results are merged in field order, so title matches must lead.
      const raw = idx.search(phrase, [
        { field: 'title',   limit: 8  },
        { field: 'content', limit: 15 },
      ])
      for (const r of raw) for (const id of r.result) if (passes(allData[id])) loose.push(id)
    }

    // Blog entries float first within each tier (blog-boost)
    const boost = (ids) => [...ids.filter(id => allData[id].source === 'blog'), ...ids.filter(id => allData[id].source !== 'blog')]
    const seen = new Set()
    return [...boost(titleHits), ...boost(contentHits), ...boost(loose)].filter(id => !seen.has(id) && seen.add(id))
  }
  window.searchRank = rankResults

  function doSearch(term, source, date, resultsEl) {
    if (!resultsEl) return
    if (!term) {
      resultsEl.innerHTML = ''
      return
    }

    const ordered = rankResults(term, source, date)
    if (ordered.length === 0) {
      resultsEl.innerHTML = `<button class="result-card">
        <h3>No results.</h3>
        <p>Try another search term?</p>
      </button>`
      return
    }

    const hl = term.replace(/^\s*"|"\s*$/g, '')
    resultsEl.innerHTML = ordered.map(id => resultCard(id, allData[id], hl)).join('\n')

    // Navigation: SPA on brain, plain href on blog
    const baseUrl = typeof BASE_URL !== 'undefined' ? BASE_URL.replace(/\/$/, '') : ''
    resultsEl.querySelectorAll('.result-card[id]').forEach(card => {
      card.addEventListener('click', () => {
        // card.id is the absolute path e.g. /brain/zettelkasten or /blog/post
        const textFrag = '#:~:text=' + encodeURIComponent(hl)
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
      doSearch(term, activeSource, activeDate, document.getElementById('results-container'))
    })
    searchBar.addEventListener('keyup', (e) => {
      if (e.key === 'Enter') {
        const first = document.querySelector('#results-container .result-card[id]')
        if (first) {
          const target = first.id + '#:~:text=' + encodeURIComponent(term)
          window.location.href = target
        }
      }
    })
  }

  // ── Inline full-page search (blog /search page) ──────────────────────────────
  // Dormant everywhere else: this whole block is a no-op unless #search-page-bar
  // exists on the page (only the blog's /search template renders it). The popup
  // modal above is untouched. Reuses the same idx / allData / doSearch engine
  // with its own independent filter state (source defaults to 'all').
  const pageBar = document.getElementById('search-page-bar')
  if (pageBar) {
    const pageResults = document.getElementById('search-page-results')
    let pTerm = ''
    let pSource = 'all'
    let pDate = 'any'

    const run = () => doSearch(pTerm, pSource, pDate, pageResults)

    // Lazy-load the shared index on first focus; awaited before any search.
    let _ensured = null
    const ensure = () => (_ensured = _ensured || _startLoad())

    pageBar.addEventListener('focus', ensure, { once: true })

    pageBar.addEventListener('input', async (e) => {
      pTerm = e.target.value
      await ensure()
      run()
    })

    pageBar.addEventListener('keyup', (e) => {
      if (e.key === 'Enter') {
        const first = pageResults && pageResults.querySelector('.result-card[id]')
        if (first) window.location.href = first.id + '#:~:text=' + encodeURIComponent(pTerm)
      }
    })

    document.querySelectorAll('#search-page-filters .filter-source button').forEach(btn => {
      btn.addEventListener('click', async () => {
        document.querySelectorAll('#search-page-filters .filter-source button')
          .forEach(b => b.classList.remove('active'))
        btn.classList.add('active')
        pSource = btn.dataset.source
        await ensure()
        run()
      })
    })

    document.querySelectorAll('#search-page-filters .filter-date button').forEach(btn => {
      btn.addEventListener('click', async () => {
        document.querySelectorAll('#search-page-filters .filter-date button')
          .forEach(b => b.classList.remove('active'))
        btn.classList.add('active')
        pDate = btn.dataset.date
        await ensure()
        run()
      })
    })

    document.querySelectorAll('.search-page-chip').forEach(chip => {
      chip.addEventListener('click', async () => {
        pTerm = chip.dataset.term || chip.textContent.replace(/^try:\s*/i, '').trim()
        pageBar.value = pTerm
        pageBar.focus()
        await ensure()
        run()
      })
    })

    // Autofocus on load kicks off the index fetch immediately.
    pageBar.focus()
  }
})()
