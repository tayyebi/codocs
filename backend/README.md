# Codocs Backend

This is a minimal Flask backend for the Codocs browser extension demo.

## Setup

1. Create a virtualenv and install requirements

```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

2. Configure environment variables in a `.env` file (optional)

```
SECRET_KEY=dev
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
DATABASE_URL=sqlite:///co_space.db
```

3. Run the app

```bash
python app.py
```

The server opens at http://127.0.0.1:5000

Endpoints:
- `GET /api/me`
- `POST /api/teams`
- `POST /api/cospaces`
- `POST /api/comments`
- `GET /api/comments/<cospace_id>`

SocketIO events:
- `join_cospace` (join room to get live comments)
- `new_comment` (emitted on new comments)

## Running tests

Install testing deps and run pytest:

```bash
pip install -r requirements.txt
pytest -q
```

