#!/usr/bin/env python3
"""Escribe en CHANGELOG.md la sección de una versión a partir de los títulos de PR desde la última etiqueta."""

import datetime
import os
import re
import subprocess
import sys

root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
os.chdir(root)
version = sys.argv[1]

CATEGORY_BY_TYPE = {"feat": "Added", "perf": "Changed", "fix": "Fixed"}
CATEGORY_ORDER = ["Added", "Changed", "Fixed"]
INTERNAL_SCOPES = {
    "agents", "ci", "conformance", "just", "pr", "probe", "site-driver",
    "suite", "test", "testbench",
}
MERGE_SUBJECT = re.compile(r"^Merge pull request #(\d+) from ")
CONVENTIONAL = re.compile(r"^(\w+)(?:\(([^)]+)\))?!?: (.+)$")


def git(*args):
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout


def previous_tag():
    try:
        return git("describe", "--tags", "--abbrev=0", "--match", "v*").strip()
    except subprocess.CalledProcessError:
        sys.exit("No hay ninguna etiqueta v* de la que partir.")


def first_parent_commits(since):
    log = git("log", "--first-parent", "--format=%s%x1f%b%x1e", f"{since}..HEAD")
    for record in log.split("\x1e"):
        if record.strip():
            subject, _, body = record.strip("\n").partition("\x1f")
            yield subject, body


def entry(subject, body):
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


def section_body(since):
    lines = {category: [] for category in CATEGORY_ORDER}
    for subject, body in reversed(list(first_parent_commits(since))):
        found = entry(subject, body)
        if found:
            lines[found[0]].append(found[1])
    blocks = [
        f"### {category}\n" + "\n".join(lines[category])
        for category in CATEGORY_ORDER
        if lines[category]
    ]
    return "\n\n".join(blocks) or "Sin cambios visibles para quien usa rFirma."


changelog = open("CHANGELOG.md", encoding="utf-8").read()
if re.search(r"^## \[" + re.escape(version) + r"\]", changelog, re.MULTILINE):
    sys.exit(f"CHANGELOG.md ya tiene una sección para {version}.")

since = previous_tag()
today = datetime.date.today().isoformat()
section = f"## [{version}] - {today}\n\n{section_body(since)}\n\n"
first_heading = re.search(r"^## \[", changelog, re.MULTILINE)
if first_heading:
    changelog = changelog[: first_heading.start()] + section + changelog[first_heading.start():]
else:
    changelog = changelog.rstrip("\n") + "\n\n" + section

open("CHANGELOG.md", "w", encoding="utf-8").write(changelog)
print(f"CHANGELOG.md: sección {version} generada desde {since}.")
