from .db import db
from datetime import datetime


class User(db.Model):
    __tablename__ = 'user'
    id = db.Column(db.Integer, primary_key=True)
    username = db.Column(db.String(80), unique=True, nullable=False)
    github_id = db.Column(db.String(80), unique=True, nullable=True)
    github_token_encrypted = db.Column(db.String(1024), nullable=True)

    # helper to list teams via memberships
    @property
    def teams(self):
        return [m.team for m in getattr(self, 'memberships', [])]


class Team(db.Model):
    __tablename__ = 'team'
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False)
    owner_id = db.Column(db.Integer, db.ForeignKey('user.id'), nullable=True)
    owner = db.relationship('User', backref='owned_teams')
    memberships = db.relationship('TeamMember', back_populates='team', cascade='all, delete-orphan')


class TeamMember(db.Model):
    __tablename__ = 'team_member'
    id = db.Column(db.Integer, primary_key=True)
    team_id = db.Column(db.Integer, db.ForeignKey('team.id'), nullable=False)
    user_id = db.Column(db.Integer, db.ForeignKey('user.id'), nullable=False)
    role = db.Column(db.String(32), default='member')
    team = db.relationship('Team', back_populates='memberships')
    user = db.relationship('User', backref='memberships')


class CoSpace(db.Model):
    __tablename__ = 'co_space'
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False)
    team_id = db.Column(db.Integer, db.ForeignKey('team.id'), nullable=False)
    team = db.relationship('Team')
    description = db.Column(db.Text, nullable=True)


class Comment(db.Model):
    __tablename__ = 'comment'
    id = db.Column(db.Integer, primary_key=True)
    cospace_id = db.Column(db.Integer, db.ForeignKey('co_space.id'), nullable=False)
    cospace = db.relationship('CoSpace')
    author_id = db.Column(db.Integer, db.ForeignKey('user.id'), nullable=True)
    author = db.relationship('User')
    selector = db.Column(db.String(512), nullable=True)
    text = db.Column(db.Text, nullable=True)
    comment_metadata = db.Column(db.Text, nullable=True)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
