from flask import Blueprint, request, jsonify, session, current_app
from .models import User, Team, CoSpace, Comment, TeamMember
from .db import db
from .socketio import socketio
from itsdangerous import URLSafeSerializer
import json, requests
import time
try:
    import eventlet
except Exception:
    eventlet = None

api_bp = Blueprint('api', __name__)


def current_user():
    uid = session.get('user_id')
    if not uid:
        return None
    return User.query.get(uid)


def get_membership(user, team):
    if not user or not team:
        return None
    return TeamMember.query.filter_by(team_id=team.id, user_id=user.id).first()


def require_roles(user, team, roles):
    if team.owner_id == user.id:
        return True
    m = get_membership(user, team)
    if not m:
        return False
    return m.role in roles


@api_bp.route('/me')
def me():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    return jsonify({'id': user.id, 'username': user.username})


@api_bp.route('/teams', methods=['POST'])
def create_team():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    data = request.json or {}
    name = data.get('name')
    team = Team(name=name, owner=user)
    # add TeamMember record with owner role
    tm = TeamMember(team=team, user=user, role='owner')
    team.memberships.append(tm)
    db.session.add(team)
    db.session.commit()
    return jsonify({'id': team.id, 'name': team.name})


@api_bp.route('/teams')
def list_teams():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    # return teams where user is a member
    out = [{'id': t.id, 'name': t.name} for t in user.teams]
    return jsonify(out)


@api_bp.route('/cospaces', methods=['POST'])
def create_cospace():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    data = request.json or {}
    name = data.get('name')
    team_id = data.get('team_id')
    team = Team.query.get(team_id)
    if not team:
        return jsonify({'error': 'team not found'}), 404
    cos = CoSpace(name=name, team=team, description=data.get('description'))
    db.session.add(cos)
    db.session.commit()
    return jsonify({'id': cos.id, 'name': cos.name})


@api_bp.route('/cospaces')
def list_cospaces():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    # return cospaces from teams the user belongs to
    teams = [t.id for t in user.teams]
    cospaces = CoSpace.query.filter(CoSpace.team_id.in_(teams)).all()
    return jsonify([{'id': c.id, 'name': c.name, 'team_id': c.team_id} for c in cospaces])


@api_bp.route('/comments', methods=['POST'])
def post_comment():
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    data = request.json or {}
    cospace = CoSpace.query.get(data.get('cospace_id'))
    if not cospace:
        return jsonify({'error': 'cospace not found'}), 404
    # check team membership & permissions: viewers may not comment
    if not require_roles(user, cospace.team, ['owner', 'admin', 'member']):
        return jsonify({'error': 'insufficient role to post comments'}), 403
    comment = Comment(cospace=cospace, author=user, selector=data.get('selector'), text=data.get('text'), metadata=data.get('metadata'))
    db.session.add(comment)
    db.session.commit()
    # Broadcast via socket
    socketio.emit('new_comment', {
        'id': comment.id,
        'cospace_id': cospace.id,
        'author': user.username,
        'selector': comment.selector,
        'text': comment.text,
        'created_at': comment.created_at.isoformat()
    }, room=f'cospace_{cospace.id}')
    return jsonify({'id': comment.id})


@api_bp.route('/comments/<int:cospace_id>')
def get_comments(cospace_id):
    since_id = request.args.get('since_id', type=int)
    q = Comment.query.filter_by(cospace_id=cospace_id)
    if since_id:
        q = q.filter(Comment.id > since_id)
    comments = q.order_by(Comment.created_at.desc()).all()
    out = []
    for c in comments:
        out.append({'id': c.id, 'author': c.author.username if c.author else None, 'selector': c.selector, 'text': c.text, 'created_at': c.created_at.isoformat()})
    return jsonify(out)


@api_bp.route('/export/github', methods=['POST'])
def export_github():
    """Create a GitHub Gist containing all comments for a given cospace.
    Expects JSON: { cospace_id: int, github_token: str, public: bool }
    """
    data = request.json or {}
    cospace_id = data.get('cospace_id')
    token = data.get('github_token')
    public = data.get('public', False)
    cospace = CoSpace.query.get(cospace_id)
    if not cospace:
        return jsonify({'error': 'cospace not found'}), 404
    comments = Comment.query.filter_by(cospace_id=cospace_id).all()
    body = json.dumps([{'id': c.id, 'author': c.author.username if c.author else None, 'selector': c.selector, 'text': c.text, 'created_at': c.created_at.isoformat()} for c in comments], indent=2)
    gist = {
        'description': f'CoSpace comments export: {cospace.name}',
        'public': bool(public),
        'files': {
            f'cospace_{cospace_id}_comments.json': { 'content': body }
        }
    }
    # if no token provided in request, look up stored encrypted token for current user
    if not token:
        uid = session.get('user_id')
        if not uid:
            return jsonify({'error': 'missing github token and not authenticated'}), 400
        user = User.query.get(uid)
        if not user or not user.github_token_encrypted:
            return jsonify({'error': 'no stored github token; either pass token or connect via /auth/github_export_login'}), 400
        s = URLSafeSerializer(current_app.config['SECRET_KEY'], salt='github-token')
        try:
            token = s.loads(user.github_token_encrypted)
        except Exception as e:
            return jsonify({'error': 'failed to decrypt stored token'}), 500
    headers = {'Authorization': f'token {token}', 'Accept':'application/vnd.github+json'}
    r = requests.post('https://api.github.com/gists', json=gist, headers=headers)
    if r.status_code >= 400:
        return jsonify({'error': 'github API error', 'details': r.json()}), r.status_code
    return jsonify(r.json())


