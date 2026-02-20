/**
 * background.js – service worker for Codocs extension.
 * Manages API communication and relays messages between popup and content scripts.
 */

const DEFAULT_SERVER = 'http://localhost:8080';

function getServerUrl() {
  return new Promise((resolve) => {
    chrome.storage.local.get(['serverUrl'], (res) => {
      resolve(res.serverUrl || DEFAULT_SERVER);
    });
  });
}

function getToken() {
  return new Promise((resolve) => {
    chrome.storage.local.get(['token'], (res) => resolve(res.token || null));
  });
}

async function apiFetch(path, options = {}) {
  const base = await getServerUrl();
  const token = await getToken();
  const headers = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(options.headers || {}),
  };
  const resp = await fetch(`${base}${path}`, { ...options, headers });
  return resp;
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === 'API_FETCH') {
    apiFetch(msg.path, msg.options)
      .then(async (resp) => {
        const body = await resp.json().catch(() => ({}));
        sendResponse({ ok: resp.ok, status: resp.status, body });
      })
      .catch((err) => sendResponse({ ok: false, error: err.message }));
    return true; // keep channel open
  }

  if (msg.type === 'LOAD_NOTES') {
    apiFetch(`/api/notes?url=${encodeURIComponent(msg.url)}`)
      .then(async (resp) => {
        const body = await resp.json().catch(() => []);
        sendResponse({ ok: resp.ok, notes: body });
      })
      .catch((err) => sendResponse({ ok: false, notes: [], error: err.message }));
    return true;
  }
});
