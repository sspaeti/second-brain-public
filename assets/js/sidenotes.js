/* ssp.sh/brain sidenotes — companion to assets/styles/sidenotes.scss.
 *
 * On wide viewports the CSS pulls every callout (blockquote.<type>-callout)
 * and footnote into the note column at the card's right edge. This script
 * does what CSS can't:
 *   1. drops a marker (.sn-home) where each callout sat, so the hairline stub
 *      can be drawn there and the note knows its source paragraph;
 *   2. builds one margin note (.sn-fn) per footnote reference from the
 *      bottom footnote list;
 *   3. sets each note's `top` (aligned with the paragraph above the callout,
 *      or the line holding the footnote reference) and pushes overlapping
 *      notes down in document order;
 *   4. links hover between note, stub and source paragraph (delegated, so it
 *      survives re-initialisation).
 * The Million SPA router swaps .singlePage on navigation, so init() is
 * idempotent: it tears down its own markup first and re-runs on
 * `million:navigate`. Below the breakpoint it only clears inline `top`s.
 */
(function () {
  'use strict';

  var BP = 1420; // keep in sync with $sn-bp in sidenotes.scss
  var mq = window.matchMedia('(min-width: ' + BP + 'px)');
  var notes = [];      // { el, anchorEl, container }
  var observers = [];
  var raf = window.requestAnimationFrame || function (f) { setTimeout(f, 16); };

  function isBlock(el) {
    return !!el && /^(P|UL|OL|FIGURE|DIV|BLOCKQUOTE|TABLE|PRE|H[1-6])$/.test(el.tagName) &&
      !/-callout(\s|$)/.test(el.className) && !el.classList.contains('sn-home');
  }

  function typeOf(el) {
    var m = (' ' + el.className + ' ').match(/\s([a-z]+)-callout\s/);
    return m ? m[1] : 'note';
  }

  function removeAll(sel) {
    var els = document.querySelectorAll(sel);
    for (var i = 0; i < els.length; i++) els[i].parentNode.removeChild(els[i]);
  }

  function reset() {
    for (var o = 0; o < observers.length; o++) observers[o].disconnect();
    observers = [];
    notes = [];
    removeAll('.sn-home, aside.sn-fn');
    var stale = document.querySelectorAll('.sn-side, .sn-fn-ref, [data-sn-src], .sn-hot');
    for (var i = 0; i < stale.length; i++) {
      var el = stale[i];
      el.classList.remove('sn-side', 'sn-fn-ref', 'sn-hot');
      el.removeAttribute('data-sn-type');
      el.removeAttribute('data-sn-id');
      el.removeAttribute('data-sn-src');
      if (el.style) el.style.top = '';
    }
  }

  function init() {
    reset();
    var root = document.querySelector('.singlePage.sn-on');
    if (!root) return;
    var article = root.querySelector('article');
    if (!article) return;
    var id = 0;

    /* 1. callouts */
    var callouts = article.querySelectorAll('blockquote[class*="-callout"]');
    for (var a = 0; a < callouts.length; a++) {
      var co = callouts[a];
      var home = document.createElement('span');
      home.className = 'sn-home';
      co.parentNode.insertBefore(home, co);
      co.setAttribute('data-sn-type', typeOf(co));
      co.classList.add('sn-side');
      co.setAttribute('data-sn-id', ++id);
      home.setAttribute('data-sn-id', id);

      var prev = home.previousElementSibling;
      while (prev && (prev.classList.contains('sn-home') || /-callout(\s|$)/.test(prev.className))) {
        prev = prev.previousElementSibling;
      }
      var src = isBlock(prev) ? prev : null;
      if (src) src.setAttribute('data-sn-src', ((src.getAttribute('data-sn-src') || '') + ' ' + id).trim());
      notes.push({ el: co, anchorEl: src || home, stubEl: home, container: article });
    }

    // stubs of consecutive callouts share one row; stub inherits the accent
    var stubIndex = 0, lastStub = null;
    var stubs = article.querySelectorAll('.sn-home');
    for (var s = 0; s < stubs.length; s++) {
      var stub = stubs[s], e = stub.previousElementSibling;
      while (e && /-callout(\s|$)/.test(e.className)) e = e.previousElementSibling;
      stubIndex = (e && e === lastStub) ? stubIndex + 1 : 0;
      stub.setAttribute('data-sn-i', stubIndex);
      stub.style.setProperty('--sn-i', stubIndex);
      lastStub = stub;
    }
    for (var n = 0; n < notes.length; n++) {
      var c = getComputedStyle(notes[n].el).getPropertyValue('--sn-c');
      if (c) notes[n].stubEl.style.setProperty('--sn-c', c.trim());
    }

    /* 2. footnotes → margin notes */
    var refs = article.querySelectorAll('sup[id^="fnref:"]');
    for (var r = 0; r < refs.length; r++) {
      var sup = refs[r];
      var num = sup.id.split(':')[1];
      var li = document.getElementById('fn:' + num);
      if (!li) continue;
      var clone = li.cloneNode(true);
      var back = clone.querySelectorAll('.footnote-backref');
      for (var b = 0; b < back.length; b++) back[b].parentNode.removeChild(back[b]);
      var aside = document.createElement('aside');
      aside.className = 'sn-side sn-fn';
      aside.innerHTML = '<span class="sn-fn-num">' + num + '</span>' + clone.innerHTML;
      aside.setAttribute('data-sn-id', ++id);
      sup.classList.add('sn-fn-ref');
      sup.setAttribute('data-sn-src', id);
      sup.parentNode.insertBefore(aside, sup.nextSibling);
      notes.push({ el: aside, anchorEl: sup, stubEl: null, container: article });
    }

    if (!notes.length) return;
    layout();
    if (window.ResizeObserver) {
      var ro = new ResizeObserver(schedule);
      ro.observe(article);
      observers.push(ro);
    }
  }

  /* 3. vertical placement + overlap resolution */
  function layout() {
    if (!mq.matches) {
      for (var i = 0; i < notes.length; i++) notes[i].el.style.top = '';
      return;
    }
    var placed = [];
    for (var j = 0; j < notes.length; j++) {
      var note = notes[j];
      var top = note.anchorEl.getBoundingClientRect().top - note.container.getBoundingClientRect().top;
      if (note.anchorEl.tagName === 'SUP') top -= 2;
      placed.push({ el: note.el, top: top });
    }
    placed.sort(function (x, y) { return x.top - y.top; });
    var prevBottom = -Infinity;
    for (var k = 0; k < placed.length; k++) {
      var t = Math.max(placed[k].top, prevBottom + 14);
      placed[k].el.style.top = t + 'px';
      prevBottom = t + placed[k].el.offsetHeight;
    }
  }
  function schedule() { raf(layout); }

  /* 4. hover linking (delegated): note/stub carry data-sn-id, sources carry
        data-sn-src="id id"; all members of a group light up together */
  function groupOf(target) {
    var el = target.closest ? target.closest('[data-sn-id], [data-sn-src]') : null;
    if (!el) return null;
    var ids = el.getAttribute('data-sn-id') ? [el.getAttribute('data-sn-id')] : el.getAttribute('data-sn-src').split(/\s+/);
    var members = [];
    for (var i = 0; i < ids.length; i++) {
      var found = document.querySelectorAll('[data-sn-id="' + ids[i] + '"], [data-sn-src~="' + ids[i] + '"]');
      for (var f = 0; f < found.length; f++) members.push(found[f]);
    }
    return members;
  }
  function setHot(ev, on) {
    if (!mq.matches) return;
    var members = groupOf(ev.target);
    if (!members) return;
    for (var i = 0; i < members.length; i++) members[i].classList[on ? 'add' : 'remove']('sn-hot');
  }
  document.addEventListener('mouseover', function (ev) { setHot(ev, true); });
  document.addEventListener('mouseout', function (ev) { setHot(ev, false); });

  // callouts.js toggles collapse on click; keep margin notes open (links still work)
  document.addEventListener('click', function (ev) {
    if (!mq.matches) return;
    var note = ev.target.closest ? ev.target.closest('blockquote.sn-side') : null;
    if (note && !ev.target.closest('a')) ev.stopPropagation();
  }, true);

  /* run */
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
  window.addEventListener('load', schedule);
  window.addEventListener('resize', schedule);
  if (mq.addEventListener) mq.addEventListener('change', schedule);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(schedule);
  window.addEventListener('million:navigate', function () { setTimeout(init, 50); });
})();
