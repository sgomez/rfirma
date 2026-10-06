#!/usr/bin/env python3
"""La selección de Claude Design derivada de los títulos de las historias (ADR-0046): `write` y `check`."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

DEFAULT_ROOT = Path(__file__).resolve().parent.parent

APP = "rfirma-app"
STORIES = "rfirma-app/src"
ENTRY = "rfirma-app/design-sync.entry.ts"
CONFIG = ".design-sync/config.json"
SEAL = ".design-sync/selection.lock"

PUBLISHED_LAYERS = {"Primitivos": 2, "Dominio": 3, "Flujos": 3}
LOCAL_LAYERS = {"Pantallas"}
WALK_NUMBER = re.compile(r"^\d+\s*·")

META_START = re.compile(r"\bconst\s+meta\s*=")
TITLE = re.compile(r'\btitle:\s*"([^"]+)"')
META_COMPONENT = re.compile(r"Meta<\s*typeof\s+(\w+)\s*>")
NAMED_IMPORT = re.compile(
    r'import\s+(type\s+)?\{([^}]*)\}\s*from\s*"([^"]+)"', re.DOTALL
)

ENTRY_HEADER = (
    "//! Lo que `/design-sync` compila para Claude Design, generado por "
    "`just design-sync-selection` desde los títulos de las historias (ADR-0046).\n"
    "\n"
    'import "./src/design-system/index.css";\n'
    'import "./src/app.css";\n'
    "\n"
)
FIXED_EXPORTS = [
    ("./src/design-system/DesignRoot", "export { DesignRoot } from"),
    ("./src/design-system/icons", "export * from"),
]


class SelectionError(Exception):
    pass


@dataclass(frozen=True)
class Piece:
    story: str
    key: str
    component: str
    module: str
    exports: tuple[str, ...]


@dataclass(frozen=True)
class Selection:
    entry: str
    title_map: dict[str, str]

    def digest(self) -> str:
        return seal_of(self.entry, self.title_map)


def seal_of(entry: str, title_map: dict[str, str]) -> str:
    canonical = entry + json.dumps(title_map, sort_keys=True, ensure_ascii=False)
    return hashlib.sha256(canonical.encode()).hexdigest()


@dataclass(frozen=True)
class Imported:
    exported: str
    module: str
    type_only: bool


def imports_of(source: str) -> dict[str, Imported]:
    found: dict[str, Imported] = {}
    for clause_type, names, module in NAMED_IMPORT.findall(source):
        for raw in names.split(","):
            name = raw.strip()
            if not name:
                continue
            type_only = bool(clause_type) or name.startswith("type ")
            name = name.removeprefix("type ").strip()
            exported, _, local = name.partition(" as ")
            found[(local or exported).strip()] = Imported(
                exported.strip(), module, type_only
            )
    return found


def title_of(source: str, story: str) -> str:
    start = META_START.search(source)
    title = TITLE.search(source, start.end()) if start else None
    if not title:
        raise SelectionError(f"{story}: la historia no declara `title` en su `meta`")
    return title.group(1)


def publishable_piece(title: str, story: str) -> str | None:
    segments = [segment.strip() for segment in title.split("/")]
    layer = segments[0]
    if layer in LOCAL_LAYERS:
        return None
    if layer not in PUBLISHED_LAYERS:
        raise SelectionError(
            f"{story}: capa desconocida «{layer}» en el título «{title}»"
        )
    if len(segments) != PUBLISHED_LAYERS[layer] or not all(segments):
        raise SelectionError(
            f"{story}: título «{title}» fuera de convención: "
            f"«{layer}» lleva {PUBLISHED_LAYERS[layer]} segmentos"
        )
    if WALK_NUMBER.match(segments[-1]):
        raise SelectionError(
            f"{story}: título «{title}» fuera de convención: "
            "la numeración de recorrido solo existe en «Pantallas»"
        )
    return segments[-1]


def entry_module(story_file: Path, specifier: str, app: Path, story: str) -> str:
    if not specifier.startswith("."):
        raise SelectionError(
            f"{story}: el componente viene de «{specifier}», que no es del árbol"
        )
    resolved = Path(os.path.normpath(story_file.parent / specifier))
    return "./" + resolved.relative_to(app).as_posix()


def piece_of(story_file: Path, root: Path) -> Piece | None:
    story = story_file.relative_to(root / STORIES).as_posix()
    source = story_file.read_text()
    piece = publishable_piece(title_of(source, story), story)
    if piece is None:
        return None
    imports = imports_of(source)
    meta_component = META_COMPONENT.search(source)
    declared = meta_component.group(1) if meta_component else "?"
    title_piece = piece.replace(" ", "")
    component = next(
        (name for name in (declared, title_piece) if name in imports), None
    )
    if component is None:
        raise SelectionError(
            f"{story}: componente publicable sin mapear: ni «{declared}» ni «{title_piece}» "
            "se importan en la historia"
        )
    source_module = imports[component].module
    exports = {component} | {
        name
        for name, imported in imports.items()
        if imported.module == source_module
        and not imported.type_only
        and name[:1].isupper()
    }
    return Piece(
        story=story,
        key=title_piece,
        component=imports[component].exported,
        module=entry_module(story_file, source_module, root / APP, story),
        exports=tuple(sorted(imports[name].exported for name in exports)),
    )


def pieces_of(root: Path) -> list[Piece]:
    pieces: list[Piece] = []
    errors: list[str] = []
    for story_file in sorted((root / STORIES).rglob("*.stories.tsx")):
        try:
            piece = piece_of(story_file, root)
        except SelectionError as error:
            errors.append(str(error))
            continue
        if piece is not None:
            pieces.append(piece)
    owners: dict[str, str] = {}
    for piece in pieces:
        if piece.key in owners:
            errors.append(
                f"{owners[piece.key]} y {piece.story}: las dos acaban en «{piece.key}» "
                "y el `titleMap` no puede distinguirlas"
            )
        owners.setdefault(piece.key, piece.story)
    if errors:
        raise SelectionError("\n".join(errors))
    return pieces


def natural(text: str) -> str:
    return text.lower()


def selection_of(root: Path) -> Selection:
    pieces = pieces_of(root)
    by_module: dict[str, set[str]] = {}
    for piece in pieces:
        by_module.setdefault(piece.module, set()).update(piece.exports)
    lines = {module: prefix for module, prefix in FIXED_EXPORTS}
    for module, names in by_module.items():
        lines[module] = "export { " + ", ".join(sorted(names, key=natural)) + " } from"
    body = "".join(
        f'{lines[module]} "{module}";\n' for module in sorted(lines, key=natural)
    )
    title_map = {
        piece.key: piece.component for piece in sorted(pieces, key=lambda p: p.key)
    }
    return Selection(entry=ENTRY_HEADER + body, title_map=title_map)


def write(root: Path) -> None:
    selection = selection_of(root)
    (root / ENTRY).write_text(selection.entry)
    config_file = root / CONFIG
    config = json.loads(config_file.read_text())
    config["titleMap"] = selection.title_map
    config_file.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n")
    (root / SEAL).write_text(selection.digest() + "\n")


def check(root: Path) -> list[str]:
    seal_file = root / SEAL
    if not seal_file.is_file():
        return [f"falta {SEAL}"]
    seal = seal_file.read_text().strip()
    problems: list[str] = []
    committed_map = json.loads((root / CONFIG).read_text()).get("titleMap", {})
    if seal_of((root / ENTRY).read_text(), committed_map) != seal:
        problems.append(
            f"{ENTRY} o el `titleMap` de {CONFIG} no coinciden con {SEAL}: editados a mano"
        )
    if selection_of(root).digest() != seal:
        problems.append(
            "la selección derivada de las historias ha cambiado desde la última regeneración"
        )
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("verb", choices=["write", "check"])
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    try:
        if args.verb == "write":
            write(args.root)
            print(f"regeneradas. Versiona {ENTRY}, {CONFIG} y {SEAL}.")
            return 0
        problems = check(args.root)
    except SelectionError as error:
        print(error, file=sys.stderr)
        return 1
    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        print(
            "Ejecuta 'just design-sync-selection' y versiona lo que cambie.",
            file=sys.stderr,
        )
        return 1
    print("selección de Claude Design al día")
    return 0


if __name__ == "__main__":
    sys.exit(main())
