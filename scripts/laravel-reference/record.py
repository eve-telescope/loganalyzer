#!/usr/bin/env python3
"""Records golden fixtures from the running Laravel reference.

    scripts/laravel-reference/run.sh start
    python3 -I scripts/laravel-reference/record.py

Writes tests/fixtures/golden/:

- pages/<name>.json: GETs of the two Inertia pages (status, and the page
  object read with `X-Inertia: true`), the analysis page for uploaded
  fixture logs with query variants.
- http/<name>.json (+ .html, .bin): the non-Inertia answers: the health
  page, the error pages, downloads with their headers and bytes, the
  static files, the paths around the routes.
- mutations.json: every upload and regenerate scenario with the answer,
  the `errors` of the page Laravel redirected to, the rows afterwards and
  the names the mock ESI was asked for.

Every scenario starts from an empty database and storage. Only the
standard library; run with `python3 -I`.
"""

import base64
import hashlib
import http.cookiejar
import json
import os
import re
import shutil
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request

BASE = "http://127.0.0.1:8317"
APP_URL = "https://loganalyzer.eve-telescope.com"
HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
REF_DIR = os.environ.get("LARAVEL_REF_DIR", "/tmp/loganalyzer-laravel-ref")
STORAGE = os.path.join(REF_DIR, "app/storage/app/private/combat-logs")
OUT = os.path.join(REPO, "tests/fixtures/golden")
PARSER = os.path.join(REPO, "tests/fixtures/parser")
PUBLIC = os.path.join(REF_DIR, "app/public")
MAX_BYTES = 25600 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def request(path, method="GET", headers=None, body=None, jar=None):
    handlers = [NoRedirect()]
    if jar is not None:
        handlers.append(urllib.request.HTTPCookieProcessor(jar))
    req = urllib.request.Request(BASE + path, method=method, headers=headers or {}, data=body)
    try:
        with urllib.request.build_opener(*handlers).open(req, timeout=120) as response:
            return response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers, error.read()


def write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if isinstance(data, bytes):
        with open(path, "wb") as handle:
            handle.write(data)
    elif isinstance(data, str):
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(data)
    else:
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2, ensure_ascii=False)
            handle.write("\n")


def mysql(sql):
    result = subprocess.run(
        ["docker", "exec", "-i", "loganalyzer-laravel-mysql", "mysql", "-uroot", "-ploganalyzer",
         "--default-character-set=utf8mb4", "-N", "-B", "--raw", "loganalyzer"],
        input=sql.encode(), capture_output=True,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode() + sql[:400])
    return result.stdout.decode()


def json_rows(sql):
    return [json.loads(line) for line in mysql(sql).splitlines() if line.strip()]


def esi(path, body=b""):
    """The mock ESI's control endpoints, reached through the reference."""
    script = (
        "import sys,urllib.request;"
        f"r=urllib.request.Request('http://loganalyzer-laravel-esi:8000{path}',method='{'POST' if body is not None else 'GET'}',data=sys.stdin.buffer.read() or None);"
        "print(urllib.request.urlopen(r).read().decode())"
    )
    result = subprocess.run(
        ["docker", "run", "--rm", "-i", "--network", "loganalyzer-laravel", "python:3-slim",
         "python3", "-I", "-c", script],
        input=body or b"", capture_output=True, check=True,
    )
    return json.loads(result.stdout or b"null")


def reset():
    mysql("set foreign_key_checks=0; truncate combat_events; truncate combat_logs; truncate eve_entities;"
          " truncate sessions; set foreign_key_checks=1;")
    shutil.rmtree(STORAGE, ignore_errors=True)
    esi("/_reset", b"")
    esi("/_fail", b"0")


def version():
    _, _, body = request("/")
    page = re.search(rb'<script data-page="app" type="application/json">(.*?)</script>', body, re.S)
    return json.loads(page.group(1))["version"]


VERSION = None


def inertia(path, jar=None):
    status, headers, body = request(
        path, headers={"X-Inertia": "true", "X-Inertia-Version": VERSION, "Accept": "text/html, application/xhtml+xml"},
        jar=jar,
    )
    record = {"status": status}
    if status == 200:
        record["page"] = json.loads(body)
    elif 300 <= status < 400:
        record["location"] = headers.get("Location")
    else:
        record["content_type"] = headers.get("Content-Type")
    return record


def session():
    jar = http.cookiejar.CookieJar()
    request("/", jar=jar)
    token = next(cookie.value for cookie in jar if cookie.name == "XSRF-TOKEN")
    return jar, urllib.parse.unquote(token)


