#!/usr/bin/env python3
"""A stand-in for ESI's `POST /universe/ids/` for the Laravel reference.

    python3 -I mock_esi.py tests/fixtures/esi/names.json

Answers like ESI: every requested name found in the table (compared
case-insensitively) under each category that knows it, with the table's
spelling; categories without a match are left out. The Rust suites run
the same table through their own mock (tests/integration/common/esi.rs).

Control, for the recorder: `POST /_fail` with a status code makes every
answer that status (`0` ends it), `GET /_requests` lists the bodies
received, `POST /_reset` forgets them.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TABLE = json.load(open(sys.argv[1], encoding="utf-8"))
STATE = {"fail": 0, "requests": []}


def answer(names):
    result = {}
    for category, known in TABLE.items():
        lowered = {name.lower(): (name, ident) for name, ident in known.items()}
        found = [
            {"id": lowered[name.lower()][1], "name": lowered[name.lower()][0]}
            for name in names
            if isinstance(name, str) and name.lower() in lowered
        ]
        if found:
            result[category] = found
    return result


class Handler(BaseHTTPRequestHandler):
    def send(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/_requests":
            self.send(200, STATE["requests"])
        else:
            self.send(404, {"error": "not found"})

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length)
        if self.path == "/_fail":
            STATE["fail"] = int(body or 0)
            self.send(200, {})
        elif self.path == "/_reset":
            STATE["requests"] = []
            self.send(200, {})
        elif self.path == "/universe/ids/":
            names = json.loads(body or b"[]")
            STATE["requests"].append({
                "names": names,
                "user_agent": self.headers.get("User-Agent"),
                "compatibility_date": self.headers.get("X-Compatibility-Date"),
            })
            if STATE["fail"]:
                self.send(STATE["fail"], {"error": "mock failure"})
            else:
                self.send(200, answer(names))
        else:
            self.send(404, {"error": "not found"})

    def log_message(self, *args):
        pass


ThreadingHTTPServer(("0.0.0.0", 8000), Handler).serve_forever()
