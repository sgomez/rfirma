#!/usr/bin/env python3
"""Los lanzadores de escritorio de rFirma no la convierten en lectora de PDF (ADR-0018)."""

from __future__ import annotations

import json
import os
import sys

DEFAULT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

TAURI_CONF = "rfirma-app/src-tauri/tauri.conf.json"
KDE_SERVICEMENU = "packaging/kde/rfirma-sign.desktop"
FLATPAK_MANIFEST = "packaging/flatpak/me.sgomez.rfirma.yml"
SERVICEMENU_TARGET = "/usr/share/kio/servicemenus/rfirma-sign.desktop"
DESKTOP_TEMPLATE_SUFFIX = ".desktop.hbs"
VERB = "Firmar con rFirma"
ALLOWED_SCHEME_HANDLER = "x-scheme-handler/afirma"


def read(root: str, path: str) -> str:
    with open(os.path.join(root, path), encoding="utf-8") as handle:
        return handle.read()


def desktop_files(root: str) -> list[str]:
    paths = []
    for base, _dirs, files in os.walk(os.path.join(root, "packaging")):
        for name in files:
            if name.endswith((".desktop", DESKTOP_TEMPLATE_SUFFIX)):
                paths.append(os.path.relpath(os.path.join(base, name), root))
    return sorted(paths)


def desktop_groups(root: str, path: str) -> dict[str, dict[str, str]]:
    groups: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    for line in read(root, path).splitlines():
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


def check_templates_are_inspected(root: str, conf: dict) -> list[str]:
    failures = []
    inspected = set(desktop_files(root))
    for target in ("deb", "rpm"):
        template = conf["bundle"]["linux"][target].get("desktopTemplate")
        if not template:
            failures.append(
                f"{TAURI_CONF}: el paquete {target} no declara `desktopTemplate`, "
                f"asi que su lanzador no se puede vigilar (ADR-0018)"
            )
            continue
        relative = os.path.relpath(
            os.path.normpath(os.path.join(root, "rfirma-app", "src-tauri", template)),
            root,
        )
        if relative not in inspected:
            failures.append(
                f"{TAURI_CONF}: la plantilla de lanzador del paquete {target} "
                f"({relative}) queda fuera de las comprobaciones de `.desktop` "
                f"(ADR-0018)"
            )
    return failures


def check_names(root: str) -> list[str]:
    failures = []
    paths = desktop_files(root)
    if not paths:
        failures.append("no hay ningun .desktop bajo packaging/")
    for path in paths:
        groups = desktop_groups(root, path)
        entry = groups.get("Desktop Entry", {})
        if entry.get("Type") == "Service":
            names = [
                group["Name"]
                for name, group in groups.items()
                if name.startswith("Desktop Action ") and "Name" in group
            ]
            if not names:
                failures.append(f"{path} no declara `Name=` en ninguna accion")
            for name in names:
                if name != VERB:
                    failures.append(
                        f"{path}: `Name={name}`. El verbo del menu es `Name={VERB}` "
                        f"(ADR-0018)"
                    )
            continue
        if "Name" not in entry:
            failures.append(f"{path} no declara `Name=`")
        elif entry["Name"] != "rFirma":
            failures.append(
                f"{path}: `Name={entry['Name']}`. Se esperaba `Name=rFirma`"
            )
    return failures


def check_no_document_handler(root: str) -> list[str]:
    failures = []
    for path in desktop_files(root):
        entry = desktop_groups(root, path).get("Desktop Entry", {})
        if entry.get("Type") == "Service":
            continue
        declared = [t for t in entry.get("MimeType", "").split(";") if t]
        offenders = [t for t in declared if not t.startswith("x-scheme-handler/")]
        if offenders:
            failures.append(
                f"{path}: declara `MimeType={';'.join(offenders)}`. Un lanzador con "
                f"tipo de documento convierte a rFirma en lectora de PDF (ADR-0018)"
            )
        for scheme in declared:
            if scheme != ALLOWED_SCHEME_HANDLER:
                failures.append(
                    f"{path}: declara `MimeType={scheme}`. El unico esquema que "
                    f"rFirma atiende es `{ALLOWED_SCHEME_HANDLER}` (ADR-0018)"
                )
    return failures


def check_kde_servicemenu(root: str, conf: dict) -> list[str]:
    failures = []
    groups = desktop_groups(root, KDE_SERVICEMENU)
    entry = groups.get("Desktop Entry", {})
    if entry.get("Type") != "Service":
        failures.append(f"{KDE_SERVICEMENU} no es un `Type=Service` (ADR-0018)")
    if "application/pdf" not in entry.get("MimeType", ""):
        failures.append(f"{KDE_SERVICEMENU} no filtra por `application/pdf` (ADR-0018)")
    if entry.get("X-KDE-Priority") != "TopLevel":
        failures.append(
            f"{KDE_SERVICEMENU} no declara `X-KDE-Priority=TopLevel` (ADR-0018)"
        )
    if entry.get("X-KDE-RequiredNumberOfUrls") != "1":
        failures.append(
            f"{KDE_SERVICEMENU} no declara `X-KDE-RequiredNumberOfUrls=1` (ADR-0018)"
        )
    for action in groups.values():
        exec_line = action.get("Exec")
        if exec_line and "%F" in exec_line:
            failures.append(
                f"{KDE_SERVICEMENU}: `Exec` con `%F` acepta varios ficheros (ADR-0018)"
            )
    if not os.access(os.path.join(root, KDE_SERVICEMENU), os.X_OK):
        failures.append(f"{KDE_SERVICEMENU} no tiene el bit de ejecucion")
    for target in ("deb", "rpm"):
        files = conf["bundle"]["linux"][target].get("files", {})
        if files.get(SERVICEMENU_TARGET) != f"../../{KDE_SERVICEMENU}":
            failures.append(
                f"{TAURI_CONF}: el paquete {target} no instala {SERVICEMENU_TARGET}"
            )
    if "servicemenus" in read(root, FLATPAK_MANIFEST):
        failures.append(
            f"{FLATPAK_MANIFEST} instala el servicemenu de KDE; el flatpak se queda "
            f"fuera (ADR-0018)"
        )
    return failures


def check(root: str = DEFAULT_ROOT) -> list[str]:
    conf = json.loads(read(root, TAURI_CONF))
    return [
        *check_templates_are_inspected(root, conf),
        *check_names(root),
        *check_no_document_handler(root),
        *check_kde_servicemenu(root, conf),
    ]


def main() -> int:
    failures = check()
    if failures:
        print("LOS LANZADORES DE ESCRITORIO ESTAN ROTOS:", file=sys.stderr)
        for message in failures:
            print(f"  - {message}", file=sys.stderr)
        return 1
    print("lanzadores de escritorio: en orden")
    return 0


if __name__ == "__main__":
    sys.exit(main())
