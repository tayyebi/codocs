/**
 * admin.js – logic for the Codocs admin page.
 */

// ── Storage helpers ───────────────────────────────────────────────────────
const store = {
  get: (keys) => new Promise((res) => chrome.storage.local.get(keys, res)),
  set: (obj) => new Promise((res) => chrome.storage.local.set(obj, res)),
  remove: (keys) => new Promise((res) => chrome.storage.local.remove(keys, res)),
};

let serverUrl = 'http://localhost:8080';
let token = null;

async function apiFetch(path, options = {}) {
  const headers = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(options.headers || {}),
  };
  const resp = await fetch(`${serverUrl}${path}`, { ...options, headers });
  const body = await resp.json().catch(() => ({}));
  return { ok: resp.ok, status: resp.status, body };
}

// ── Init ──────────────────────────────────────────────────────────────────
async function init() {
  const stored = await store.get(['serverUrl', 'token', 'username']);
  if (stored.serverUrl) {
    serverUrl = stored.serverUrl;
    document.getElementById('inp-server').value = serverUrl;
  }
  if (stored.token) {
    token = stored.token;
    showAdminPanel(stored.username || 'admin');
  }
}

// ── Login ─────────────────────────────────────────────────────────────────
document.getElementById('btn-login').addEventListener('click', async () => {
  const url = document.getElementById('inp-server').value.trim().replace(/\/$/, '');
  const username = document.getElementById('inp-username').value.trim();
  const password = document.getElementById('inp-password').value;
  const msg = document.getElementById('login-msg');
  msg.textContent = '';

  if (!url || !username || !password) {
    msg.className = 'error'; msg.textContent = 'All fields are required.'; return;
  }

  serverUrl = url;
  await store.set({ serverUrl });

  const resp = await apiFetch('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  });

  if (!resp.ok) {
    msg.className = 'error';
    msg.textContent = resp.body?.error || 'Authentication failed.';
    return;
  }

  token = resp.body.token;
  await store.set({ token, username: resp.body.username });
  document.getElementById('login-section').style.display = 'none';
  showAdminPanel(resp.body.username);
});

// ── Admin panel ───────────────────────────────────────────────────────────
function showAdminPanel(username) {
  document.getElementById('hdr-user').textContent = username;
  document.getElementById('btn-logout').style.display = 'inline-block';
  document.getElementById('admin-section').style.display = 'block';
  document.getElementById('login-section').style.display = 'none';
  document.getElementById('server-info').textContent = serverUrl;
  loadUsers();
}

document.getElementById('btn-refresh-users').addEventListener('click', loadUsers);

async function loadUsers() {
  const tbody = document.getElementById('users-tbody');
  const msg = document.getElementById('users-msg');
  msg.textContent = '';
  tbody.innerHTML = '<tr><td colspan="5" style="color:#64748b">Loading…</td></tr>';

  const resp = await apiFetch('/api/admin/users');
  if (!resp.ok) {
    tbody.innerHTML = '';
    msg.className = 'error';
    msg.textContent = resp.body?.error || 'Failed to load users. Are you an admin?';
    return;
  }

  const users = resp.body || [];
  if (!users.length) {
    tbody.innerHTML = '<tr><td colspan="5" style="color:#64748b">No users found.</td></tr>';
    return;
  }

  tbody.innerHTML = users.map((u) => `
    <tr>
      <td>${escHtml(u.username)}</td>
      <td><a href="${escHtml(u.actor_url)}" style="color:#60a5fa;font-size:11px" target="_blank">${escHtml(u.actor_url)}</a></td>
      <td>
        <span class="badge ${u.is_admin ? 'badge-admin' : ''}">${u.is_admin ? 'admin' : 'user'}</span>
      </td>
      <td style="color:#64748b;font-size:11px">${new Date(u.created_at).toLocaleDateString()}</td>
      <td>
        ${!u.is_admin ? `<button class="btn btn-primary" onclick="promoteUser('${escHtml(u.id)}')" style="margin-right:4px;font-size:11px">Promote</button>` : ''}
        <button class="btn btn-danger" onclick="deleteUser('${escHtml(u.id)}', '${escHtml(u.username)}')" style="font-size:11px">Delete</button>
      </td>
    </tr>
  `).join('');
}

async function deleteUser(id, username) {
  const msg = document.getElementById('users-msg');
  if (!confirm(`Delete user "${username}"? This will also delete all their notes.`)) return;
  const resp = await apiFetch(`/api/admin/users/${id}`, { method: 'DELETE' });
  if (!resp.ok) {
    msg.className = 'error'; msg.textContent = resp.body?.error || 'Delete failed.'; return;
  }
  msg.className = 'success'; msg.textContent = `User "${username}" deleted.`;
  loadUsers();
}

async function promoteUser(id) {
  const msg = document.getElementById('users-msg');
  const resp = await apiFetch(`/api/admin/users/${id}/promote`, { method: 'POST' });
  if (!resp.ok) {
    msg.className = 'error'; msg.textContent = resp.body?.error || 'Promote failed.'; return;
  }
  msg.className = 'success'; msg.textContent = 'User promoted to admin.';
  loadUsers();
}

// ── Logout ────────────────────────────────────────────────────────────────
document.getElementById('btn-logout').addEventListener('click', async () => {
  token = null;
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
