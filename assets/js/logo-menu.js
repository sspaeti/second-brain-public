/* Right-click on the header logo opens a small brand menu (media kit, logo and
   portrait downloads) instead of the browser context menu, the way Vercel,
   Linear and GitHub do it. Left click still goes home. A second right-click on
   the open menu falls through to the native menu.
   Canonical copy: second-brain-public/assets/js/logo-menu.js (brain-bundle in
   layouts/partials/head.html; logo anchor .ssp-brand-logo). The blog's
   `make sync-popover` (run by `make prepare`) copies it to
   sspaeti-hugo-blog/assets/js/logo-menu.js (site bundle in
   layouts/partials/assets.html; logo anchor .header-logo), so edit it here.
   Asset URLs are absolute on ssp.sh, where the brain lives under /brain/, so
   they resolve on both. The menu's own logo image is read from the clicked
   anchor. Styles: logo-menu.scss (same canonical/copy arrangement). */
(function () {
  'use strict';
  var LOGO = '.header-logo, .ssp-brand-logo';
  var menu = null;

  function build(anchor) {
    var img = anchor.querySelector('img');
    menu = document.createElement('div');
    menu.className = 'logo-menu';
    menu.setAttribute('role', 'menu');
    menu.setAttribute('aria-label', 'Brand assets');
    menu.hidden = true;
    menu.innerHTML =
      '<div class="logo-menu__head">' +
        (img ? '<img src="' + img.getAttribute('src') + '" alt="" width="36" height="36" loading="lazy">' : '') +
        '<span><b>ssp.sh</b><small>Simon Späti · SSP Data GmbH</small></span>' +
      '</div>' +
      '<a role="menuitem" href="/media/"><span>Media kit</span><small>bios · photos · logo · facts</small></a>' +
      '<a role="menuitem" href="/media/ssp_sh_logo_2010px.png" download="ssp_sh_logo_2010px.png"><span>Download logo</span><small>PNG · 2010 px</small></a>' +
      '<a role="menuitem" href="/media/simon_spaeti_ssp_2023.jpg" download="simon_spaeti_ssp_2023.jpg"><span>Download portrait</span><small>JPG · 1024 px</small></a>' +
      '<div class="logo-menu__foot">right-click again for the browser menu</div>';
    document.body.appendChild(menu);

    menu.addEventListener('click', function (e) { if (e.target.closest('a')) close(); });
    document.addEventListener('click', function (e) { if (menu && !menu.hidden && !menu.contains(e.target)) close(); });
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') close(); });
    window.addEventListener('scroll', close, { passive: true });
    window.addEventListener('resize', close);
  }

  function open(anchor) {
    if (!menu) build(anchor);
    if (!document.body.contains(menu)) document.body.appendChild(menu); // SPA navigation may have replaced the body
    var r = anchor.getBoundingClientRect();
    menu.hidden = false;
    var w = menu.offsetWidth;
    var left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8));
    menu.style.left = left + 'px';
    menu.style.top = (r.bottom + 6) + 'px';
    var first = menu.querySelector('a');
    if (first) first.focus({ preventScroll: true });
  }

  function close() { if (menu && !menu.hidden) menu.hidden = true; }

  document.addEventListener('contextmenu', function (e) {
    if (menu && !menu.hidden && menu.contains(e.target)) return; // native menu on the menu itself
    var logo = e.target.closest(LOGO);
    if (!logo) { close(); return; }
    e.preventDefault();
    if (menu && !menu.hidden) { close(); return; }
    open(logo);
  });
})();
