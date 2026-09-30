"""Los tres verbos de `app_version.py` sobre un árbol temporal con su propio git."""

from __future__ import annotations

import datetime
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "app_version.py"

CARGO_TOML = """[package]
name = "rfirma"
version = "0.3.0"
edition = "2021"

[dependencies]
serde = { version = "1.0.200" }
"""

CARGO_LOCK = """version = 4

[[package]]
name = "serde"
version = "1.0.200"

[[package]]
name = "rfirma"
version = "0.3.0"
dependencies = [
 "serde",
]
"""

METAINFO = """<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>me.sgomez.rfirma</id>
  <name>rFirma</name>
  <releases>
    <release version="0.3.0" date="2026-01-01">
      <url type="details">https://github.com/sgomez/rfirma/blob/main/CHANGELOG.md</url>
    </release>
  </releases>
</component>
"""

CHANGELOG = """# Changelog

## [0.3.0] - 2026-01-01

### Fixed
- Algo arreglado (#3).

## [0.2.0] - 2025-12-01

### Added
- Algo nuevo (#2).
"""

TAURI_CONF = {"productName": "rfirma", "app": {"windows": [{"title": "rFirma"}]}}

README = "Descarga: https://github.com/sgomez/rfirma/releases/latest/download/rfirma.flatpak\n"

TODAY = datetime.datetime.now(datetime.UTC).date().isoformat()


class AppVersionTree(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        self.write("rfirma-app/src-tauri/Cargo.toml", CARGO_TOML)
        self.write("rfirma-app/src-tauri/Cargo.lock", CARGO_LOCK)
        self.write("rfirma-app/src-tauri/tauri.conf.json", json.dumps(TAURI_CONF))
        self.write("packaging/flatpak/me.sgomez.rfirma.metainfo.xml", METAINFO)
        self.write("packaging/flatpak/sources.lock", "")
        self.write("CHANGELOG.md", CHANGELOG)
        self.write("README.md", README)
        self.git("init", "--quiet", "--initial-branch=main")
        self.git("add", "-A")
        self.git("commit", "--quiet", "-m", "chore: version 0.3.0")
        self.git("tag", "v0.3.0")

    def write(self, relative: str, content: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def read(self, relative: str) -> str:
        return (self.root / relative).read_text(encoding="utf-8")

    def replace(self, relative: str, old: str, new: str) -> None:
        self.write(relative, self.read(relative).replace(old, new, 1))

    def git(self, *args: str) -> None:
        subprocess.run(
            [
                "git",
                "-C",
                str(self.root),
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "tag.gpgsign=false",
                *args,
            ],
            check=True,
            capture_output=True,
        )

    def merge_pr(self, number: int, title: str) -> None:
        self.git(
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            f"Merge pull request #{number} from sgomez/rama",
            "-m",
            title,
        )

    def run_verb(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *args],
            capture_output=True,
            text=True,
            check=False,
        )


class Check(AppVersionTree):
    def assert_check_fails_with(self, fragment: str, *args: str) -> None:
        result = self.run_verb("check", *args)
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn(fragment, result.stderr)

    def test_prints_the_version_when_every_site_agrees(self) -> None:
        result = self.run_verb("check")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "0.3.0")

    def test_accepts_the_expected_version_with_or_without_the_tag_prefix(self) -> None:
        self.assertEqual(self.run_verb("check", "--expected", "0.3.0").returncode, 0)
        self.assertEqual(self.run_verb("check", "--expected", "v0.3.0").returncode, 0)

    def test_rejects_a_tree_that_is_not_the_expected_version(self) -> None:
        self.assert_check_fails_with("esperaba '0.4.0'", "--expected", "v0.4.0")

    def test_detects_a_cargo_lock_out_of_step(self) -> None:
        self.replace(
            "rfirma-app/src-tauri/Cargo.lock",
            'name = "rfirma"\nversion = "0.3.0"',
            'name = "rfirma"\nversion = "0.2.0"',
        )
        self.assert_check_fails_with("Cargo.lock dice '0.2.0'")

    def test_detects_a_metainfo_out_of_step(self) -> None:
        self.replace(
            "packaging/flatpak/me.sgomez.rfirma.metainfo.xml",
            'version="0.3.0"',
            'version="0.2.0"',
        )
        self.assert_check_fails_with("publica la versión '0.2.0'")

    def test_detects_a_changelog_without_the_section_of_the_version(self) -> None:
        self.replace(
            "CHANGELOG.md", "## [0.3.0] - 2026-01-01", "## [0.2.1] - 2026-01-01"
        )
        self.assert_check_fails_with(
            "la sección más reciente de CHANGELOG.md es '0.2.1'"
        )

    def test_detects_a_version_declared_in_tauri_conf(self) -> None:
        self.write(
            "rfirma-app/src-tauri/tauri.conf.json",
            json.dumps({**TAURI_CONF, "version": "0.3.0"}),
        )
        self.assert_check_fails_with("tauri.conf.json declara `version`")

    def test_detects_release_notes_copied_into_the_metainfo(self) -> None:
        self.replace(
            "packaging/flatpak/me.sgomez.rfirma.metainfo.xml",
            "</url>",
            "</url>\n      <description><p>Notas</p></description>",
        )
        self.assert_check_fails_with("copia las notas en <description>")

    def test_detects_a_versioned_download_link_in_the_readme(self) -> None:
        self.write("README.md", README + "releases/download/v0.3.0/rfirma.deb\n")
        self.assert_check_fails_with("descarga con versión dentro")

    def test_detects_a_product_name_written_as_prose(self) -> None:
        self.write(
            "rfirma-app/src-tauri/tauri.conf.json",
            json.dumps({**TAURI_CONF, "productName": "rFirma"}),
        )
        self.assert_check_fails_with("productName es 'rFirma'")


