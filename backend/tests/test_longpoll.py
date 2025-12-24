import threading
import time
import pytest
from backend.app import create_app
from backend.db import db
from backend.models import User, Team, CoSpace, Comment, TeamMember


def test_longpoll_returns_immediately_if_new_comment(client, app):
    with app.app_context():
        # create user, team, cospace
        user = User(username='u1', github_id='1')
        db.session.add(user)
        team = Team(name='t1', owner=user)
        db.session.add(team)
        tm = TeamMember(team=team, user=user, role='owner')
        db.session.add(tm)
        cos = CoSpace(name='c1', team=team)
        db.session.add(cos)
        db.session.commit()
        # add a comment
        c = Comment(cospace=cos, author=user, selector='body', text='hello')
        db.session.add(c)
        db.session.commit()
        # call longpoll with since_id less than comment id
        res = client.get(f'/api/comments/longpoll/{cos.id}?since_id=0&timeout=1')
        assert res.status_code == 200
        data = res.get_json()
        assert isinstance(data, list)
        assert len(data) >= 1
        assert any(x['text'] == 'hello' for x in data)


def test_longpoll_waits_and_returns_when_comment_created(client, app):
    with app.app_context():
        user = User(username='u2', github_id='2')
        db.session.add(user)
        team = Team(name='t2', owner=user)
        db.session.add(team)
        cos = CoSpace(name='c2', team=team)
        db.session.add(cos)
        db.session.commit()
        cos_id = cos.id

    # create a thread to insert a comment after a short delay
    def insert_comment():
        time.sleep(0.5)
        with app.app_context():
            u = User.query.filter_by(username='u2').first()
            c = Comment(cospace_id=cos_id, author=u, selector='body', text='delayed')
            db.session.add(c)
            db.session.commit()

    t = threading.Thread(target=insert_comment)
    t.start()
    res = client.get(f'/api/comments/longpoll/{cos_id}?since_id=0&timeout=5')
    assert res.status_code == 200
    data = res.get_json()
    assert isinstance(data, list)
    assert any(x['text'] == 'delayed' for x in data)
    t.join()
