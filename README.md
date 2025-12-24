# Codocs (Comments for the web) 🚀

A minimal demo implementing a browser extension (Chrome/Firefox) and a Flask backend that lets teams collaborate and comment on page elements.

Features:
- Annotate (anchor comments) to page elements via a selector
- Dashboard to browse comments
- Teams and Codocs (basic models)
- Real-time updates via Socket.IO (room-based)
- Export comments to GitHub Gist

## Quick start

1. Backend

```bash
cd backend
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
# (optional) set environment variables in .env
python app.py
```

2. Load the extension

- Open Chrome: chrome://extensions
- Enable Developer mode
- Load unpacked extension -> select `extension/` folder

3. Use the extension

- Click extension icon to open popup
- Sign in (opens GitHub OAuth in server if configured)
- Create a team / codoc via API or UI
- Enable comment mode and click an element to anchor a comment
- Open the Dashboard (Options) to see all comments
- Export to GitHub Gist from the Dashboard (either enter a token or connect via `Connect GitHub Gist` to store a token securely on your account)

## API Endpoints

- `POST /api/teams` — create team
- `GET /api/teams` — list teams (user)
- `GET /api/teams/<id>/members` — list team members and roles
- `POST /api/cospaces` — create a cospace
- `POST /api/comments` — post anchored comment (checks team membership)
- `GET /api/comments/<cospace_id>` — list comments
- `POST /api/export/github` — export cospace comments to a GitHub Gist

## Notes & Next steps
- This is a minimal demo to get you started. Security (CSRF, permissions), better selector generation, team roles & access checks, and robust client state should be added for production.
- Consider storing GitHub tokens securely (do not store plain tokens without precautions).

## Team Roles
- Role-based access: Owner/Admin/Member/Viewer
- Owner can manage team, transfer ownership, and set members' roles.
- Admins can add/remove members and change roles (not ownership), members can comment, viewers read-only.

New features in this update:
- Polished members management UI (modal with inline role dropdowns) in the extension popup.
- In-page real-time notifications and badges: the content script polls new comments for the active CoSpace and shows toast notifications and spatial badges on matched elements.
- GitHub Gist export: you can now connect your GitHub account via `/auth/github_export_login` (button in dashboard) to store an encrypted Gist token server-side for seamless exports. Note: tokens are encrypted with the app SECRET_KEY; for production use a secure vault.

Long polling and reliability
- The content script now uses a long-poll endpoint `/api/comments/longpoll/<cospace_id>?since_id=<id>` to receive new comments with a persistent request (timeout 25s). This reduces periodic polling churn and delivers comments as soon as they become available.

UX improvements
- Members modal now supports search and sort, and changing roles/removing members uses a confirmation dialog to avoid mistakes. The popup UI also persists the active CoSpace so in-page notifications know where to poll.

