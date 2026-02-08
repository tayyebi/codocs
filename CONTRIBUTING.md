# Contributing to Codocs

Thank you for your interest in contributing to Codocs! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates. When creating a bug report, include:

- A clear and descriptive title
- Steps to reproduce the behavior
- Expected vs. actual behavior
- Screenshots if applicable
- Your environment (OS, Python version, browser, etc.)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, include:

- A clear and descriptive title
- Detailed description of the proposed functionality
- Why this enhancement would be useful
- Possible implementation approach (optional)

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. Ensure the test suite passes
4. Make sure your code follows the existing style
5. Write a clear commit message
6. Open a pull request with a clear title and description

## Development Setup

### Backend Development

```bash
cd backend
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
python app.py
```

### Running Tests

```bash
# From project root
PYTHONPATH=. pytest -v
```

### Extension Development

1. Make changes to files in the `extension/` directory
2. Load the unpacked extension in Chrome:
   - Navigate to `chrome://extensions`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select the `extension/` directory

## Coding Guidelines

### Python Code Style

- Follow PEP 8 style guide
- Use meaningful variable and function names
- Add docstrings to functions and classes
- Keep functions focused and concise
- Add comments for complex logic

### JavaScript Code Style

- Use ES6+ features where appropriate
- Use consistent indentation (2 spaces)
- Add comments for complex logic
- Handle errors appropriately

### Git Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

Example:
```
Add GitHub Gist export functionality

- Implement new API endpoint for Gist creation
- Add OAuth flow for GitHub authentication
- Update frontend to display export button
- Add tests for export functionality

Fixes #123
```

## Project Structure

```
codocs/
├── backend/          # Flask backend application
│   ├── api.py        # API endpoints
│   ├── app.py        # Application factory
│   ├── models.py     # Database models
│   ├── db.py         # Database configuration
│   ├── socketio.py   # Socket.IO configuration
│   └── tests/        # Backend tests
├── extension/        # Browser extension
│   ├── manifest.json # Extension manifest
│   └── content.js    # Content script
└── .github/          # GitHub configuration
    └── workflows/    # CI/CD workflows
```

## Testing

- Write tests for all new features
- Ensure existing tests pass before submitting PR
- Aim for good test coverage
- Use descriptive test names

## Documentation

- Update README.md if you change functionality
- Add comments to complex code sections
- Update API documentation for new endpoints
- Include examples where helpful

## Questions?

Feel free to open an issue with your question or reach out to the maintainers.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