def multipart(fields, files):
    boundary = "----loganalyzer-golden-boundary"
    body = b""
    for name, value in fields:
        body += f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n'.encode() + value.encode() + b"\r\n"
    for name, filename, content in files:
        body += (f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"; filename="{filename}"\r\n'
                 "Content-Type: text/plain\r\n\r\n").encode() + content + b"\r\n"
    body += f"--{boundary}--\r\n".encode()
    return body, f"multipart/form-data; boundary={boundary}"


def rows():
    return {
        "combat_logs": json_rows(
            "select json_object('uuid', uuid, 'listener', listener, 'session_started', session_started,"
            " 'original_filename', original_filename) from combat_logs order by id"),
        "combat_events": json_rows(
            "select json_object('log', l.uuid, 'timestamp', e.timestamp, 'damage', e.damage, 'direction', e.direction,"
            " 'player_name', e.player_name, 'corporation', e.corporation, 'ship_name', e.ship_name, 'weapon', e.weapon,"
            " 'quality', e.quality, 'type', e.type) from combat_events e join combat_logs l on l.id = e.combat_log_id"
            " order by e.id"),
        "eve_entities": json_rows(
            "select json_object('kind', kind, 'name', name, 'eve_id', eve_id) from eve_entities order by id"),
        "raw_logs": sorted(os.listdir(STORAGE)) if os.path.isdir(STORAGE) else [],
    }


def upload(fields=(), files=(), csrf=True, referer="/"):
    jar, token = session()
    body, content_type = multipart(fields, files)
    headers = {"Content-Type": content_type, "Referer": BASE + referer, "X-Inertia": "true",
               "X-Inertia-Version": VERSION, "Accept": "text/html, application/xhtml+xml"}
    if csrf:
        headers["X-XSRF-TOKEN"] = token
    status, response_headers, response_body = request("/analyze", "POST", headers, body, jar)
    return jar, status, response_headers, response_body


def answer(jar, status, headers, body):
    record = {"status": status}
    if 300 <= status < 400:
        location = headers.get("Location")
        record["location"] = location.replace(BASE, "") if location else None
        followed = inertia(record["location"], jar)
        if followed["status"] == 200:
            record["errors"] = followed["page"]["props"].get("errors", {})
            record["component"] = followed["page"]["component"]
    else:
        record["content_type"] = headers.get("Content-Type")
        record["body"] = body.decode("utf-8", "replace") if len(body) < 20000 else None
    return record


def fixture(name):
    with open(os.path.join(PARSER, f"{name}.txt"), "rb") as handle:
        return handle.read()


def upload_scenario(name, fields=(), files=(), csrf=True, fail_esi=0, known=()):
    reset()
    for kind, entity, eve_id in known:
        mysql(f"insert into eve_entities (kind, name, eve_id, created_at, updated_at) values"
              f" ('{kind}', '{entity}', {eve_id if eve_id is not None else 'null'}, now(), now());")
    if fail_esi:
        esi("/_fail", str(fail_esi).encode())
    started = time.time()
    jar, status, headers, body = upload(fields, files, csrf)
    record = answer(jar, status, headers, body)
    record.update({
        "name": name,
        "kind": "upload",
        "request": {
            "fields": [list(field) for field in fields],
            "files": [{"field": f, "filename": n, "fixture": fx, "size": len(c) if fx is None else None,
                       "fill": fill} for f, n, c, fx, fill in files_meta(files)],
            "csrf": csrf,
        },
        "fail_esi": fail_esi,
        "known": [list(entry) for entry in known],
        "esi_requests": esi("/_requests", None),
        "rows": rows(),
        "seconds": round(time.time() - started, 1),
    })
    print(f"  {name}: {status} {record.get('location')} {record.get('errors')}")
    return record


FILE_META = {}


def files_meta(files):
    for field, filename, content in files:
        fixture_name, fill = FILE_META.get(id(content), (None, None))
        yield field, filename, content, fixture_name, fill


def file_from_fixture(name):
    content = fixture(name)
    FILE_META[id(content)] = (name, None)
    return content


def file_filled(size, byte=b"a"):
    content = byte * size
    FILE_META[id(content)] = (None, byte.decode())
    return content


