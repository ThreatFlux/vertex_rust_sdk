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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--platform")
    args = parser.parse_args()
    requests = []
    token_exchanges = []
    holding = threading.Event()
    release = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            raw_body = self.rfile.read(int(self.headers["Content-Length"]))
            if self.path == "/token":
                form = parse_qs(raw_body.decode())
                assertion = form["assertion"][0]
                header, claims, signature = assertion.split(".")
                payload = json.loads(decode_base64(claims))
                if (form["grant_type"] != ["urn:ietf:params:oauth:grant-type:jwt-bearer"]
                    or json.loads(decode_base64(header))["alg"] != "RS256"
                    or payload["iss"] != "fixture@example.invalid"
                    or payload["aud"] != f"https://host.docker.internal:{self.server.server_port}/token"):
                    self.send_error(400)
                    return
                signature_file = root / "signature.bin"
                signature_file.write_bytes(decode_base64(signature))
                verified = subprocess.run(["openssl", "dgst", "-sha256", "-verify", str(root / "public.pem"),
                    "-signature", str(signature_file)], input=f"{header}.{claims}".encode(), capture_output=True)
                if verified.returncode:
                    self.send_error(400)
                    return
                token_exchanges.append(payload)
                response = b'{"access_token":"fixture-access-token","token_type":"Bearer","expires_in":3600}'
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(response)))
                self.end_headers()
                self.wfile.write(response)
                return
            body = json.loads(raw_body)
            if self.headers.get("Authorization") != "Bearer fixture-access-token" or not self.path.endswith(":countTokens"):
                self.send_error(400)
                return
            requests.append((self.path, body))
            if "fixture-hold" in json.dumps(body):
                holding.set()
                release.wait(20)
                return
            response = b'{"totalTokens":7}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)

        def log_message(self, *_):
            pass

    with tempfile.TemporaryDirectory(prefix="vertex-https-") as directory:
        root = Path(directory)
        certificate, key = root / "ca.crt", root / "key.pem"
        subprocess.run([
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            "-subj", "/CN=host.docker.internal", "-addext", "subjectAltName=DNS:host.docker.internal",
            "-addext", "basicConstraints=critical,CA:TRUE", "-keyout", str(key), "-out", str(certificate),
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        certificate.chmod(0o644)
        subprocess.run(["openssl", "pkey", "-in", str(key), "-pubout", "-out", str(root / "public.pem")],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        server = ThreadingHTTPServer(("0.0.0.0", 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(certificate, key)
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            command = ["docker", "run", "--rm", "--read-only", "--cap-drop", "ALL",
                       "--security-opt", "no-new-privileges", "--tmpfs", "/tmp:rw,nosuid,noexec",
                       "--add-host", "host.docker.internal:host-gateway"]
            if args.platform:
                command += ["--platform", args.platform]
            command += ["-e", "GOOGLE_ACCESS_TOKEN=fixture-access-token", "-e", "VERTEX_PROJECT_ID=fixture-project",
                        "-e", "VERTEX_REGION=us-central1", "-e", "VERTEX_TIMEOUT=3", "-e", "VERTEX_MAX_RETRIES=1",
                        "-e", f"VERTEX_BASE_URL=https://host.docker.internal:{server.server_port}"]
            trusted = subprocess.run(command + ["-v", f"{certificate}:/fixture/ca.crt:ro", "-e",
                "SSL_CERT_FILE=/fixture/ca.crt", args.image, "tokens", "fixture text"],
                text=True, capture_output=True, timeout=45)
            if trusted.returncode or "Token count: 7" not in trusted.stdout or len(requests) != 1:
                raise RuntimeError(f"trusted TLS/auth request failed: {trusted.stdout}\n{trusted.stderr}")
            untrusted = subprocess.run(command + [args.image, "tokens", "fixture text"],
                text=True, capture_output=True, timeout=45)
            if (not untrusted.returncode or len(requests) != 1
                or "self-signed certificate" not in untrusted.stderr.lower()):
                raise RuntimeError(f"expected untrusted-certificate rejection: {untrusted.stderr}")
            wrong_host = [part.replace("VERTEX_BASE_URL=https://host.docker.internal:",
                                      "VERTEX_BASE_URL=https://wrong.fixture:") for part in command]
            mismatch = subprocess.run(wrong_host + ["--add-host", "wrong.fixture:host-gateway", "-v",
                f"{certificate}:/fixture/ca.crt:ro", "-e", "SSL_CERT_FILE=/fixture/ca.crt",
                args.image, "tokens", "fixture text"], text=True, capture_output=True, timeout=45)
            if (not mismatch.returncode or len(requests) != 1
                or "hostname mismatch" not in mismatch.stderr.lower()):
                raise RuntimeError(f"expected TLS hostname-mismatch rejection: {mismatch.stderr}")

            credentials = root / "credentials.json"
            credentials.write_text(json.dumps({
                "type": "service_account", "project_id": "fixture-project", "private_key_id": "fixture",
                "private_key": key.read_text(), "client_email": "fixture@example.invalid", "client_id": "fixture",
                "auth_uri": "https://example.invalid/auth", "token_uri": f"https://host.docker.internal:{server.server_port}/token",
                "auth_provider_x509_cert_url": "", "client_x509_cert_url": "",
            }))
            credentials.chmod(0o644)
            account = subprocess.run(command + ["-v", f"{certificate}:/fixture/ca.crt:ro", "-v",
                f"{credentials}:/fixture/credentials.json:ro", "-e", "SSL_CERT_FILE=/fixture/ca.crt",
                "-e", "GOOGLE_APPLICATION_CREDENTIALS=/fixture/credentials.json",
                args.image, "tokens", "fixture text"], text=True, capture_output=True, timeout=45)
            if account.returncode or "Token count: 7" not in account.stdout or len(token_exchanges) != 1 or len(requests) != 2:
                raise RuntimeError(f"service-account JWT/HTTPS exchange failed: {account.stdout}\n{account.stderr}")
            # Stop while the CLI waits for HTTPS: tini must forward TERM to the
            # application and exit without Docker needing its KILL fallback.
            detached = [part for part in command if part != "--rm"]
            container = subprocess.check_output(detached + ["-d", "-v", f"{certificate}:/fixture/ca.crt:ro",
                "-e", "SSL_CERT_FILE=/fixture/ca.crt", args.image, "tokens", "fixture-hold"], text=True).strip()
            try:
                if not holding.wait(15):
                    raise RuntimeError("signal fixture did not reach the HTTPS server")
                subprocess.run(["docker", "stop", "--time", "5", container], check=True,
                               stdout=subprocess.DEVNULL, timeout=20)
                exit_code = subprocess.check_output(["docker", "inspect", "--format",
                    "{{.State.ExitCode}}", container], text=True).strip()
                if exit_code not in {"0", "143"}:
                    raise RuntimeError(f"init did not forward TERM cleanly: exit {exit_code}")
            finally:
                release.set()
                subprocess.run(["docker", "rm", "-f", container], stdout=subprocess.DEVNULL, check=True)
            print(f"Container HTTPS verified: {args.image}; token and RS256 service-account auth, hostname lookup, trusted success, untrusted/hostname rejection, read-only/nonroot, TERM forwarding")
        finally:
            server.shutdown()
            server.server_close()
            thread.join()


if __name__ == "__main__":
    main()
