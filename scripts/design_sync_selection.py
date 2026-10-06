#!/usr/bin/env python3
"""La selección de Claude Design derivada de los títulos de las historias (ADR-0046): regenerada con `just design-sync-selection`."""

from __future__ import annotations

import argparse
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

PUBLISHED_LAYERS = {"Primitivos": 2, "Dominio": 3, "Flujos": 3}
LOCAL_LAYERS = {"Pantallas"}
OVERRIDE_KEYS = ("cardMode", "primaryStory", "viewport")
CATALOG_START = "<!-- design-sync:catalog:start -->"
CATALOG_END = "<!-- design-sync:catalog:end -->"
WALK_NUMBER = re.compile(r"^\d+\s*·")

META_START = re.compile(r"\bconst\s+meta\s*=")
TITLE = re.compile(r'\btitle:\s*"([^"]+)"')
DESIGN_SYNC = re.compile(r"\bdesignSync:\s*\{([^}]*)\}")
OVERRIDE_PAIR = re.compile(r'(\w+):\s*"([^"]*)"')
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
    title: str
    key: str
    component: str
    module: str
    exports: tuple[str, ...]
    overrides: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class Selection:
    entry: str
    title_map: dict[str, str]
    overrides: dict[str, dict[str, str]]
    catalog: str


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


def overrides_of(source: str, story: str) -> tuple[tuple[str, str], ...]:
    declared = DESIGN_SYNC.search(source)
    if not declared:
        return ()
    pairs = OVERRIDE_PAIR.findall(declared.group(1))
    unknown = sorted({key for key, _ in pairs} - set(OVERRIDE_KEYS))
    if unknown:
        raise SelectionError(
            f"{story}: `designSync` declara «{', '.join(unknown)}», "
            f"que no es ninguno de {', '.join(OVERRIDE_KEYS)}"
        )
    return tuple(sorted(pairs))


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
    title = title_of(source, story)
    piece = publishable_piece(title, story)
    overrides = overrides_of(source, story)
    if piece is None:
        if overrides:
            raise SelectionError(
                f"{story}: `designSync` en una historia no publicable «{title}»"
            )
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
        title=title,
        key=title_piece,
        component=imports[component].exported,
        module=entry_module(story_file, source_module, root / APP, story),
        exports=tuple(sorted(imports[name].exported for name in exports)),
        overrides=overrides,
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
    declared: dict[str, Piece] = {}
    for piece in pieces:
        if not piece.overrides:
            continue
        other = declared.setdefault(piece.component, piece)
        if other.overrides != piece.overrides:
            errors.append(
                f"{other.story} y {piece.story}: declaran `designSync` distinto "
                f"para «{piece.component}»"
            )
    if errors:
        raise SelectionError("\n".join(errors))
    return pieces


def catalog_of(pieces: list[Piece]) -> str:
    rows = [
        "| Capa | Título | Componente |",
        "| --- | --- | --- |",
    ]
    for piece in sorted(pieces, key=lambda p: p.title.lower()):
        layer, _, rest = piece.title.partition("/")
        label = " / ".join(segment.strip() for segment in rest.split("/"))
        rows.append(f"| {layer.strip()} | {label} | `{piece.component}` |")
    return "\n".join(rows) + "\n"


def block_bounds(text: str, file: str) -> tuple[int, int]:
    start, end = text.find(CATALOG_START), text.find(CATALOG_END)
    if start < 0 or end < start:
        raise SelectionError(
            f"{file}: faltan los marcadores {CATALOG_START} y {CATALOG_END}"
        )
    return start + len(CATALOG_START), end


def readme_header(root: Path, config: dict) -> Path | None:
    header = config.get("readmeHeader")
    return root / header if header else None


def write_catalog(root: Path, config: dict, catalog: str) -> None:
    header = readme_header(root, config)
    if header is None:
        return
    text = header.read_text()
    start, end = block_bounds(text, str(header.relative_to(root)))
    header.write_text(f"{text[:start]}\n{catalog}{text[end:]}")


def natural(text: str) -> str:
    return text.lower()


def selection_of(root: Path, config: dict) -> Selection:
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
    overrides = {
        piece.component: dict(piece.overrides)
        for piece in sorted(pieces, key=lambda p: p.component)
        if piece.overrides
    }
    return Selection(
        entry=ENTRY_HEADER + body,
        title_map=title_map,
        overrides=overrides,
        catalog=catalog_of(pieces) if readme_header(root, config) else "",
    )


def write(root: Path) -> None:
    config_file = root / CONFIG
    config = json.loads(config_file.read_text())
    selection = selection_of(root, config)
    (root / ENTRY).write_text(selection.entry)
    config["titleMap"] = selection.title_map
    config["overrides"] = selection.overrides
    write_catalog(root, config, selection.catalog)
    config_file.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    try:
        write(args.root)
    except SelectionError as error:
        print(error, file=sys.stderr)
        return 1
    print(f"regeneradas. Versiona {ENTRY} y {CONFIG}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