def mutations():
    scenarios = []
    log = file_from_fixture("testlog")
    scenarios.append(upload_scenario("upload-testlog", files=[("log_file", "combat.txt", log)]))
    scenarios.append(upload_scenario("upload-named-file", files=[("log_file", "my-fight.txt", log)]))
    scenarios.append(upload_scenario("upload-testlog2", files=[("log_file", "testlog2.log", file_from_fixture("testlog2"))]))
    scenarios.append(upload_scenario("upload-testlog3", files=[("log_file", "testlog3.txt", file_from_fixture("testlog3"))]))
    scenarios.append(upload_scenario("upload-hammerhead", files=[(
        "log_file", "combat.txt", file_from_fixture("upload-names-matching-an-item-type-are-never-treated-as-characters"))]))
    scenarios.append(upload_scenario("upload-known-names", files=[("log_file", "combat.txt", log)],
                                     known=[("character", "Aria Vex", 93000001), ("character", "korr malan", None),
                                            ("inventory_type", "DRAKE", 24698)]))
    scenarios.append(upload_scenario("upload-esi-down", files=[("log_file", "combat.txt", log)], fail_esi=500))
    scenarios.append(upload_scenario("upload-esi-404", files=[("log_file", "combat.txt", log)], fail_esi=404))
    scenarios.append(upload_scenario("upload-tricky-names", files=[("log_file", "names.txt", file_from_fixture("edge-tricky-names"))]))
    scenarios.append(upload_scenario("upload-drones", files=[("log_file", "drones.txt", file_from_fixture("edge-drone-attribution"))]))
    scenarios.append(upload_scenario("upload-invalid-utf8", files=[("log_file", "broken.txt", file_from_fixture("edge-invalid-utf8"))]))
    scenarios.append(upload_scenario("upload-chat-log", files=[("log_file", "Local_20260101.txt", file_from_fixture("edge-utf16le-chat-bom"))]))
    scenarios.append(upload_scenario("upload-empty-file", files=[("log_file", "empty.txt", file_filled(0))]))
    scenarios.append(upload_scenario("upload-uppercase-extension", files=[("log_file", "FIGHT.TXT", log)]))
    scenarios.append(upload_scenario("upload-log-extension", files=[("log_file", "fight.Log", log)]))
    scenarios.append(upload_scenario("upload-windows-path", files=[("log_file", "C:\\fakepath\\fight.txt", log)]))
    scenarios.append(upload_scenario("upload-unicode-name", files=[("log_file", "Kämpfe 日本.txt", log)]))
    scenarios.append(upload_scenario("upload-max-size", files=[("log_file", "big.txt", file_filled(MAX_BYTES))]))
    scenarios.append(upload_scenario("upload-without-file"))
    scenarios.append(upload_scenario("upload-empty-file-input", files=[("log_file", "", b"")]))
    scenarios.append(upload_scenario("upload-text-field", fields=[("log_file", "not a file")]))
    scenarios.append(upload_scenario("upload-blank-text-field", fields=[("log_file", "   ")]))
    scenarios.append(upload_scenario("upload-wrong-extension", files=[("log_file", "fight.pdf", log)]))
    scenarios.append(upload_scenario("upload-no-extension", files=[("log_file", "fight", log)]))
    scenarios.append(upload_scenario("upload-trailing-space-extension", files=[("log_file", "fight.txt ", log)]))
    scenarios.append(upload_scenario("upload-oversized", files=[("log_file", "huge.txt", file_filled(MAX_BYTES + 1))]))
    scenarios.append(upload_scenario("upload-oversized-wrong-extension", files=[("log_file", "huge.pdf", file_filled(MAX_BYTES + 1))]))
    scenarios.append(upload_scenario("upload-without-csrf", files=[("log_file", "combat.txt", log)], csrf=False))
    scenarios.append(upload_scenario("upload-long-fields", files=[("log_file", "long.txt", file_from_fixture("edge-long-fields"))]))
    scenarios.append(upload_scenario("upload-huge-numbers", files=[("log_file", "huge.txt", file_from_fixture("edge-huge-numbers"))]))
    scenarios.append(upload_scenario("upload-numeric-pilot", files=[("log_file", "numeric.txt", file_from_fixture("edge-numeric-pilot"))]))
    scenarios.append(upload_scenario("upload-rich-neut-without-ship", files=[("log_file", "neut.txt", file_from_fixture("edge-rich-neut-without-ship"))]))

    # Regenerate.
    for name, raw, stale in [
        ("regenerate", "upload-a-stored-combat-log-can-be-regenerated-from-its-raw-contents", True),
        ("regenerate-corrupt", None, True),
        ("regenerate-missing", "missing", True),
    ]:
        reset()
        uuid = "8f0c2b6e-3d1a-4c5b-9e7f-0a1b2c3d4e5f"
        mysql("insert into combat_logs (uuid, listener, session_started, original_filename, created_at, updated_at)"
              f" values ('{uuid}', 'Stale Listener', '2025.01.01 00:00:00', 'fight.txt', now(), now());"
              "insert into combat_events (combat_log_id, timestamp, damage, direction, player_name, weapon, quality, type)"
              " values (1, '2025.01.01 00:00:01', 1, 'incoming', 'Stale Pilot', 'Stale Weapon', 'Hits', 'damage');")
        os.makedirs(STORAGE, exist_ok=True)
        path = os.path.join(STORAGE, f"{uuid}.txt.gz")
        if raw == "missing":
            pass
        elif raw is None:
            with open(path, "wb") as handle:
                handle.write(b"this is not gzip")
        else:
            import gzip
            with open(path, "wb") as handle:
                handle.write(gzip.compress(fixture(raw), 9))
        before = open(path, "rb").read() if os.path.exists(path) else None
        jar, token = session()
        status, headers, body = request(
            f"/logs/{uuid}/regenerate", "POST",
            {"X-XSRF-TOKEN": token, "X-Inertia": "true", "X-Inertia-Version": VERSION,
             "Referer": f"{BASE}/logs/{uuid}", "Accept": "text/html, application/xhtml+xml"},
            b"", jar)
        record = answer(jar, status, headers, body)
        after = open(path, "rb").read() if os.path.exists(path) else None
        record.update({"name": name, "kind": "regenerate", "uuid": uuid, "raw_fixture": raw,
                       "file_unchanged": before == after, "esi_requests": esi("/_requests", None), "rows": rows()})
        print(f"  {name}: {status}")
        scenarios.append(record)

    # Regenerate of an unknown log, and without the CSRF token.
    for name, path, csrf in [("regenerate-unknown", "/logs/00000000-0000-4000-8000-000000000000/regenerate", True),
                             ("regenerate-without-csrf", "/logs/00000000-0000-4000-8000-000000000000/regenerate", False)]:
        reset()
        jar, token = session()
        headers = {"Accept": "text/html"}
        if csrf:
            headers["X-XSRF-TOKEN"] = token
        status, response_headers, body = request(path, "POST", headers, b"", jar)
        scenarios.append({"name": name, "kind": "regenerate", "status": status,
                          "content_type": response_headers.get("Content-Type"), "body": body.decode("utf-8", "replace")})
    write(os.path.join(OUT, "mutations.json"), scenarios)


