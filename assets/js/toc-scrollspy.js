(function () {
  let observer = null;

  function setup() {
    if (observer) { observer.disconnect(); observer = null; }

    const toc = document.querySelector('#TableOfContents');
    if (!toc) return;

    const links = Array.from(toc.querySelectorAll('a[href^="#"]'));
    if (!links.length) return;

    const idToLink = new Map();
    const headings = [];
    for (const link of links) {
      const id = decodeURIComponent(link.getAttribute('href').slice(1));
      const heading = document.getElementById(id);
      if (heading) {
        idToLink.set(id, link);
        headings.push(heading);
      }
    }
    if (!headings.length) return;

    let activeId = null;
    const setActive = (id) => {
      if (id === activeId) return;
      if (activeId) idToLink.get(activeId)?.classList.remove('toc-active');
      activeId = id;
      if (id) idToLink.get(id)?.classList.add('toc-active');
    };

    const visible = new Set();
    observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) visible.add(entry.target.id);
        else visible.delete(entry.target.id);
      }

      // pick the topmost visible heading; otherwise the last one above the viewport
      let current = null;
      for (const h of headings) {
        if (visible.has(h.id)) { current = h.id; break; }
      }
      if (!current) {
        for (const h of headings) {
          if (h.getBoundingClientRect().top < 100) current = h.id;
          else break;
        }
      }
      setActive(current || headings[0].id);
    }, {
      // shrink the effective viewport: only headings near the top count as "active"
      rootMargin: '-80px 0px -70% 0px',
      threshold: 0,
    });

    for (const h of headings) observer.observe(h);
  }

  window.initTocScrollspy = setup;

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', setup);
  } else {
    setup();
  }
})();
