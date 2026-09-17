#!/usr/bin/env python3
"""Sube la version en los sitios que declara el candado de check-version.py.

Cambia la fuente (`tauri.conf.json`) y cuadra detras el resto: `package.json`,
`Cargo.toml`, el metainfo, `Cargo.lock` (dejando que `cargo` lo reescriba) y
el sello sha256 de `packaging/flatpak/sources.lock` (ID-150). Termina
invocando el propio candado para confirmar que quedo todo en orden.

Uso: scripts/bump-version.sh <version>
"""

from __future__ import annotations

import datetime
import hashlib
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

TAURI_CONF = "rfirma-app/src-tauri/tauri.conf.json"
PACKAGE_JSON = "rfirma-app/package.json"
CARGO_TOML = "rfirma-app/src-tauri/Cargo.toml"
CARGO_LOCK = "rfirma-app/src-tauri/Cargo.lock"
PNPM_LOCK = "rfirma-app/pnpm-lock.yaml"
METAINFO = "packaging/flatpak/me.sgomez.rfirma.metainfo.xml"
SOURCES_LOCK = "packaging/flatpak/sources.lock"


def replace_json_version(relative: str, version: str) -> None:
    path = ROOT / relative
    text = path.read_text(encoding="utf-8")
    updated, count = re.subn(
        r'("version"\s*:\s*)"[^"]+"', rf'\g<1>"{version}"', text, count=1
    )
    if count != 1:
        sys.exit(f'{relative}: no encontre "version" que sustituir')
    path.write_text(updated, encoding="utf-8")


def replace_cargo_toml_version(version: str) -> None:
    path = ROOT / CARGO_TOML
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    in_package = False
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
            continue
        if in_package and re.match(r'version\s*=\s*"[^"]+"', stripped):
            lines[i] = f'version = "{version}"\n'
            path.write_text("".join(lines), encoding="utf-8")
            return
    sys.exit(f"{CARGO_TOML}: no encontre version en [package]")


def replace_metainfo_release(version: str) -> None:
    path = ROOT / METAINFO
    text = path.read_text(encoding="utf-8")
    today = datetime.date.today().isoformat()
    updated, count = re.subn(
        r'<release version="[^"]+" date="[^"]+">',
        f'<release version="{version}" date="{today}">',
        text,
        count=1,
    )
    if count != 1:
        sys.exit(f"{METAINFO}: no encontre <release> que sustituir")
    path.write_text(updated, encoding="utf-8")


def sha256_of(relative: str) -> str:
    digest = hashlib.sha256()
    digest.update((ROOT / relative).read_bytes())
    return digest.hexdigest()


def regenerate_sources_lock() -> None:
    lines = [f"{sha256_of(rel)}  {rel}\n" for rel in (CARGO_LOCK, PNPM_LOCK)]
    (ROOT / SOURCES_LOCK).write_text("".join(lines), encoding="utf-8")


def main() -> int:
    if len(sys.argv) != 2:
        sys.exit(f"Uso: {sys.argv[0]} <version>")
    version = sys.argv[1]

    replace_json_version(TAURI_CONF, version)
    replace_json_version(PACKAGE_JSON, version)
    replace_cargo_toml_version(version)
    replace_metainfo_release(version)

    # cargo reescribe Cargo.lock detras de Cargo.toml (packaging/check-version.py).
    # `cargo metadata` toca solo la entrada del paquete local: a diferencia de
    # `cargo generate-lockfile`, no vuelve a resolver el resto de dependencias.
    subprocess.run(
        [
            "cargo",
            "metadata",
            "--manifest-path",
            str(ROOT / CARGO_TOML),
            "--format-version",
            "1",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )

    regenerate_sources_lock()

    subprocess.run([sys.executable, str(ROOT / "packaging/check-version.py")], check=True)
    print(f"version subida a {version} en los cinco sitios del candado.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