def page(name, path, jar=None):
    record = inertia(path, jar)
    record["path"] = path
    write(os.path.join(OUT, f"pages/{name}.json"), record)
    print(f"  page {name}: {record['status']}")
    return record


def record_http(name, path, method="GET", headers=None, keep_body="html"):
    status, response_headers, body = request(path, method, headers or {})
    record = {
        "path": path,
        "method": method,
        "status": status,
        "content_type": response_headers.get("Content-Type"),
        "headers": {key.lower(): value for key, value in response_headers.items()
                    if key.lower() in ("content-disposition", "cache-control", "allow", "location", "content-length")},
        "sha256": hashlib.sha256(body).hexdigest(),
        "length": len(body),
    }
    write(os.path.join(OUT, f"http/{name}.json"), record)
    if keep_body == "html":
        write(os.path.join(OUT, f"http/{name}.html"), body.decode("utf-8", "replace"))
    elif keep_body == "bin":
        write(os.path.join(OUT, f"http/{name}.bin"), body)
    print(f"  http {name}: {status}")
    return record


def stored_upload(name, filename, fixture_name):
    jar, status, headers, body = upload(files=[("log_file", filename, fixture(fixture_name))])
    assert status == 302, (name, status, body[:300])
    uuid = headers["Location"].rsplit("/", 1)[-1]
    return uuid