@api_bp.route('/teams/<int:team_id>/members')
def get_team_members(team_id):
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    team = Team.query.get(team_id)
    if not team:
        return jsonify({'error': 'team not found'}), 404
    # membership required to view members
    if not get_membership(user, team) and team.owner_id != user.id:
        return jsonify({'error': 'not a team member'}), 403
    out = []
    for m in team.memberships:
        out.append({'user_id': m.user.id, 'username': m.user.username, 'role': m.role})
    return jsonify(out)


@api_bp.route('/teams/<int:team_id>/members', methods=['POST'])
def add_team_member(team_id):
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    team = Team.query.get(team_id)
    if not team:
        return jsonify({'error': 'team not found'}), 404
    # only owner or admin can add members
    if not require_roles(user, team, ['owner', 'admin']):
        return jsonify({'error': 'only owner/admin can add members'}), 403
    data = request.json or {}
    github_username = data.get('github_username')
    role = data.get('role', 'member')
    if not github_username:
        return jsonify({'error': 'missing github_username'}), 400
    u = User.query.filter_by(username=github_username).first()
    if not u:
        return jsonify({'error': 'user not found'}), 404
    existing = TeamMember.query.filter_by(team_id=team.id, user_id=u.id).first()
    if existing:
        return jsonify({'error': 'user already a member'}), 400
    tm = TeamMember(team=team, user=u, role=role)
    db.session.add(tm)
    db.session.commit()
    return jsonify({'ok': True})


@api_bp.route('/teams/<int:team_id>/members/<int:user_id>', methods=['PUT'])
def set_team_member_role(team_id, user_id):
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    team = Team.query.get(team_id)
    if not team:
        return jsonify({'error': 'team not found'}), 404
    # only owner or admin can change roles, but only owner can assign 'owner'
    if not require_roles(user, team, ['owner', 'admin']):
        return jsonify({'error': 'not allowed'}), 403
    data = request.json or {}
    role = data.get('role')
    if not role:
        return jsonify({'error': 'missing role'}), 400
    if role == 'owner' and team.owner_id != user.id:
        return jsonify({'error': 'only owner can transfer ownership'}), 403
    tm = TeamMember.query.filter_by(team_id=team.id, user_id=user_id).first()
    if not tm:
        return jsonify({'error': 'team member not found'}), 404
    tm.role = role
    # if transferring ownership, update team.owner_id
    if role == 'owner':
        team.owner_id = user_id
    db.session.commit()
    return jsonify({'ok': True})


@api_bp.route('/teams/<int:team_id>/members/<int:user_id>', methods=['DELETE'])
def remove_team_member(team_id, user_id):
    user = current_user()
    if not user:
        return jsonify({'error': 'not authenticated'}), 401
    team = Team.query.get(team_id)
    if not team:
        return jsonify({'error': 'team not found'}), 404
    if not require_roles(user, team, ['owner', 'admin']):
        return jsonify({'error': 'only owner/admin can remove members'}), 403
    tm = TeamMember.query.filter_by(team_id=team.id, user_id=user_id).first()
    if tm:
        db.session.delete(tm)
        db.session.commit()
    return jsonify({'ok': True})


@api_bp.route('/comments/longpoll/<int:cospace_id>')
def longpoll_comments(cospace_id):
    """Long-poll for new comments since a given id. Returns immediately if new comments are present, otherwise waits up to timeout seconds."""
    since_id = request.args.get('since_id', type=int, default=0)
    timeout = request.args.get('timeout', type=int, default=25)
    deadline = time.time() + timeout
    while time.time() < deadline:
        q = Comment.query.filter(Comment.cospace_id == cospace_id)
        if since_id:
            q = q.filter(Comment.id > since_id)
        new_comments = q.order_by(Comment.created_at.asc()).all()
        if new_comments:
            out = []
            for c in new_comments:
                out.append({'id': c.id, 'author': c.author.username if c.author else None, 'selector': c.selector, 'text': c.text, 'created_at': c.created_at.isoformat()})
            return jsonify(out)
        # sleep for a short period; prefer eventlet.sleep when available to avoid blocking
        if eventlet:
            eventlet.sleep(1)
        else:
            time.sleep(1)
    # timeout, return empty
    return jsonify([])
