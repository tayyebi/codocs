import pytest
from backend.app import create_app
from backend.db import db
from backend.models import User, Team, CoSpace, TeamMember


def login(client, user_id):
    with client.session_transaction() as sess:
        sess['user_id'] = user_id


def test_owner_can_add_and_remove_member(client, app):
    with app.app_context():
        owner = User(username='owner', github_id='10')
        alice = User(username='alice', github_id='11')
        db.session.add_all([owner, alice])
        team = Team(name='teamA', owner=owner)
        tm = TeamMember(team=team, user=owner, role='owner')
        db.session.add_all([team, tm])
        db.session.commit()
        team_id = team.id
        owner_id = owner.id

    login(client, owner_id)
    # add alice
    res = client.post(f'/api/teams/{team_id}/members', json={'github_username': 'alice', 'role': 'member'})
    assert res.status_code == 200
    j = res.get_json()
    assert j.get('ok')

    # list members
    res = client.get(f'/api/teams/{team_id}/members')
    assert res.status_code == 200
    members = res.get_json()
    assert any(m['username']=='alice' for m in members)

    # set role to admin
    # find alice id
    alice_record = next(m for m in members if m['username']=='alice')
    res = client.put(f"/api/teams/{team_id}/members/{alice_record['user_id']}", json={'role':'admin'})
    assert res.status_code == 200

    # remove alice
    res = client.delete(f"/api/teams/{team_id}/members/{alice_record['user_id']}")
    assert res.status_code == 200
    res = client.get(f'/api/teams/{team_id}/members')
    members = res.get_json()
    assert not any(m['username']=='alice' for m in members)


def test_non_admin_cannot_add_member(client, app):
    with app.app_context():
        owner = User(username='owner2', github_id='20')
        bob = User(username='bob', github_id='21')
        carol = User(username='carol', github_id='22')
        db.session.add_all([owner, bob, carol])
        team = Team(name='teamB', owner=owner)
        tm_owner = TeamMember(team=team, user=owner, role='owner')
        tm_bob = TeamMember(team=team, user=bob, role='member')
        db.session.add_all([team, tm_owner, tm_bob])
        db.session.commit()
        team_id = team.id
        bob_id = bob.id

    login(client, bob_id)
    res = client.post(f'/api/teams/{team_id}/members', json={'github_username':'carol','role':'member'})
    assert res.status_code == 403


def test_admin_can_add_member(client, app):
    with app.app_context():
        owner = User(username='owner3', github_id='30')
        admin = User(username='admin', github_id='31')
        carol = User(username='carol2', github_id='32')
        db.session.add_all([owner, admin, carol])
        team = Team(name='teamC', owner=owner)
        tm_owner = TeamMember(team=team, user=owner, role='owner')
        tm_admin = TeamMember(team=team, user=admin, role='admin')
        db.session.add_all([team, tm_owner, tm_admin])
        db.session.commit()
        team_id = team.id
        admin_id = admin.id

    login(client, admin_id)
    res = client.post(f'/api/teams/{team_id}/members', json={'github_username':'carol2','role':'member'})
    assert res.status_code == 200
    j = res.get_json(); assert j.get('ok')
