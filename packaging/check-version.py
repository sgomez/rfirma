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
    candado   rfirma-app/src-tauri/Cargo.lock        [[package]] name = "rfirma"
    candado   packaging/.../metainfo.xml             <release version=...>

CUIDADO CON `Cargo.lock`: lo reescribe el primer `cargo` que corra despues de
tocar `Cargo.toml`, y `packaging/flatpak/sources.lock` sella su `sha256`. Si se
sube la version y no se regenera ese sello, quien se pone rojo NO es este
candado sino `check-flatpak-sources`, antes que el, y con un mensaje que manda
ejecutar `just flatpak-sources` —receta que no corre en el entorno de
desarrollo—. Subir la version es, en orden: cambiar `tauri.conf.json`, cuadrar
`package.json`, `Cargo.toml` y el metainfo, dejar que `cargo` reescriba
`Cargo.lock`, y regenerar el `sha256` de los dos ficheros de bloqueo dentro de
`packaging/flatpak/sources.lock` (`sha256sum`, sin tocar `cargo-sources.json`
si no ha cambiado ninguna dependencia).

`rfirma-native-bridge/pom.xml` es el SEXTO sitio y SALE del candado (ID-150):
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
  * la regla `-rc.N` -> sin paquetes nativos existe y se comporta (ID-154);
  * el verbo del menu contextual es un `Type=Service` de KDE que se llama
    «Firmar con rFirma», sale al primer nivel, desaparece con mas de un fichero
    y no viaja en el flatpak, mientras que ningun lanzador —ni los `.desktop`
    sueltos ni la plantilla `.desktop.hbs` que rellena el bundler de Tauri—
    declara `application/pdf` (ID-155, ID-156, ID-162, ID-165, ADR-0018).

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
CARGO_LOCK = "rfirma-app/src-tauri/Cargo.lock"
METAINFO = "packaging/flatpak/me.sgomez.rfirma.metainfo.xml"
README = "README.md"
RC_RULE = "packaging/native-packages-allowed.sh"
KDE_SERVICEMENU = "packaging/kde/rfirma-sign.desktop"
FLATPAK_MANIFEST = "packaging/flatpak/me.sgomez.rfirma.yml"
SERVICEMENU_TARGET = "/usr/share/kio/servicemenus/rfirma-sign.desktop"

# La plantilla del lanzador que el bundler de Tauri rellena para el `.deb` y
# el `.rpm`: es un `.desktop` con marcas de Handlebars dentro.
DESKTOP_TEMPLATE_SUFFIX = ".desktop.hbs"

# El verbo del menu contextual, palabra por palabra (ID-165).
VERB = "Firmar con rFirma"

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


def cargo_lock_version() -> str | None:
    """La `version` del paquete `rfirma` dentro de Cargo.lock.

    Va en el candado porque `cargo` la reescribe sola detras de `Cargo.toml`, y
    `packaging/flatpak/sources.lock` sella el `sha256` del fichero: una version
    a medio subir se manifiesta como un rojo de `check-flatpak-sources`, que
    manda hacer algo que no es lo que hay que hacer.
    """
    package = None
    for line in read(CARGO_LOCK).splitlines():
        stripped = line.strip()
        if stripped == "[[package]]":
            package = None
            continue
        match = re.match(r'name\s*=\s*"([^"]+)"', stripped)
        if match:
            package = match.group(1)
            continue
        match = re.match(r'version\s*=\s*"([^"]+)"', stripped)
        if match and package == "rfirma":
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
        (CARGO_LOCK, cargo_lock_version()),
    ):
        if found != version:
            fail(
                f"{path} dice {found!r} y {TAURI_CONF} dice {version!r}. "
                f"La fuente es {TAURI_CONF}: cambia el resto para que cuadre."
            )
            if path == CARGO_LOCK:
                fail(
                    f"{CARGO_LOCK} lo reescribe `cargo` solo, pero su sha256 "
                    f"esta sellado en packaging/flatpak/sources.lock: regenera "
                    f"ese sello con `sha256sum` de los dos ficheros de bloqueo "
                    f"o `check-flatpak-sources` se pondra rojo antes que esto."
                )


