import os
from flask import Flask
from .db import db
from .socketio import socketio


def create_app():
    app = Flask(__name__, instance_relative_config=False)
    app.config.from_mapping(
        SECRET_KEY=os.environ.get('SECRET_KEY', 'dev'),
        SQLALCHEMY_DATABASE_URI=os.environ.get('DATABASE_URL', 'sqlite:///codocs.db'),
        SQLALCHEMY_TRACK_MODIFICATIONS=False,
    )
    db.init_app(app)
    socketio.init_app(app)

    # register blueprints
    from .api import api_bp
    app.register_blueprint(api_bp, url_prefix='/api')

    return app


if __name__ == '__main__':
    app = create_app()
    # Run with socketio so emits work in production as well
    socketio.run(app, host='0.0.0.0', port=int(os.environ.get('PORT', 5000)))
