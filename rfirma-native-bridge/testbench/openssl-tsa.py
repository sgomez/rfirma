#!/usr/bin/env python3
"""TSA RFC 3161 por HTTP en el bucle local: cada peticion la contesta `openssl ts -reply` (ADR-0030).

Uso: openssl-tsa.py <carpeta> <fichero-de-puerto>
"""

import http.server
import pathlib
import subprocess
import sys

FOLDER = pathlib.Path(sys.argv[1])
PORT_FILE = pathlib.Path(sys.argv[2])


def write_the_authority():
    FOLDER.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "3650",
            "-subj",
            "/CN=rfirma fake TSA",
            "-addext",
            "extendedKeyUsage=critical,timeStamping",
            "-keyout",
            FOLDER / "tsa.key",
            "-out",
            FOLDER / "tsa.pem",
        ],
        check=True,
        capture_output=True,
    )
    (FOLDER / "serial").write_text("01\n")
    (FOLDER / "tsa.cnf").write_text(
        "[ tsa ]\ndefault_tsa = fake\n[ fake ]\n"
        f"serial = {FOLDER / 'serial'}\nsigner_digest = sha256\n"
        "default_policy = 0.4.0.2023.1.1\ndigests = sha1, sha256, sha384, sha512\n"
        "ess_cert_id_alg = sha256\n"
    )


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        query = FOLDER / "query.tsq"
        reply = FOLDER / "reply.tsr"
        query.write_bytes(self.rfile.read(int(self.headers["Content-Length"])))
        subprocess.run(
            [
                "openssl",
                "ts",
                "-reply",
                "-config",
                FOLDER / "tsa.cnf",
                "-queryfile",
                query,
                "-signer",
                FOLDER / "tsa.pem",
                "-inkey",
                FOLDER / "tsa.key",
                "-out",
                reply,
            ],
            check=True,
            capture_output=True,
        )
        body = reply.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "application/timestamp-reply")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


write_the_authority()
server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
PORT_FILE.write_text(str(server.server_address[1]))
server.serve_forever()
