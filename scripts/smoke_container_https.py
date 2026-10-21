#!/usr/bin/env python3
"""Exercise real container auth, hostname resolution, TLS trust and API parsing."""

import argparse
import base64
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import ssl
import subprocess
import tempfile
import threading
from urllib.parse import parse_qs


def decode_base64(value):
    return base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))


class Handler(BaseHTTPRequestHandler):
    """Serve dummy tokens and API responses for the local container fixture."""

    def do_POST(self):
        raw_body = self.rfile.read(int(self.headers["Content-Length"]))
        if self.path == "/token":
            self.exchange_token(raw_body)
            return
        body = json.loads(raw_body)
        if self.headers.get("Authorization") != "Bearer fixture-access-token" or not self.path.endswith(":countTokens"):
            self.send_error(400)
            return
        fixture = self.server.fixture
        fixture.requests.append((self.path, body))
        if "fixture-hold" in json.dumps(body):
            fixture.holding.set()
            fixture.release.wait(20)
            return
        self.send_json(b'{"totalTokens":7}')

    def exchange_token(self, raw_body):
        form = parse_qs(raw_body.decode())
        header, claims, signature = form["assertion"][0].split(".")
        payload = json.loads(decode_base64(claims))
        if (form["grant_type"] != ["urn:ietf:params:oauth:grant-type:jwt-bearer"]
            or json.loads(decode_base64(header))["alg"] != "RS256"
            or payload["iss"] != "fixture@example.invalid"
            or payload["aud"] != f"https://host.docker.internal:{self.server.server_port}/token"):
            self.send_error(400)
            return
        fixture = self.server.fixture
        signature_file = fixture.root / "signature.bin"
        signature_file.write_bytes(decode_base64(signature))
        verified = subprocess.run([
            "openssl", "dgst", "-sha256", "-verify", str(fixture.root / "public.pem"),
            "-signature", str(signature_file),
        ], input=f"{header}.{claims}".encode(), capture_output=True)
        if verified.returncode:
            self.send_error(400)
            return
        fixture.token_exchanges.append(payload)
        self.send_json(b'{"access_token":"fixture-access-token","token_type":"Bearer","expires_in":3600}')

    def send_json(self, response):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, *_):
        pass


