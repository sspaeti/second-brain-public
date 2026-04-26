// Lightweight expand/lightbox for graph and mermaid diagrams
(function () {
  let activeSimulation = null;

  function createOverlay() {
    const overlay = document.createElement('div');
    overlay.className = 'lightbox-overlay';
    overlay.innerHTML = `
      <div class="lightbox-content">
        <button class="lightbox-close" aria-label="Close">&times;</button>
        <div class="lightbox-body"></div>
      </div>
    `;
    document.body.appendChild(overlay);

    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) closeLightbox();
    });
    overlay.querySelector('.lightbox-close').addEventListener('click', closeLightbox);
    return overlay;
  }

  function closeLightbox() {
    if (activeSimulation) {
      activeSimulation.stop();
      activeSimulation = null;
    }
    const overlay = document.querySelector('.lightbox-overlay');
    if (overlay) {
      overlay.classList.remove('lightbox-visible');
      setTimeout(() => {
        overlay.querySelector('.lightbox-body').innerHTML = '';
      }, 200);
    }
  }

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeLightbox();
  });

  window.openLightbox = function (sourceElement) {
    const isGraph = sourceElement.classList.contains('graph-wrapper') ||
                    sourceElement.querySelector('#graph-container');

    const overlay = document.querySelector('.lightbox-overlay') || createOverlay();
    const body = overlay.querySelector('.lightbox-body');
    body.innerHTML = '';

    if (isGraph && typeof drawGraph === 'function') {
      // Live interactive graph
      const graphDiv = document.createElement('div');
      graphDiv.style.width = '100%';
      graphDiv.style.minHeight = '85vh';
      body.appendChild(graphDiv);
      overlay.classList.add('lightbox-visible');

      requestAnimationFrame(async () => {
        activeSimulation = await drawGraph(
          window.__graphBaseUrl,
          false,
          window.__graphPathColors || [],
          JSON.parse(JSON.stringify(window.__graphConfig || { depth: 1, enableDrag: true, enableLegend: false, enableZoom: true, opacityScale: 4, scale: 1.5, repelForce: 5, fontSize: 0.6 })),
          graphDiv
        );
      });
    } else {
      // Clone content (mermaid, etc.)
      const clone = sourceElement.cloneNode(true);
      const btn = clone.querySelector('.expand-btn');
      if (btn) btn.remove();
      body.appendChild(clone);
      overlay.classList.add('lightbox-visible');
    }
  };

  function attachExpandButtons() {
    document.querySelectorAll('[data-expandable]:not([data-expand-attached])').forEach(container => {
      container.setAttribute('data-expand-attached', 'true');
      container.style.position = 'relative';
      const btn = document.createElement('button');
      btn.className = 'expand-btn';
      btn.setAttribute('aria-label', 'Expand');
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>';
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        window.openLightbox(container);
      });
      container.appendChild(btn);
    });
  }

  document.addEventListener('DOMContentLoaded', attachExpandButtons);

  // Re-attach after SPA navigation
  const obs = new MutationObserver(() => {
    requestAnimationFrame(attachExpandButtons);
  });
  obs.observe(document.body, { childList: true, subtree: true });
})();