def check_metainfo(version: str) -> None:
    tree = ET.fromstring(read(METAINFO))

    # El <name> es lo que GNOME Software ensena: prosa, como el `Name=` del
    # lanzador y el titulo de la ventana (ID-166).
    name = tree.find("name")
    if name is None or (name.text or "").strip() != "rFirma":
        fail(
            f"{METAINFO}: <name> es "
            f"{None if name is None else (name.text or '').strip()!r}. Es lo "
            f"que se lee en el centro de software, asi que es prosa: `rFirma` "
            f"(ID-166)."
        )

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


def desktop_files() -> list[str]:
    """Rutas relativas de todo `.desktop` bajo `packaging/`, plantillas incluidas.

    El lanzador que instalan el `.deb` y el `.rpm` no es un `.desktop` suelto:
    es la plantilla `desktopTemplate` de Tauri, con extension `.desktop.hbs`.
    Dejarla fuera de esta lista dejaba sin vigilar justo el fichero donde un
    `MimeType=` convierte a rFirma en lectora de PDF (ID-155, ADR-0018).
    """
    paths = []
    for base, _dirs, files in os.walk(os.path.join(ROOT, "packaging")):
        for name in files:
            if name.endswith((".desktop", DESKTOP_TEMPLATE_SUFFIX)):
                paths.append(os.path.relpath(os.path.join(base, name), ROOT))
    return sorted(paths)


def check_desktop_templates_are_inspected(conf: dict) -> None:
    """Toda plantilla de lanzador declarada en Tauri pasa por las puertas.

    `desktop_files()` recorre `packaging/`; si algun dia la plantilla del `.deb`
    o la del `.rpm` se mudara fuera de ahi —o cambiara de extension— el ID-155
    dejaria de ser falsable en silencio. Esto lo pone rojo.
    """
    inspected = set(desktop_files())
    for target in ("deb", "rpm"):
        template = conf["bundle"]["linux"][target].get("desktopTemplate")
        if not template:
            fail(
                f"{TAURI_CONF}: el paquete {target} no declara "
                f"`desktopTemplate`, asi que su lanzador no se puede vigilar "
                f"(ID-155)"
            )
            continue
        relative = os.path.relpath(
            os.path.normpath(os.path.join(ROOT, "rfirma-app", "src-tauri", template)),
            ROOT,
        )
        if relative not in inspected:
            fail(
                f"{TAURI_CONF}: la plantilla de lanzador del paquete {target} "
                f"({relative}) queda fuera de las comprobaciones de `.desktop`. "
                f"Sin ella nada impide que rFirma se registre como lectora de "
                f"PDF (ID-155, ADR-0018)."
            )


def desktop_groups(path: str) -> dict[str, dict[str, str]]:
    """El `.desktop` como grupos (`[Desktop Entry]`, `[Desktop Action X]`)."""
    groups: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    for line in read(path).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = groups.setdefault(line[1:-1], {})
            continue
        if "=" in line:
            key, value = line.split("=", 1)
            current[key.strip()] = value.strip()
    return groups


def check_desktop_names() -> None:
    """`Name=` es lo que la persona ve: va en prosa (ID-165, ID-166).

    En un lanzador (`Type=Application`) es el nombre del producto; en un
    *servicemenu* de KDE (`Type=Service`) es el verbo, y el verbo esta fijado
    palabra por palabra por el ID-165.
    """
    paths = desktop_files()
    if not paths:
        fail("no hay ningun .desktop bajo packaging/")
    for path in paths:
        groups = desktop_groups(path)
        entry = groups.get("Desktop Entry", {})
        if entry.get("Type") == "Service":
            names = [
                group["Name"]
                for name, group in groups.items()
                if name.startswith("Desktop Action ") and "Name" in group
            ]
            if not names:
                fail(f"{path} no declara `Name=` en ninguna accion")
            for name in names:
                if name != VERB:
                    fail(
                        f"{path}: `Name={name}`. El verbo del menu es "
                        f"`Name={VERB}`, ni «Firmar» a secas ni otra cosa "
                        f"(ID-165)."
                    )
            continue
        if "Name" not in entry:
            fail(f"{path} no declara `Name=`")
        elif entry["Name"] != "rFirma":
            fail(
                f"{path}: `Name={entry['Name']}`. En el lanzador se ve prosa, "
                f"asi que es `Name=rFirma` (ID-145, ID-166)."
            )


