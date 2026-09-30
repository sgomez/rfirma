#!/usr/bin/env python3
"""La versión de la aplicación: `bump`, `check` y `changelog`; no la de las herramientas fijadas."""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import tomllib

DEFAULT_ROOT = Path(__file__).resolve().parent.parent

TAURI_CONF = "rfirma-app/src-tauri/tauri.conf.json"
CARGO_TOML = "rfirma-app/src-tauri/Cargo.toml"
CARGO_LOCK = "rfirma-app/src-tauri/Cargo.lock"
METAINFO = "packaging/flatpak/me.sgomez.rfirma.metainfo.xml"
SOURCES_LOCK = "packaging/flatpak/sources.lock"
CHANGELOG = "CHANGELOG.md"
README = "README.md"

SEMVER = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?$")
SECTION_HEADING = re.compile(r"^## \[([^\]]+)\]", re.MULTILINE)
LOCK_ENTRY = re.compile(r'(\[\[package\]\]\nname = "rfirma"\nversion = ")([^"]+)(")')
METAINFO_RELEASE = re.compile(r'<release version="[^"]+" date="[^"]+">')

CATEGORY_BY_TYPE = {"feat": "Added", "perf": "Changed", "fix": "Fixed"}
CATEGORY_ORDER = ["Added", "Changed", "Fixed"]
INTERNAL_SCOPES = {
    "agents",
    "ci",
    "conformance",
    "just",
    "pr",
    "probe",
    "site-driver",
    "suite",
    "test",
    "testbench",
}
MERGE_SUBJECT = re.compile(r"^Merge pull request #(\d+) from ")
CONVENTIONAL = re.compile(r"^(\w+)(?:\(([^)]+)\))?!?: (.+)$")


class VersionError(Exception):
    pass


