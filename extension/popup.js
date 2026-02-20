/**
 * popup.js – logic for the Codocs extension popup.
 */

// ── State ─────────────────────────────────────────────────────────────────
let currentTab = null;
let selectedSelector = null;

// ── Storage helpers ───────────────────────────────────────────────────────
const store = {
  get: (keys) => new Promise((res) => chrome.storage.local.get(keys, res)),
  set: (obj) => new Promise((res) => chrome.storage.local.set(obj, res)),
  remove: (keys) => new Promise((res) => chrome.storage.local.remove(keys, res)),
};

async function api(path, options = {}) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type: 'API_FETCH', path, options }, resolve);
  });
}

// ── Init ──────────────────────────────────────────────────────────────────
async function init() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  currentTab = tab;

  const { token, username } = await store.get(['token', 'username']);

  if (token && username) {
    showNotesView(username);
  } else {
    const { serverUrl } = await store.get(['serverUrl']);
    document.getElementById('inp-server').value = serverUrl || 'http://localhost:8080';
    document.getElementById('view-auth').style.display = 'block';
  }
}

// ── Auth view ─────────────────────────────────────────────────────────────
document.getElementById('btn-login').addEventListener('click', async () => {
  await doAuth('login');
});

document.getElementById('btn-register').addEventListener('click', async () => {
  await doAuth('register');
});

async function doAuth(mode) {
  const serverUrl = document.getElementById('inp-server').value.trim().replace(/\/$/, '');
  const username = document.getElementById('inp-username').value.trim();
  const password = document.getElementById('inp-password').value;
  const msg = document.getElementById('auth-msg');
  msg.textContent = '';

  if (!serverUrl || !username || !password) {
    msg.className = 'error';
    msg.textContent = 'All fields are required.';
    return;
  }

  await store.set({ serverUrl });

  const endpoint = mode === 'login' ? '/api/auth/login' : '/api/auth/register';
  const resp = await api(endpoint, {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });

  if (!resp || !resp.ok) {
    msg.className = 'error';
    msg.textContent = resp?.body?.error || 'Authentication failed.';
    return;
  }

  await store.set({ token: resp.body.token, username: resp.body.username });
  document.getElementById('view-auth').style.display = 'none';
  showNotesView(resp.body.username);
}

// ── Notes view ────────────────────────────────────────────────────────────
async function showNotesView(username) {
  document.getElementById('hdr-user').textContent = username;
  document.getElementById('btn-logout').style.display = 'inline-block';
  document.getElementById('view-notes').style.display = 'block';
  await loadNotes();
}

async function loadNotes() {
  const list = document.getElementById('note-list');
  list.innerHTML = '<div style="color:#64748b;padding:6px 0">Loading…</div>';

  if (!currentTab?.url) {
    list.innerHTML = '<div style="color:#64748b;padding:6px 0">No page URL.</div>';
    return;
  }

  const resp = await api(`/api/notes?url=${encodeURIComponent(currentTab.url)}`);
  if (!resp || !resp.ok) {
    list.innerHTML = '<div class="error">Failed to load notes.</div>';
    return;
  }

  const notes = resp.body || [];
  if (!notes.length) {
    list.innerHTML = '<div style="color:#64748b;padding:6px 0">No notes yet on this page.</div>';
    return;
  }

  list.innerHTML = notes.map((n) => `
    <div class="note-item">
      <div class="meta">${escHtml(n.author)} • ${new Date(n.created_at).toLocaleString()}
        ${n.is_local ? '' : ' 🌐'}
      </div>
      <div class="body">${escHtml(n.body)}</div>
    </div>
  `).join('');
}

// ── Pick element ──────────────────────────────────────────────────────────
document.getElementById('btn-pick').addEventListener('click', async () => {
  await chrome.tabs.sendMessage(currentTab.id, { type: 'CDX_PICK_MODE' });
  window.close(); // close popup so user can interact with the page
});

// Listen for anchor from content script (via storage, since popup closed)
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'local' && changes.pendingSelector) {
    selectedSelector = changes.pendingSelector.newValue;
    const el = document.getElementById('note-anchor');
    if (el) el.textContent = `Element: ${selectedSelector}`;
  }
});

window.addEventListener('message', (e) => {
  if (e.data?.type === 'CDX_ANCHOR_PICKED') {
    selectedSelector = e.data.selector;
    document.getElementById('note-anchor').textContent = `Element: ${selectedSelector}`;
  }
});

// ── Submit note ───────────────────────────────────────────────────────────
document.getElementById('btn-submit').addEventListener('click', async () => {
  const body = document.getElementById('inp-body').value.trim();
  const msg = document.getElementById('note-msg');
  msg.textContent = '';

  if (!body) {
    msg.className = 'error'; msg.textContent = 'Note text is required.'; return;
  }
  if (!currentTab?.url) {
    msg.className = 'error'; msg.textContent = 'No active page URL.'; return;
  }

  const payload = { page_url: currentTab.url, body };
  if (selectedSelector) payload.selector = selectedSelector;

  const resp = await api('/api/notes', {
    method: 'POST',
    body: JSON.stringify(payload),
  });

  if (!resp || !resp.ok) {
    msg.className = 'error';
    msg.textContent = resp?.body?.error || 'Failed to post note.';
    return;
  }

  msg.className = 'success'; msg.textContent = 'Note posted!';
  document.getElementById('inp-body').value = '';
  selectedSelector = null;
  document.getElementById('note-anchor').textContent = 'No element selected';

  // reload notes in the page and popup
  chrome.tabs.sendMessage(currentTab.id, { type: 'CDX_RELOAD_NOTES' });
  setTimeout(loadNotes, 300);
});

// ── Logout ────────────────────────────────────────────────────────────────
document.getElementById('btn-logout').addEventListener('click', async () => {
  await store.remove(['token', 'username']);
  location.reload();
});

// ── Utility ───────────────────────────────────────────────────────────────
function escHtml(s) {
  return String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

init();
