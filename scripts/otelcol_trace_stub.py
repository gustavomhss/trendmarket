#!/usr/bin/env python3
"""Minimal stub collector used when native otelcol-contrib binary is unavailable."""

from __future__ import annotations

import argparse
import os
import signal
import socket
import sys
import threading
import time
import urllib.parse

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class MetricsHandler(BaseHTTPRequestHandler):
    metrics_payload = (
        "# TYPE otelcol_stub_up gauge\n"
        "otelcol_stub_up 1\n"
    ).encode()

    def do_GET(self):  # noqa: N802 (http server signature)
        if self.path == "/metrics":
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; version=0.0.4")
            self.send_header("Content-Length", str(len(self.metrics_payload)))
            self.end_headers()
            self.wfile.write(self.metrics_payload)
        else:
            self.send_error(404, "Not Found")

    def log_message(self, format: str, *args):  # noqa: A003 (shadow built-in)
        sys.stdout.write("[metrics] " + (format % args) + "\n")
        sys.stdout.flush()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Stub otelcol trace collector")
    parser.add_argument("--config", required=True, help="Path to config file (unused, for parity)")
    parser.add_argument("--stub-validate", action="store_true", help="Only validate config and exit")
    return parser.parse_args()


def parse_endpoint() -> tuple[str, int]:
    addr = os.environ.get("OTELCOL_LISTEN_ADDR", "127.0.0.1")
    port = int(os.environ.get("OTELCOL_LISTEN_PORT", "8888"))
    return addr, port


def check_exporter(name: str, url: str | None) -> None:
    if not url:
        sys.stdout.write(f"[stub] exporter {name} disabled (no endpoint)\n")
        sys.stdout.flush()
        return
    parsed = urllib.parse.urlparse(url)
    host = parsed.hostname or "localhost"
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    try:
        with socket.create_connection((host, port), timeout=2):
            sys.stdout.write(f"[stub] exporter {name} reachable at {host}:{port}\n")
            sys.stdout.flush()
    except OSError as exc:
        sys.stderr.write(f"[stub] exporter {name} connection failed: {exc}\n")
        sys.stderr.flush()


def main() -> int:
    args = parse_args()
    addr, port = parse_endpoint()

    if args.stub_validate:
        if not os.path.exists(args.config):
            sys.stderr.write(f"[stub] config not found: {args.config}\n")
            return 1
        sys.stdout.write(f"[stub] validation succeeded for {args.config}\n")
        sys.stdout.flush()
        return 0

    sys.stdout.write(f"[stub] starting metrics endpoint on http://{addr}:{port}/metrics\n")
    sys.stdout.flush()

    tempo = os.environ.get("TEMPO_OTLP_HTTP", "http://localhost:4318")
    jaeger = os.environ.get("JAEGER_OTLP_HTTP", "http://localhost:4318")
    check_exporter("tempo", tempo)
    check_exporter("jaeger", jaeger)

    server = ThreadingHTTPServer((addr, port), MetricsHandler)
    shutdown = threading.Event()

    def handle_signal(signum, _frame):
        sys.stdout.write(f"[stub] received signal {signum}, shutting down\n")
        sys.stdout.flush()
        shutdown.set()

    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.5}, daemon=True)
    thread.start()

    try:
        while not shutdown.is_set():
            time.sleep(0.5)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