def read(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def write(root: Path, relative: str, text: str) -> None:
    (root / relative).write_text(text, encoding="utf-8")


def cargo_version(root: Path) -> str:
    package = tomllib.loads(read(root, CARGO_TOML)).get("package", {})
    if "version" not in package:
        raise VersionError(f"{CARGO_TOML}: no hay version en [package]")
    return package["version"]


def cargo_lock_version(root: Path) -> str | None:
    match = LOCK_ENTRY.search(read(root, CARGO_LOCK))
    return match.group(2) if match else None


def changelog_versions(root: Path) -> list[str]:
    return SECTION_HEADING.findall(read(root, CHANGELOG))


def check(root: Path, expected: str | None = None) -> list[str]:
    version = cargo_version(root)
    failures: list[str] = []
    if expected is not None and expected.removeprefix("v") != version:
        failures.append(
            f"{CARGO_TOML} dice {version!r} y quien llama esperaba "
            f"{expected.removeprefix('v')!r}"
        )
    lock = cargo_lock_version(root)
    if lock != version:
        failures.append(f"{CARGO_LOCK} dice {lock!r} y {CARGO_TOML} dice {version!r}")
    newest = next(iter(changelog_versions(root)), None)
    if newest != version:
        failures.append(
            f"la sección más reciente de {CHANGELOG} es {newest!r} y "
            f"{CARGO_TOML} dice {version!r}"
        )
    conf = json.loads(read(root, TAURI_CONF))
    if "version" in conf:
        failures.append(f"{TAURI_CONF} declara `version`: la fuente es {CARGO_TOML}")
    failures += check_product_name(conf)
    failures += check_metainfo(root, version)
    failures += check_readme(root)
    return failures


def check_product_name(conf: dict) -> list[str]:
    failures = []
    if conf.get("productName") != "rfirma":
        failures.append(
            f"{TAURI_CONF}: productName es {conf.get('productName')!r}; es el "
            f"identificador del binario y del paquete, `rfirma`"
        )
    for window in conf.get("app", {}).get("windows", []):
        if window.get("title") != "rFirma":
            failures.append(
                f"{TAURI_CONF}: el título de la ventana es {window.get('title')!r}; "
                f"lo lee la persona usuaria, `rFirma`"
            )
    return failures


def check_metainfo(root: Path, version: str) -> list[str]:
    tree = ET.fromstring(read(root, METAINFO))
    failures = []
    name = tree.find("name")
    shown = None if name is None else (name.text or "").strip()
    if shown != "rFirma":
        failures.append(f"{METAINFO}: <name> es {shown!r}; es prosa, `rFirma`")
    releases = tree.find("releases")
    release = None if releases is None else releases.find("release")
    if release is None:
        return [*failures, f"{METAINFO} no declara ninguna <release>"]
    if release.get("version") != version:
        failures.append(
            f"{METAINFO} publica la versión {release.get('version')!r} y "
            f"{CARGO_TOML} dice {version!r}"
        )
    if not release.get("date"):
        failures.append(f"{METAINFO}: la <release> no lleva `date`")
    details = [
        u.text or "" for u in release.findall("url") if u.get("type") == "details"
    ]
    if not any("CHANGELOG" in url for url in details):
        failures.append(
            f'{METAINFO}: la <release> no lleva <url type="details"> al CHANGELOG'
        )
    if release.find("description") is not None:
        failures.append(
            f"{METAINFO}: la <release> copia las notas en <description>; "
            f"se referencia el CHANGELOG, no se duplica"
        )
    return failures


def check_readme(root: Path) -> list[str]:
    readme = read(root, README)
    failures = []
    versioned = re.search(r"releases/download/[^)\s]+", readme)
    if versioned:
        failures.append(
            f"{README} enlaza a una descarga con versión dentro "
            f"({versioned.group(0)}); usa releases/latest/download/"
        )
    if "releases/latest/download/" not in readme:
        failures.append(f"{README} no enlaza a releases/latest/download/")
    return failures


def changelog_section(root: Path, version: str) -> str:
    changelog = read(root, CHANGELOG)
    headings = list(SECTION_HEADING.finditer(changelog))
    for index, heading in enumerate(headings):
        if heading.group(1) == version:
            body_start = changelog.index("\n", heading.end()) + 1
            end = (
                headings[index + 1].start()
                if index + 1 < len(headings)
                else len(changelog)
            )
            return changelog[body_start:end].strip()
    raise VersionError(f"{CHANGELOG} no tiene sección para {version}")


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], check=True, capture_output=True, text=True
    ).stdout


def previous_tag(root: Path) -> str:
    try:
        return git(root, "describe", "--tags", "--abbrev=0", "--match", "v*").strip()
    except subprocess.CalledProcessError as error:
        raise VersionError("no hay ninguna etiqueta v* de la que partir") from error


def changelog_entry(subject: str, body: str) -> tuple[str, str] | None:
    merge = MERGE_SUBJECT.match(subject)
    if merge:
        title = next((line for line in body.splitlines() if line.strip()), "")
        reference = f" (#{merge.group(1)})"
    else:
        title, reference = subject, ""
    parsed = CONVENTIONAL.match(title.strip())
    if not parsed:
        return None
    kind, scope, description = parsed.groups()
    category = CATEGORY_BY_TYPE.get(kind)
    if category is None or scope in INTERNAL_SCOPES:
        return None
    description = description.rstrip(".")
    return category, f"- {description[0].upper()}{description[1:]}{reference}."


def section_body(root: Path, since: str) -> str:
    log = git(root, "log", "--first-parent", "--format=%s%x1f%b%x1e", f"{since}..HEAD")
    commits = [
        record.strip("\n").partition("\x1f")
        for record in log.split("\x1e")
        if record.strip()
    ]
    lines: dict[str, list[str]] = {category: [] for category in CATEGORY_ORDER}
    for subject, _, body in reversed(commits):
        found = changelog_entry(subject, body)
        if found:
            lines[found[0]].append(found[1])
    blocks = [
        f"### {category}\n" + "\n".join(lines[category])
        for category in CATEGORY_ORDER
        if lines[category]
    ]
    return "\n\n".join(blocks) or "Sin cambios visibles para quien usa rFirma."