def check_no_pdf_handler() -> None:
    """ID-155: rFirma es un verbo, no el programa de los PDF.

    Un `MimeType=` en un lanzador (`Type=Application`) la registra como
    candidata a lector predeterminado y produce «Abrir con rFirma», que miente
    sobre lo que va a pasar. En un `Type=Service` la misma clave no registra
    nada: filtra en que menu contextual aparece el verbo, y ahi es obligatoria.
    """
    for path in desktop_files():
        entry = desktop_groups(path).get("Desktop Entry", {})
        if entry.get("Type") == "Service":
            continue
        if "MimeType" in entry:
            fail(
                f"{path}: declara `MimeType={entry['MimeType']}`. Un lanzador "
                f"con tipo asociado convierte a rFirma en candidata a lector "
                f"de PDF, y rFirma es un verbo (ID-155, ADR-0018)."
            )


def check_kde_servicemenu(conf: dict) -> None:
    """El verbo de KDE: primer nivel, un solo PDF, y fuera del flatpak.

    ID-156 y ID-165 lo describen; el ID-162 deja al flatpak fuera a proposito.
    """
    groups = desktop_groups(KDE_SERVICEMENU)
    entry = groups.get("Desktop Entry", {})
    if entry.get("Type") != "Service":
        fail(f"{KDE_SERVICEMENU} no es un `Type=Service` (ID-156)")
    if "application/pdf" not in entry.get("MimeType", ""):
        fail(
            f"{KDE_SERVICEMENU} no filtra por `application/pdf`: el verbo "
            f"apareceria sobre cualquier fichero (ID-156)"
        )
    if entry.get("X-KDE-Priority") != "TopLevel":
        fail(
            f"{KDE_SERVICEMENU} no declara `X-KDE-Priority=TopLevel`: el verbo "
            f"caeria dentro del submenu «Acciones» (ID-165)"
        )
    if entry.get("X-KDE-RequiredNumberOfUrls") != "1":
        fail(
            f"{KDE_SERVICEMENU} no declara `X-KDE-RequiredNumberOfUrls=1`: con "
            f"mas de un PDF seleccionado el verbo tiene que desaparecer "
            f"(ID-156)"
        )
    for action in groups.values():
        exec_line = action.get("Exec")
        if exec_line and "%F" in exec_line:
            fail(
                f"{KDE_SERVICEMENU}: `Exec` con `%F` acepta varios ficheros; "
                f"rFirma firma uno (ID-156)"
            )

    if not os.access(os.path.join(ROOT, KDE_SERVICEMENU), os.X_OK):
        fail(
            f"{KDE_SERVICEMENU} no tiene el bit de ejecucion: KDE pide "
            f"confirmacion para cada servicemenu que no lo lleve"
        )

    for target in ("deb", "rpm"):
        files = conf["bundle"]["linux"][target].get("files", {})
        if files.get(SERVICEMENU_TARGET) != f"../../{KDE_SERVICEMENU}":
            fail(
                f"{TAURI_CONF}: el paquete {target} no instala "
                f"{SERVICEMENU_TARGET} (ID-156)"
            )

    if "servicemenus" in read(FLATPAK_MANIFEST):
        fail(
            f"{FLATPAK_MANIFEST} instala el servicemenu de KDE. El flatpak se "
            f"queda fuera del verbo a proposito (ID-162, ADR-0018)."
        )


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
            [script, sample], capture_output=True, text=True, check=False
        ).returncode
        if code != expected:
            verdict = "no produce" if expected else "produce"
            fail(
                f"{RC_RULE} {sample}: salida {code}, se esperaba {expected} "
                f"({sample} {verdict} paquetes nativos, ID-154)"
            )

    if subprocess.run([script], capture_output=True, check=False).returncode != 2:
        fail(f"{RC_RULE} sin argumento deberia fallar con 2 (uso incorrecto)")


def main() -> int:
    conf = json.loads(read(TAURI_CONF))
    version = conf["version"]
    check_lock(version)
    check_product_name(conf)
    check_metainfo(version)
    check_readme()
    check_desktop_names()
    check_desktop_templates_are_inspected(conf)
    check_no_pdf_handler()
    check_kde_servicemenu(conf)
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

    print(f"candado de la version: {version} en los cinco sitios, y el resto en orden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
