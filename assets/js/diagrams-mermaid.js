// Diagram renderer: mermaid.js, selected by config.toml mermaidRenderer = "mermaid" (or unset).
// Inlined into <head> by partials/head.html; lazy: imported once a page has a .mermaid.
// Exposes window.initMermaid() for SPA navigation. Theme toggle re-renders.
let mermaidLib = null;
let isRendering = false;

function getCurrentTheme() {
  const saved = document.documentElement.getAttribute('saved-theme');
  if (saved) return saved;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

async function loadMermaid() {
  if (!mermaidLib) {
    // "@11" = jsDelivr resolves to the newest 11.x (patch/minor fixes arrive on their own),
    // but never a major (12.x exists since 2026-09 — bump deliberately, and together with the
    // blog: sspaeti-hugo-blog/layouts/partials/assets.html uses the same range).
    const module = await import('https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs');
    mermaidLib = module.default;
  }
  return mermaidLib;
}

async function renderMermaidDiagrams() {
  const diagrams = document.querySelectorAll('.mermaid:not(.mermaid-processed)');
  if (diagrams.length === 0 || isRendering) return;

  isRendering = true;
  try {
    const mermaid = await loadMermaid();
    const isDark = getCurrentTheme() === 'dark';

    mermaid.initialize({
      startOnLoad: false,
      theme: isDark ? 'dark' : 'neutral',
      darkMode: isDark,
      htmlLabels: true,
      themeVariables: isDark ? {
        primaryColor: '#2b2b33',
        primaryTextColor: '#f8f8f2',
        primaryBorderColor: '#81A1C1',
        lineColor: '#81A1C1',
        secondaryColor: '#383844',
        tertiaryColor: '#2E3440',
        background: '#2E3440',
        mainBkg: '#383844',
        textColor: '#f8f8f2',
        edgeLabelBackground: '#2b2b33',
        nodeTextColor: '#f8f8f2'
      } : {}
    });

    for (const el of diagrams) {
      try {
        const content = el.getAttribute('data-original-content') || el.textContent;
        el.setAttribute('data-original-content', content);
        const id = `mermaid-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
        const result = await mermaid.render(id, content);
        el.innerHTML = result.svg;
        el.classList.add('mermaid-processed');
      } catch (err) {
        console.error('Mermaid render error:', err);
      }
    }
  } finally {
    isRendering = false;
  }
}

function resetAndRerender() {
  document.querySelectorAll('.mermaid.mermaid-processed').forEach(el => {
    el.classList.remove('mermaid-processed');
    const original = el.getAttribute('data-original-content');
    if (original) el.textContent = original;
  });
  renderMermaidDiagrams();
}

window.initMermaid = renderMermaidDiagrams;

// Initialize on page load
document.addEventListener('DOMContentLoaded', renderMermaidDiagrams);

// SPA navigation: observe for new mermaid elements
const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    if (mutation.addedNodes.length > 0) {
      const hasMermaid = [...mutation.addedNodes].some(node =>
        node.nodeType === Node.ELEMENT_NODE &&
        (node.classList?.contains('mermaid') || node.querySelector?.('.mermaid'))
      );
      if (hasMermaid) {
        renderMermaidDiagrams();
        break;
      }
    }
  }
});
observer.observe(document.body, { childList: true, subtree: true });

// Theme toggle re-render
document.addEventListener('DOMContentLoaded', () => {
  const toggle = document.querySelector('#darkmode-toggle');
  if (toggle) {
    toggle.addEventListener('change', () => {
      requestAnimationFrame(resetAndRerender);
    });
  }
});

// System dark mode change
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
  if (!document.documentElement.hasAttribute('saved-theme')) {
    requestAnimationFrame(resetAndRerender);
  }
});
