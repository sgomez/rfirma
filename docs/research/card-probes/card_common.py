"""Lo que comparten las sondas de tarjeta: módulo PKCS#11 por plataforma, reloj común y redacción de datos personales."""

import os
import platform
import re
import subprocess
import sys
import time

DEFAULT_MODULES = {
    "Linux": [
        "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
        "/usr/lib/aarch64-linux-gnu/opensc-pkcs11.so",
        "/usr/lib64/opensc-pkcs11.so",
        "/usr/lib/opensc-pkcs11.so",
    ],
    "Darwin": ["/Library/OpenSC/lib/opensc-pkcs11.so"],
    "Windows": [r"C:\Program Files\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll"],
}
DNIE_TOKEN = re.compile(r"^DNI electr", re.I)
SAFE_LABEL = re.compile(r"^(Cert|Kpriv|Kpub)[A-Za-z]*$|^DNI electr", re.I)


def default_module():
    for path in DEFAULT_MODULES.get(platform.system(), []):
        if os.path.exists(path):
            return path
    sys.exit("No encuentro OpenSC; pásalo con --module <ruta>")


def safe(label):
    label = (label or "").strip()
    return label if SAFE_LABEL.match(label) else "<redactado>"


def clock(t0, tag):
    return lambda *parts: print(f"[{tag} {time.time() - t0:7.2f}s]", *parts, flush=True)


def child(script, *args):
    return [sys.executable, "-I", "-u", script, *args]


def pkcs15_pin_lines():
    try:
        out = subprocess.run(
            ["pkcs15-tool", "--list-pins"], capture_output=True, text=True, timeout=30
        ).stdout
    except (OSError, subprocess.TimeoutExpired) as e:
        return [f"pkcs15-tool no disponible: {e}"]
    keys = ("PIN [", "Flags", "Tries", "tries", "Length", "Type")
    return [
        line.strip() for line in out.splitlines() if any(k in line for k in keys)
    ] or ["(sin líneas de PIN)"]