def with_changelog_section(changelog: str, section: str) -> str:
    first_heading = SECTION_HEADING.search(changelog)
    if first_heading is None:
        return changelog.rstrip("\n") + "\n\n" + section
    return (
        changelog[: first_heading.start()]
        + section
        + changelog[first_heading.start() :]
    )


def with_package_version(cargo_toml: str, version: str) -> str:
    lines = cargo_toml.splitlines(keepends=True)
    in_package = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
        elif in_package and re.match(r'version\s*=\s*"[^"]+"', stripped):
            lines[index] = f'version = "{version}"\n'
            return "".join(lines)
    raise VersionError(f"{CARGO_TOML}: no hay version en [package]")


def bump(root: Path, version: str, today: datetime.date) -> None:
    if not SEMVER.match(version):
        raise VersionError(f"la versión tiene que ser X.Y.Z o X.Y.Z-pre: {version}")
    if version in changelog_versions(root):
        raise VersionError(f"{CHANGELOG} ya tiene una sección para {version}")
    lock = read(root, CARGO_LOCK)
    if not LOCK_ENTRY.search(lock):
        raise VersionError(f"{CARGO_LOCK}: no hay entrada del paquete rfirma")
    metainfo = read(root, METAINFO)
    if not METAINFO_RELEASE.search(metainfo):
        raise VersionError(f"{METAINFO}: no hay <release> que sustituir")
    cargo_toml = with_package_version(read(root, CARGO_TOML), version)
    since = previous_tag(root)
    section = f"## [{version}] - {today.isoformat()}\n\n{section_body(root, since)}\n\n"

    write(root, CHANGELOG, with_changelog_section(read(root, CHANGELOG), section))
    write(root, CARGO_TOML, cargo_toml)
    write(root, CARGO_LOCK, LOCK_ENTRY.sub(rf"\g<1>{version}\g<3>", lock, count=1))
    write(
        root,
        METAINFO,
        METAINFO_RELEASE.sub(
            f'<release version="{version}" date="{today.isoformat()}">',
            metainfo,
            count=1,
        ),
    )
    digest = hashlib.sha256((root / CARGO_LOCK).read_bytes()).hexdigest()
    write(root, SOURCES_LOCK, f"{digest}  {CARGO_LOCK}\n")


def parse_arguments(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="app_version.py", description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    verbs = parser.add_subparsers(dest="verb", required=True)
    bump_verb = verbs.add_parser("bump", help="sube la versión en todos sus sitios")
    bump_verb.add_argument("version")
    check_verb = verbs.add_parser("check", help="imprime la versión si todo cuadra")
    check_verb.add_argument(
        "--expected", help="la versión que tiene que tener el árbol"
    )
    changelog_verb = verbs.add_parser(
        "changelog", help="imprime la sección de una versión"
    )
    changelog_verb.add_argument("version", nargs="?")
    return parser.parse_args(argv)


def run(arguments: argparse.Namespace) -> int:
    root = arguments.root
    if arguments.verb == "bump":
        bump(root, arguments.version, datetime.datetime.now(datetime.UTC).date())
        print(f"versión {arguments.version} en todos sus sitios", file=sys.stderr)
        return run(
            argparse.Namespace(root=root, verb="check", expected=arguments.version)
        )
    if arguments.verb == "changelog":
        print(changelog_section(root, arguments.version or cargo_version(root)))
        return 0
    failures = check(root, arguments.expected)
    if failures:
        print("la versión de la aplicación no cuadra:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(
            f"la fuente es {CARGO_TOML}: `just bump-version <versión>`", file=sys.stderr
        )
        return 1
    print(cargo_version(root))
    return 0


def main(argv: list[str]) -> int:
    try:
        return run(parse_arguments(argv))
    except VersionError as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
