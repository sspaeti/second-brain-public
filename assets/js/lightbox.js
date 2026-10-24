// Lightweight expand/lightbox for the graph, mermaid diagrams and note images.
// Markup: any [data-expandable] gets a top-right expand button. Images are
// wrapped in <span class="img-expandable" data-expandable data-full="…">
// (partials/responsive-image.html, _markup/render-image.html) and open the
// original file; same UX as the blog (theme.js initImageExpand).
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
      if (e.target === overlay) return closeLightbox();
      // image mode: the card fills the viewport, so a click beside the picture
      // (on the content/body, not the <img>) closes too
      if (overlay.classList.contains('is-image') &&
          (e.target.classList.contains('lightbox-content') || e.target.classList.contains('lightbox-body'))) closeLightbox();
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
        overlay.classList.remove('is-image');
        overlay.scrollTop = 0;
      }, 200);
    }
  }

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeLightbox();
  });

  window.closeLightbox = closeLightbox;

  window.openLightbox = function (sourceElement) {
    const isGraph = sourceElement.classList.contains('graph-wrapper') ||
                    sourceElement.querySelector('#graph-container');

    const overlay = document.querySelector('.lightbox-overlay') || createOverlay();
    const body = overlay.querySelector('.lightbox-body');
    body.innerHTML = '';
    overlay.classList.toggle('is-image', !!sourceElement.dataset.full);

    if (isGraph) {
      // Live interactive graph. d3/graph.js are lazy (head.html); by the time
      // the expand button is visible they are normally loaded already, but
      // wait on the loader so a fast click still works.
      const graphDiv = document.createElement('div');
      graphDiv.style.width = '100%';
      graphDiv.style.minHeight = '85vh';
      body.appendChild(graphDiv);
      overlay.classList.add('lightbox-visible');

      requestAnimationFrame(async () => {
        if (window.__loadGraphLibs) await window.__loadGraphLibs();
        if (typeof drawGraph !== 'function') return;
        activeSimulation = await drawGraph(
          window.__graphBaseUrl,
          false,
          window.__graphPathColors || [],
          JSON.parse(JSON.stringify(window.__graphConfig || { depth: 1, enableDrag: true, enableLegend: false, enableZoom: true, opacityScale: 4, scale: 1.5, repelForce: 5, fontSize: 0.6 })),
          graphDiv
        );
      });
    } else if (sourceElement.dataset.full) {
      // Image: a fresh <img> of the original file (not the srcset variant the
      // page picked). `.is-image` on the overlay drops the card: native size up
      // to the viewport width on the dark backdrop, tall screenshots scroll
      // (custom.scss; same as the blog's expand-lightbox).
      const src = sourceElement.querySelector('img');
      const full = document.createElement('img');
      full.src = sourceElement.dataset.full;
      full.alt = src ? src.alt : '';
      full.className = 'lightbox-image';
      full.decoding = 'async';
      body.appendChild(full);
      overlay.classList.add('lightbox-visible');
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
      // Images: a plain left-click on the picture opens it too (cursor: zoom-in
      // in custom.scss); modifier clicks are left alone.
      if (container.dataset.full) {
        const img = container.querySelector('img');
        if (img) img.addEventListener('click', (e) => {
          if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
          e.preventDefault();
          window.openLightbox(container);
        });
      }
    });
  }

  document.addEventListener('DOMContentLoaded', attachExpandButtons);

  // Re-attach after SPA navigation
  const obs = new MutationObserver(() => {
    requestAnimationFrame(attachExpandButtons);
  });
  obs.observe(document.body, { childList: true, subtree: true });
})();