class ContainerFixture:
    """Own ephemeral test keys, HTTPS service and container check state."""

    def __init__(self, root, image, platform):
        self.root = root
        self.image = image
        self.platform = platform
        self.certificate = root / "ca.crt"
        self.key = root / "key.pem"
        self.requests = []
        self.token_exchanges = []
        self.holding = threading.Event()
        self.release = threading.Event()

    def generate_certificate(self):
        subprocess.run([
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            "-subj", "/CN=host.docker.internal", "-addext", "subjectAltName=DNS:host.docker.internal",
            "-addext", "basicConstraints=critical,CA:TRUE", "-keyout", str(self.key), "-out", str(self.certificate),
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.certificate.chmod(0o644)
        subprocess.run(["openssl", "pkey", "-in", str(self.key), "-pubout", "-out", str(self.root / "public.pem")],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def start_server(self):
        # Docker's bridge/host-gateway must reach this temporary test-only service.
        self.server = ThreadingHTTPServer(("0.0.0.0", 0), Handler)
        self.server.fixture = self
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.load_cert_chain(self.certificate, self.key)
        self.server.socket = context.wrap_socket(self.server.socket, server_side=True)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def command(self):
        command = ["docker", "run", "--rm", "--read-only", "--cap-drop", "ALL",
                   "--security-opt", "no-new-privileges", "--tmpfs", "/tmp:rw,nosuid,noexec",
                   "--add-host", "host.docker.internal:host-gateway"]
        if self.platform:
            command += ["--platform", self.platform]
        return command + [
            "-e", "GOOGLE_ACCESS_TOKEN=fixture-access-token", "-e", "VERTEX_PROJECT_ID=fixture-project",
            "-e", "VERTEX_REGION=us-central1", "-e", "VERTEX_TIMEOUT=3", "-e", "VERTEX_MAX_RETRIES=1",
            "-e", f"VERTEX_BASE_URL=https://host.docker.internal:{self.server.server_port}",
        ]

    def trust_options(self):
        return ["-v", f"{self.certificate}:/fixture/ca.crt:ro", "-e", "SSL_CERT_FILE=/fixture/ca.crt"]

    def run_tokens(self, command, options):
        return subprocess.run(command + options + ["--", self.image, "tokens", "fixture text"],
                              text=True, capture_output=True, timeout=45)

    def check_trusted(self):
        trusted = self.run_tokens(self.command(), self.trust_options())
        if trusted.returncode or "Token count: 7" not in trusted.stdout or len(self.requests) != 1:
            raise RuntimeError(f"trusted TLS/auth request failed: {trusted.stdout}\n{trusted.stderr}")

    def check_untrusted(self):
        untrusted = self.run_tokens(self.command(), [])
        if (not untrusted.returncode or len(self.requests) != 1
            or "self-signed certificate" not in untrusted.stderr.lower()):
            raise RuntimeError(f"expected untrusted-certificate rejection: {untrusted.stderr}")

    def check_hostname(self):
        wrong_host = [part.replace("VERTEX_BASE_URL=https://host.docker.internal:",
                                  "VERTEX_BASE_URL=https://wrong.fixture:") for part in self.command()]
        mismatch = self.run_tokens(wrong_host, ["--add-host", "wrong.fixture:host-gateway"] + self.trust_options())
        if (not mismatch.returncode or len(self.requests) != 1
            or "hostname mismatch" not in mismatch.stderr.lower()):
            raise RuntimeError(f"expected TLS hostname-mismatch rejection: {mismatch.stderr}")

    def check_service_account(self):
        credentials = self.root / "credentials.json"
        # The key is generated locally for this run; this is no cloud credential.
        credentials.write_text(json.dumps({
            "type": "service_account", "project_id": "fixture-project", "private_key_id": "fixture",
            "private_key": self.key.read_text(), "client_email": "fixture@example.invalid", "client_id": "fixture",
            "auth_uri": "https://example.invalid/auth",
            "token_uri": f"https://host.docker.internal:{self.server.server_port}/token",
            "auth_provider_x509_cert_url": "", "client_x509_cert_url": "",
        }))
        credentials.chmod(0o644)
        options = self.trust_options() + ["-v", f"{credentials}:/fixture/credentials.json:ro",
                                         "-e", "GOOGLE_APPLICATION_CREDENTIALS=/fixture/credentials.json"]
        account = self.run_tokens(self.command(), options)
        if (account.returncode or "Token count: 7" not in account.stdout
            or len(self.token_exchanges) != 1 or len(self.requests) != 2):
            raise RuntimeError(f"service-account JWT/HTTPS exchange failed: {account.stdout}\n{account.stderr}")

    def check_signal(self):
        # Stop during HTTPS: tini must forward TERM without Docker's KILL fallback.
        detached = [part for part in self.command() if part != "--rm"]
        container = subprocess.check_output(detached + ["-d"] + self.trust_options()
            + ["--", self.image, "tokens", "fixture-hold"], text=True).strip()
        try:
            if not self.holding.wait(15):
                raise RuntimeError("signal fixture did not reach the HTTPS server")
            subprocess.run(["docker", "stop", "--time", "5", container], check=True,
                           stdout=subprocess.DEVNULL, timeout=20)
            exit_code = subprocess.check_output(["docker", "inspect", "--format",
                "{{.State.ExitCode}}", container], text=True).strip()
            if exit_code not in {"0", "143"}:
                raise RuntimeError(f"init did not forward TERM cleanly: exit {exit_code}")
        finally:
            self.release.set()
            subprocess.run(["docker", "rm", "-f", container], stdout=subprocess.DEVNULL, check=True)

    def check(self):
        self.generate_certificate()
        self.start_server()
        try:
            self.check_trusted()
            self.check_untrusted()
            self.check_hostname()
            self.check_service_account()
            self.check_signal()
            print(f"Container HTTPS verified: {self.image}; token and RS256 service-account auth, "
                  "hostname lookup, trusted success, untrusted/hostname rejection, read-only/nonroot, TERM forwarding")
        finally:
            self.server.shutdown()
            self.server.server_close()
            self.thread.join()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--platform", choices=("linux/amd64", "linux/arm64"))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="vertex-https-") as directory:
        ContainerFixture(Path(directory), args.image, args.platform).check()


if __name__ == "__main__":
    main()
