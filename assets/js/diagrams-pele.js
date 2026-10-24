// Diagram renderer: pele (https://pele.run), selected by config.toml mermaidRenderer = "pele".
// Inlined into <head> by partials/head.html (no extra request); lazy: the library is only
// imported once a page has a pre.mermaid. Colours come from --pele-* CSS variables
// (assets/styles/pele.scss), so the theme toggle needs no re-render.
// Same hooks as diagrams-mermaid.js: window.initMermaid() for SPA navigation, plus
// window.__rerenderDiagrams(root) for lightbox.js.
let peleLib = null;

async function loadPele() {
  if (!peleLib) {
    // "@1" = newest 1.x from jsDelivr, never a new major (same policy as mermaid@11 below).
    peleLib = await import('https://cdn.jsdelivr.net/npm/pele@1/dist/pele.js');
  }
  return peleLib;
}

// Pele draws gantt charts at a fixed width (config gantt.useWidth, default 800px) and only
// caps it to the container, where mermaid.js stretched them to 100%. A huge useWidth makes
// the cap win, so the chart fills whatever width the container (or the lightbox) has.
// Row-based height, so nothing distorts. Diagram-level frontmatter/directives still override.
const RENDER_CONFIG = { gantt: { useWidth: 20000 } };

async function renderDiagrams(root = document) {
  const diagrams = root.querySelectorAll('pre.mermaid:not(.mermaid-processed)');
  if (diagrams.length === 0) return;
  const { mount } = await loadPele();
  for (const el of diagrams) {
    if (el.classList.contains('mermaid-processed')) continue; // a parallel call got it
    const source = el.getAttribute('data-original-content') || el.textContent.trim();
    el.setAttribute('data-original-content', source);
    el.classList.add('mermaid-processed');
    el.textContent = '';
    mount(el, source, { zoom: false, config: RENDER_CONFIG, onError: (err) => console.error('Pele render error:', err) });
  }
}

window.initMermaid = () => renderDiagrams();
// lightbox.js: re-mount the cloned diagram so pele lays it out for the overlay width
window.__rerenderDiagrams = (root) => {
  root.querySelectorAll('pre.mermaid.mermaid-processed').forEach((el) => {
    el.classList.remove('mermaid-processed');
    el.textContent = '';
  });
  return renderDiagrams(root);
};

document.addEventListener('DOMContentLoaded', () => renderDiagrams());

// SPA navigation: observe for new diagram elements
const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    if (mutation.addedNodes.length > 0) {
      const hasDiagram = [...mutation.addedNodes].some(node =>
        node.nodeType === Node.ELEMENT_NODE &&
        (node.classList?.contains('mermaid') || node.querySelector?.('.mermaid'))
      );
      if (hasDiagram) {
        renderDiagrams();
        break;
      }
    }
  }
});
observer.observe(document.body, { childList: true, subtree: true });