def pages():
    reset()
    page("home", "/")
    uuids = {
        "testlog": stored_upload("testlog", "my-fight.txt", "testlog"),
        "testlog2": stored_upload("testlog2", "testlog2.txt", "testlog2"),
        "testlog3": stored_upload("testlog3", "testlog3.txt", "testlog3"),
        "hammerhead": stored_upload("hammerhead", "combat.txt", "upload-names-matching-an-item-type-are-never-treated-as-characters"),
        "tricky": stored_upload("tricky", "Kämpfe.txt", "edge-tricky-names"),
        "drones": stored_upload("drones", "with space.txt", "edge-drone-attribution"),
        "empty": stored_upload("empty", "per%cent.txt", "edge-empty"),
        "multiline": stored_upload("multiline", "日本語.txt", "edge-multiline-then-notify"),
    }
    write(os.path.join(OUT, "uuids.json"), uuids)
    log = uuids["testlog"]
    page("log-testlog", f"/logs/{log}")
    page("log-testlog2", f"/logs/{uuids['testlog2']}")
    page("log-testlog3", f"/logs/{uuids['testlog3']}")
    page("log-hammerhead", f"/logs/{uuids['hammerhead']}")
    page("log-tricky", f"/logs/{uuids['tricky']}")
    page("log-drones", f"/logs/{uuids['drones']}")
    page("log-empty", f"/logs/{uuids['empty']}")
    page("log-multiline", f"/logs/{uuids['multiline']}")
    page("log-filters", f"/logs/{log}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex")
    page("log-filters-trimmed", f"/logs/{log}?from=%20&to=&pilot=%20%20Aria%20Vex%20&hide=%20dpsDealt%20,,0,%20,neutIn,%200%20")
    page("log-filters-unicode-space", f"/logs/{log}?pilot=%E2%80%8BAria%E3%80%80Vex%C2%A0&from=%E2%80%8E2026-02-12T10:20:00")
    page("log-filters-repeated", f"/logs/{log}?from=a&from=b&hide=dpsDealt&hide=neutOut")
    page("log-filters-plus", f"/logs/{log}?pilot=Aria+Vex&hide=")
    page("log-filters-only-zero", f"/logs/{log}?hide=0")
    page("log-filters-array", f"/logs/{log}?pilot%5B%5D=Aria%20Vex&from%5Bx%5D=1")
    page("log-filters-hide-array", f"/logs/{log}?hide%5B%5D=dpsDealt")
    page("log-uppercase-uuid", f"/logs/{log.upper()}")
    page("log-unknown", "/logs/00000000-0000-4000-8000-000000000000")
    page("log-malformed", "/logs/not-a-uuid")
    page("log-braces", f"/logs/%7B{log}%7D")
    page("log-no-hyphens", f"/logs/{log.replace('-', '')}")
    page("log-trailing-space", f"/logs/{log}%20")
    page("log-trailing-slash", f"/logs/{log}/")
    page("index-php", "/index.php")

    # Downloads, with the names browsers sent.
    for key in ["testlog", "tricky", "drones", "empty", "multiline"]:
        record_http(f"download-{key}", f"/logs/{uuids[key]}/download", keep_body="bin")
    record_http("download-head", f"/logs/{log}/download", "HEAD", keep_body=None)
    record_http("download-unknown", "/logs/00000000-0000-4000-8000-000000000000/download")
    record_http("download-trailing-slash", f"/logs/{log}/download/", keep_body=None)
    os.remove(os.path.join(STORAGE, f"{uuids['testlog2']}.txt.gz"))
    page("log-without-raw-file", f"/logs/{uuids['testlog2']}")
    record_http("download-without-raw-file", f"/logs/{uuids['testlog2']}/download")
    with open(os.path.join(STORAGE, f"{uuids['testlog3']}.txt.gz"), "wb") as handle:
        handle.write(b"not gzip at all")
    record_http("download-corrupt", f"/logs/{uuids['testlog3']}/download", keep_body="bin")
    page("log-corrupt-raw-file", f"/logs/{uuids['testlog3']}")


def http_routes():
    record_http("up", "/up")
    record_http("not-found", "/nope")
    record_http("analyze-get", "/analyze")
    record_http("analyze-head", "/analyze", "HEAD", keep_body=None)
    record_http("storage-get", "/storage/combat-logs/x.txt.gz")
    record_http("storage-put", "/storage/x", "PUT")
    record_http("regenerate-get", "/logs/00000000-0000-4000-8000-000000000000/regenerate")
    record_http("api-unknown", "/api/pages/home")
    for name in ["favicon.ico", "favicon.svg", "apple-touch-icon.png", "icon-192.png", "icon-512.png",
                 "icon-512-maskable.png", "og-image.png", "robots.txt", "site.webmanifest"]:
        record_http(f"static-{name}", f"/{name}", keep_body=None)


VERSION = version()
print("http")
http_routes()
print("pages")
pages()
print("mutations")
mutations()
reset()
print("done")
