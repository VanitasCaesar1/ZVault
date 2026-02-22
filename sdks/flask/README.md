# flask-zvault

ZVault integration for Flask — `app.config.from_zvault()`.

## Install

```bash
pip install flask-zvault
```

## Quick Start

```python
from flask import Flask
from flask_zvault import ZVault

app = Flask(__name__)
vault = ZVault(app, env="production")

@app.route("/")
def index():
    db_url = app.config["DATABASE_URL"]
    return "ok"
```

## License

MIT
