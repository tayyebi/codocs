/**
 * content.js – injected into every page.
 * Fetches and renders sticky notes; handles element-picking for note creation.
 */
(function () {
  if (window.__codocs_loaded) return;
  window.__codocs_loaded = true;

  // ── Styles ────────────────────────────────────────────────────────────────
  const style = document.createElement('style');
  style.textContent = `
    .cdx-badge {
      position: absolute;
      z-index: 2147483647;
      background: #fbbf24;
      color: #1f2937;
      font: bold 11px/1 sans-serif;
      padding: 3px 7px;
      border-radius: 12px;
      cursor: pointer;
      box-shadow: 0 2px 6px rgba(0,0,0,.35);
      white-space: nowrap;
      max-width: 220px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .cdx-badge:hover { background: #f59e0b; }
    .cdx-pick-outline { outline: 3px solid #3b82f6 !important; cursor: crosshair !important; }
    .cdx-toast {
      position: fixed; bottom: 24px; right: 24px; z-index: 2147483647;
      background: #1e293b; color: #f1f5f9; padding: 10px 16px;
      border-radius: 8px; font: 13px/1.4 sans-serif;
      box-shadow: 0 4px 14px rgba(0,0,0,.4); max-width: 320px;
      animation: cdx-slide-in .2s ease;
    }
    @keyframes cdx-slide-in {
      from { opacity: 0; transform: translateY(10px); }
      to   { opacity: 1; transform: translateY(0); }
    }
  `;
  document.head.appendChild(style);

  // ── Helpers ───────────────────────────────────────────────────────────────
  function computeSelector(el) {
    const path = [];
    while (el && el.nodeType === Node.ELEMENT_NODE) {
      let seg = el.nodeName.toLowerCase();
      if (el.id) { seg += '#' + el.id; path.unshift(seg); break; }
      let nth = 1, sib = el;
      while ((sib = sib.previousElementSibling)) { if (sib.nodeName === el.nodeName) nth++; }
      if (nth > 1) seg += `:nth-of-type(${nth})`;
      path.unshift(seg);
      el = el.parentElement;
    }
    return path.join(' > ');
  }

  function toast(msg, duration = 5000) {
    const t = document.createElement('div');
    t.className = 'cdx-toast';
    t.textContent = msg;
    document.body.appendChild(t);
    setTimeout(() => t.remove(), duration);
  }

  // ── Note badges ───────────────────────────────────────────────────────────
  const badges = [];

  function placeBadge(note) {
    const el = note.selector ? document.querySelector(note.selector) : null;
    const badge = document.createElement('div');
    badge.className = 'cdx-badge';
    badge.title = `${note.author}: ${note.body}`;
    badge.textContent = `💬 ${note.author}`;
    badge.addEventListener('click', () => {
      toast(`${note.author}: ${note.body}`, 8000);
    });
    document.body.appendChild(badge);
    const entry = { el, badge, note };
    badges.push(entry);
    updatePositions();
  }

  function updatePositions() {
    badges.forEach(({ el, badge }) => {
      if (!el) {
        badge.style.bottom = '80px';
        badge.style.right = '24px';
        badge.style.position = 'fixed';
        return;
      }
      try {
        const r = el.getBoundingClientRect();
        badge.style.top = `${window.scrollY + r.top}px`;
        badge.style.left = `${window.scrollX + r.right - 8}px`;
      } catch (_) { badge.remove(); }
    });
  }

  window.addEventListener('scroll', updatePositions, { passive: true });
  window.addEventListener('resize', updatePositions, { passive: true });

  // ── Load notes for this page ──────────────────────────────────────────────
  function loadNotes() {
    chrome.runtime.sendMessage(
      { type: 'LOAD_NOTES', url: location.href },
      (resp) => {
        if (!resp || !resp.ok) return;
        (resp.notes || []).forEach(placeBadge);
      }
    );
  }

  loadNotes();

  // ── Element-picking mode ──────────────────────────────────────────────────
  let pickMode = false;
  let hovered = null;

  function enterPickMode() {
    pickMode = true;
    document.addEventListener('mouseover', onOver, true);
    document.addEventListener('click', onPick, true);
    toast('Click an element to anchor your note…');
  }

  function exitPickMode() {
    pickMode = false;
    document.removeEventListener('mouseover', onOver, true);
    document.removeEventListener('click', onPick, true);
    if (hovered) { hovered.classList.remove('cdx-pick-outline'); hovered = null; }
  }

  function onOver(e) {
    if (hovered && hovered !== e.target) hovered.classList.remove('cdx-pick-outline');
    hovered = e.target;
    hovered.classList.add('cdx-pick-outline');
  }

  function onPick(e) {
    if (!pickMode) return;
    e.preventDefault(); e.stopPropagation();
    const selector = computeSelector(e.target);
    exitPickMode();
    window.postMessage({ type: 'CDX_ANCHOR_PICKED', selector }, '*');
  }

  // ── Messages from popup ───────────────────────────────────────────────────
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === 'CDX_PICK_MODE') enterPickMode();
    if (msg.type === 'CDX_RELOAD_NOTES') {
      badges.forEach(({ badge }) => badge.remove());
      badges.length = 0;
      loadNotes();
    }
  });
})();
