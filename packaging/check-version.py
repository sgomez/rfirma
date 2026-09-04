#!/usr/bin/env python3
"""El candado de la version y del nombre del producto (ID-150, ID-151, ID-152,
ID-154, ID-145, ID-166).

LA FUENTE ES `rfirma-app/src-tauri/tauri.conf.json`, y no por gusto: es el
unico fichero que el bundler de Tauri lee para sellar la version dentro de los
tres paquetes, asi que es el que no puede mentir. Los otros tres sitios donde
el numero aparece quedan EN CANDADO: si divergen, esto se pone rojo.

    fuente    rfirma-app/src-tauri/tauri.conf.json   "version"
    candado   rfirma-app/package.json                "version"
    candado   rfirma-app/src-tauri/Cargo.toml        [package] version
    candado   packaging/.../metainfo.xml             <release version=...>

`rfirma-native-bridge/pom.xml` es el QUINTO sitio y SALE del candado (ID-150):
la version del puente es un artefacto interno que no lee nadie fuera del propio
Maven, y atarla a la de la aplicacion solo produce commits de ruido. No se
comprueba aqui a proposito; no lo anadas.

Lo demas que se comprueba son invariantes del mismo bloque de decisiones, todas
estaticas y de milisegundos:

  * el metainfo referencia el CHANGELOG en vez de copiarlo (ID-152);
  * el README no lleva enlaces de descarga con version dentro (ID-151);
  * todo `.desktop` y el titulo de la ventana ensenan `rFirma`, mientras que
    `productName` —que es el nombre del binario— sigue en `rfirma` (ID-145,
    ID-166);
  * la regla `-rc.N` -> sin paquetes nativos existe y se comporta (ID-154).

Uso: packaging/check-version.py
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

TAURI_CONF = "rfirma-app/src-tauri/tauri.conf.json"
PACKAGE_JSON = "rfirma-app/package.json"
CARGO_TOML = "rfirma-app/src-tauri/Cargo.toml"
METAINFO = "packaging/flatpak/me.sgomez.rfirma.metainfo.xml"
README = "README.md"
RC_RULE = "packaging/native-packages-allowed.sh"

failures: list[str] = []


def fail(message: str) -> None:
    failures.append(message)


def read(path: str) -> str:
    with open(os.path.join(ROOT, path), encoding="utf-8") as handle:
        return handle.read()


def package_json_version() -> str:
    return json.loads(read(PACKAGE_JSON))["version"]


def cargo_version() -> str | None:
    """La `version` de la seccion [package], no la de una dependencia."""
    section = None
    for line in read(CARGO_TOML).splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            section = stripped
            continue
        if section == "[package]":
            match = re.match(r'version\s*=\s*"([^"]+)"', stripped)
            if match:
                return match.group(1)
    return None


def newest_release(tree: ET.Element) -> ET.Element | None:
    """La primera <release> del metainfo: AppStream las quiere de nueva a vieja."""
    releases = tree.find("releases")
    if releases is None:
        return None
    return releases.find("release")


def check_lock(version: str) -> None:
    for path, found in (
        (PACKAGE_JSON, package_json_version()),
        (CARGO_TOML, cargo_version()),
    ):
        if found != version:
            fail(
                f"{path} dice {found!r} y {TAURI_CONF} dice {version!r}. "
                f"La fuente es {TAURI_CONF}: cambia el resto para que cuadre."
            )


def check_metainfo(version: str) -> None:
    tree = ET.fromstring(read(METAINFO))
    release = newest_release(tree)
    if release is None:
        fail(f"{METAINFO} no declara ninguna <release>")
        return

    if release.get("version") != version:
        fail(
            f"{METAINFO} publica la version {release.get('version')!r} y "
            f"{TAURI_CONF} dice {version!r}"
        )
    if not release.get("date"):
        fail(f"{METAINFO}: la <release> no lleva `date` (ID-152)")

    details = [
        url.text or ""
        for url in release.findall("url")
        if url.get("type") == "details"
    ]
    if not details:
        fail(
            f"{METAINFO}: la <release> no lleva <url type=\"details\"> al "
            f"CHANGELOG (ID-152)"
        )
    elif not any("CHANGELOG" in url for url in details):
        fail(f"{METAINFO}: el <url type=\"details\"> no apunta al CHANGELOG")

    if release.find("description") is not None:
        fail(
            f"{METAINFO}: la <release> COPIA las notas en <description>. "
            f"El metainfo referencia el CHANGELOG, no lo duplica (ID-152)."
        )


def check_product_name(conf: dict) -> None:
    """ID-166: `rFirma` donde se lee, `rfirma` donde se identifica."""
    if conf.get("productName") != "rfirma":
        fail(
            f"{TAURI_CONF}: productName es {conf.get('productName')!r}. Es un "
            f"IDENTIFICADOR —el binario, el paquete, el .desktop—, asi que va "
            f"todo en minuscula (ID-166)."
        )
    for window in conf.get("app", {}).get("windows", []):
        if window.get("title") != "rFirma":
            fail(
                f"{TAURI_CONF}: el titulo de la ventana es "
                f"{window.get('title')!r}. Lo lee la persona usuaria, asi que "
                f"es prosa: `rFirma` (ID-166)."
            )


def check_readme() -> None:
    readme = read(README)
    # Una URL de descarga con la version dentro (`releases/download/v0.4.0/...`)
    # es justo lo que ID-151 prohibe: envejece en silencio.
    versioned = re.search(r"releases/download/[^)\s]+", readme)
    if versioned:
        fail(
            f"{README} enlaza a una descarga con version dentro "
            f"({versioned.group(0)}); usa releases/latest/download/ (ID-151)"
        )
    if "releases/latest/download/" not in readme:
        fail(f"{README} no enlaza a releases/latest/download/ (ID-151)")


def check_desktop_names() -> None:
    """`Name=` es lo que la persona ve en el lanzador: va en prosa (ID-166)."""
    found = False
    for base, _dirs, files in os.walk(os.path.join(ROOT, "packaging")):
        for name in files:
            if not name.endswith(".desktop"):
                continue
            found = True
            path = os.path.relpath(os.path.join(base, name), ROOT)
            for line in read(path).splitlines():
                if line.startswith("Name="):
                    if line != "Name=rFirma":
                        fail(
                            f"{path}: {line!r}. En el lanzador se ve prosa, "
                            f"asi que es `Name=rFirma` (ID-145, ID-166)."
                        )
                    break
            else:
                fail(f"{path} no declara `Name=`")
    if not found:
        fail("no hay ningun .desktop bajo packaging/")


def check_rc_rule() -> None:
    """ID-154, comprobado ejecutando la regla, no leyendola."""
    script = os.path.join(ROOT, RC_RULE)
    if not os.access(script, os.X_OK):
        fail(f"{RC_RULE} no existe o no es ejecutable (ID-154)")
        return

    for sample, expected in (
        ("0.4.0", 0),
        ("v1.10.2", 0),
        ("0.4.0-rc.1", 1),
        ("0.4.0-rc.10", 1),
        ("1.0.0-beta.1", 1),
    ):
        code = subprocess.run(
            [script, sample], capture_output=True, text=True
        ).returncode
        if code != expected:
            verdict = "no produce" if expected else "produce"
            fail(
                f"{RC_RULE} {sample}: salida {code}, se esperaba {expected} "
                f"({sample} {verdict} paquetes nativos, ID-154)"
            )

    if subprocess.run([script], capture_output=True).returncode != 2:
        fail(f"{RC_RULE} sin argumento deberia fallar con 2 (uso incorrecto)")


def main() -> int:
    conf = json.loads(read(TAURI_CONF))
    version = conf["version"]
    check_lock(version)
    check_product_name(conf)
    check_metainfo(version)
    check_readme()
    check_desktop_names()
    check_rc_rule()

    if failures:
        print("EL CANDADO DE LA VERSION ESTA ROTO:", file=sys.stderr)
        for message in failures:
            print(f"  - {message}", file=sys.stderr)
        print(file=sys.stderr)
        print(
            f"La version se cambia SOLO en {TAURI_CONF} y el resto se pone al "
            f"dia detras (ID-150).",
            file=sys.stderr,
        )
        return 1

    print(f"candado de la version: {version} en los cuatro sitios, y el resto en orden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