class Bump(AppVersionTree):
    def test_leaves_every_site_in_step(self) -> None:
        result = self.run_verb("bump", "0.4.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "0.4.0")
        self.assertIn(
            'version = "0.4.0"\nedition', self.read("rfirma-app/src-tauri/Cargo.toml")
        )
        self.assertIn(
            'serde = { version = "1.0.200" }',
            self.read("rfirma-app/src-tauri/Cargo.toml"),
        )
        lock = self.read("rfirma-app/src-tauri/Cargo.lock")
        self.assertIn('name = "rfirma"\nversion = "0.4.0"', lock)
        self.assertIn('name = "serde"\nversion = "1.0.200"', lock)
        self.assertIn(
            f'<release version="0.4.0" date="{TODAY}">',
            self.read("packaging/flatpak/me.sgomez.rfirma.metainfo.xml"),
        )
        self.assertTrue(
            self.read("CHANGELOG.md").split("## [")[1].startswith(f"0.4.0] - {TODAY}")
        )
        digest = hashlib.sha256(
            (self.root / "rfirma-app/src-tauri/Cargo.lock").read_bytes()
        )
        self.assertEqual(
            self.read("packaging/flatpak/sources.lock"),
            f"{digest.hexdigest()}  rfirma-app/src-tauri/Cargo.lock\n",
        )

    def test_writes_the_changelog_from_the_titles_of_the_merged_prs(self) -> None:
        self.merge_pr(10, "feat(ui): firma con rúbrica")
        self.merge_pr(11, "ci: cachea maven")
        self.merge_pr(12, "fix(ci): repara el carril")
        self.merge_pr(13, "fix: no se cuelga al cancelar.")
        self.merge_pr(14, "perf: arranca antes")
        self.assertEqual(self.run_verb("bump", "0.4.0").returncode, 0)
        self.assertEqual(
            self.run_verb("changelog", "0.4.0").stdout.strip(),
            "### Added\n- Firma con rúbrica (#10).\n\n"
            "### Changed\n- Arranca antes (#14).\n\n"
            "### Fixed\n- No se cuelga al cancelar (#13).",
        )

    def test_says_so_when_nothing_visible_changed(self) -> None:
        self.merge_pr(11, "ci: cachea maven")
        self.run_verb("bump", "0.3.1")
        self.assertEqual(
            self.run_verb("changelog", "0.3.1").stdout.strip(),
            "Sin cambios visibles para quien usa rFirma.",
        )

    def test_refuses_a_version_the_changelog_already_has_and_touches_nothing(
        self,
    ) -> None:
        result = self.run_verb("bump", "0.2.0")
        self.assertEqual(result.returncode, 1)
        self.assertIn("ya tiene una sección para 0.2.0", result.stderr)
        self.assertEqual(self.read("rfirma-app/src-tauri/Cargo.toml"), CARGO_TOML)

    def test_refuses_something_that_is_not_a_version(self) -> None:
        result = self.run_verb("bump", "v0.4")
        self.assertEqual(result.returncode, 1)
        self.assertIn("X.Y.Z", result.stderr)
        self.assertEqual(self.read("CHANGELOG.md"), CHANGELOG)

    def test_refuses_a_repository_without_a_previous_tag(self) -> None:
        self.git("tag", "-d", "v0.3.0")
        result = self.run_verb("bump", "0.4.0")
        self.assertEqual(result.returncode, 1)
        self.assertIn("etiqueta v*", result.stderr)
        self.assertEqual(self.read("rfirma-app/src-tauri/Cargo.lock"), CARGO_LOCK)


class Changelog(AppVersionTree):
    def test_extracts_the_section_of_a_version(self) -> None:
        result = self.run_verb("changelog", "0.2.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "### Added\n- Algo nuevo (#2).")

    def test_extracts_the_section_of_the_current_version_without_argument(self) -> None:
        self.assertEqual(
            self.run_verb("changelog").stdout.strip(),
            "### Fixed\n- Algo arreglado (#3).",
        )

    def test_fails_naming_a_version_without_section(self) -> None:
        result = self.run_verb("changelog", "9.9.9")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no tiene sección para 9.9.9", result.stderr)


if __name__ == "__main__":
    unittest.main()
