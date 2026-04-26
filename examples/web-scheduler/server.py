#!/usr/bin/env python3
"""
scheduler-svc — fumola web service, runs the same way locally and in docker.

Listens on :8080 by default (override with PORT). Serves:

  GET  /                            -> index.html
  GET  /schedule, /schedule/<v>     -> invokes fumola handle(method, path, body)
  GET  /<anything>                  -> fumola handler (handler decides 404)
  POST /<anything>                  -> fumola handler

Every fumola response carries an X-Fumola-DCG-Digest header (sha256 over
project_name|method|path|body|output). In an attested deployment the digest
is what the dstack TEE quote signs.

Local dev:
    cd ~/projects/matthammer && PORT=8002 python apps/scheduler-svc/server.py

In docker (built by Dockerfile in this directory):
    docker run -p 8080:8080 scheduler-svc:local
"""
import asyncio
import hashlib
import json
import os
import re
import shutil
import sys

try:
    from aiohttp import web
except ImportError:
    sys.exit("aiohttp not installed; pip install aiohttp")

HERE = os.path.dirname(os.path.abspath(__file__))
HANDLE_FUMOLA = os.path.join(HERE, "handle.fumola")
INDEX_HTML = os.path.join(HERE, "index.html")
PROJECT_NAME = "scheduler-svc"
TIMEOUT_SEC = 10
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def find_fumola_binary() -> str:
    # 1. Explicit override.
    env = os.environ.get("FUMOLA_BIN")
    if env and os.access(env, os.X_OK):
        return env
    # 2. Inside the docker image we install fumola here.
    if os.access("/usr/local/bin/fumola", os.X_OK):
        return "/usr/local/bin/fumola"
    # 3. Local dev: walk up looking for target/release/fumola in any
    #    ancestor directory. Works whether this lives under the fumola
    #    repo (../../target/...) or in a sibling layout.
    here = HERE
    for _ in range(6):
        candidate = os.path.join(here, "target", "release", "fumola")
        if os.access(candidate, os.X_OK):
            return candidate
        nxt = os.path.dirname(here)
        if nxt == here:
            break
        here = nxt
    # 4. PATH lookup as last resort.
    p = shutil.which("fumola")
    if p:
        return p
    sys.exit("fumola binary not found; build it (cargo build --release) or set FUMOLA_BIN")


FUMOLA_BIN = None  # filled in main()


def fumola_quote(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def sha_digest(*parts) -> str:
    h = hashlib.sha256()
    for p in parts:
        h.update(p.encode("utf-8") if isinstance(p, str) else p)
        h.update(b"\x00")
    return h.hexdigest()


async def invoke_fumola(method: str, path: str, body: bytes) -> dict:
    body_str = body.decode("utf-8", errors="replace") if body else ""
    module = os.path.splitext(HANDLE_FUMOLA)[0]
    program = f'import H "{module}"; H.handle({fumola_quote(method)}, {fumola_quote(path)}, {fumola_quote(body_str)})'
    proc = await asyncio.create_subprocess_exec(
        FUMOLA_BIN, "eval", program, "--import", HANDLE_FUMOLA,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=TIMEOUT_SEC)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        return {"status": 504, "raw": "fumola timeout"}

    combined = ANSI.sub("", (stdout or b"").decode("utf-8", errors="replace") + (stderr or b"").decode("utf-8", errors="replace"))
    if "ERROR fumola" in combined and ("Interruption" in combined or "SyntaxError" in combined):
        return {"status": 500, "raw": combined, "error": True}
    lines = []
    for line in combined.splitlines():
        s = line.lstrip()
        if s.startswith("[20") and "fumola" in s:
            continue
        lines.append(line)
    return {"status": 200, "raw": "\n".join(lines).strip()}


async def serve_index(request: web.Request) -> web.Response:
    if not os.path.isfile(INDEX_HTML):
        return web.Response(text="(no index.html)", status=404)
    with open(INDEX_HTML, "rb") as f:
        return web.Response(body=f.read(), headers={"Content-Type": "text/html; charset=utf-8"})


async def route(request: web.Request) -> web.Response:
    path = request.path
    if path == "/" or path == "/index.html":
        return await serve_index(request)
    body = await request.read()
    result = await invoke_fumola(request.method, path, body)
    raw = result.get("raw", "")
    digest = sha_digest(PROJECT_NAME, request.method, path, body, raw)
    headers = {
        "X-Fumola-DCG-Digest": digest,
        "X-Fumola-Project": PROJECT_NAME,
        "Content-Type": "text/plain; charset=utf-8",
    }
    return web.Response(body=raw, status=result["status"], headers=headers)


def main():
    global FUMOLA_BIN
    FUMOLA_BIN = find_fumola_binary()
    port = int(os.environ.get("PORT", "8080"))
    print(f"scheduler-svc: fumola={FUMOLA_BIN}")
    print(f"scheduler-svc: handle={HANDLE_FUMOLA}")
    print(f"scheduler-svc: serving http://0.0.0.0:{port}/")
    app = web.Application()
    app.router.add_route("*", "/{path:.*}", route)
    web.run_app(app, host="0.0.0.0", port=port, print=lambda *a: None)


if __name__ == "__main__":
    main()
