"""Cada invariante de `check_launchers.py` falla al romperlo, sobre un arbol temporal."""

from __future__ import annotations

import copy
import json
import os
import shutil
import tempfile
import unittest

import check_launchers as launchers

SERVICEMENU = """[Desktop Entry]
Type=Service
MimeType=application/pdf;
X-KDE-Priority=TopLevel
X-KDE-RequiredNumberOfUrls=1

[Desktop Action sign]
Name=Firmar con rFirma
Exec=rfirma %f
"""

LAUNCHER = """[Desktop Entry]
Type=Application
Name=rFirma
MimeType=x-scheme-handler/afirma;
"""

CONF = {
    "bundle": {
        "linux": {
            target: {
                "desktopTemplate": "../../packaging/rfirma.desktop.hbs",
                "files": {
                    launchers.SERVICEMENU_TARGET: "../../" + launchers.KDE_SERVICEMENU
                },
            }
            for target in ("deb", "rpm")
        }
    }
}


class LauncherChecks(unittest.TestCase):
    def setUp(self) -> None:
        self.root = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.root)
        self.write_conf(CONF)
        self.write_servicemenu(SERVICEMENU)
        self.write("packaging/rfirma.desktop.hbs", LAUNCHER)
        self.write(launchers.FLATPAK_MANIFEST, "app-id: me.sgomez.rfirma\n")

    def write(self, path: str, content: str, executable: bool = False) -> None:
        full = os.path.join(self.root, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w", encoding="utf-8") as handle:
            handle.write(content)
        os.chmod(full, 0o755 if executable else 0o644)

    def write_conf(self, conf: dict) -> None:
        self.write(launchers.TAURI_CONF, json.dumps(conf))

    def write_servicemenu(self, content: str, executable: bool = True) -> None:
        self.write(launchers.KDE_SERVICEMENU, content, executable)

    def assert_fails_with(self, fragment: str) -> None:
        failures = launchers.check(self.root)
        self.assertTrue(
            any(fragment in message for message in failures),
            f"{fragment!r} en {failures}",
        )

    def test_accepts_a_correct_tree(self) -> None:
        self.assertEqual(launchers.check(self.root), [])

    def test_rejects_a_pdf_mime_type_in_a_launcher(self) -> None:
        self.write(
            "packaging/gnome/rfirma.desktop",
            LAUNCHER.replace("x-scheme-handler/afirma", "application/pdf"),
        )
        self.assert_fails_with("MimeType=application/pdf")

    def test_rejects_a_pdf_mime_type_in_the_bundler_template(self) -> None:
        self.write(
            "packaging/rfirma.desktop.hbs", LAUNCHER + "MimeType=application/pdf;\n"
        )
        self.assert_fails_with("MimeType=application/pdf")

    def test_rejects_a_scheme_other_than_afirma(self) -> None:
        self.write("packaging/rfirma.desktop.hbs", LAUNCHER.replace("afirma", "otro"))
        self.assert_fails_with("x-scheme-handler/otro")

    def test_rejects_a_template_outside_the_inspected_tree(self) -> None:
        conf = copy.deepcopy(CONF)
        conf["bundle"]["linux"]["deb"]["desktopTemplate"] = "fuera.desktop.hbs"
        self.write_conf(conf)
        self.assert_fails_with("queda fuera")

    def test_rejects_a_bundle_without_a_template(self) -> None:
        conf = copy.deepcopy(CONF)
        del conf["bundle"]["linux"]["rpm"]["desktopTemplate"]
        self.write_conf(conf)
        self.assert_fails_with("no declara `desktopTemplate`")

    def test_rejects_a_launcher_not_named_rfirma(self) -> None:
        self.write(
            "packaging/rfirma.desktop.hbs",
            LAUNCHER.replace("Name=rFirma", "Name=rfirma"),
        )
        self.assert_fails_with("Name=rfirma")

    def test_rejects_a_changed_verb(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("Firmar con rFirma", "Firmar"))
        self.assert_fails_with("El verbo del menu")

    def test_rejects_a_servicemenu_that_is_not_a_service(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("Type=Service", "Type=Application"))
        self.assert_fails_with("no es un `Type=Service`")

    def test_rejects_a_servicemenu_without_the_pdf_filter(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("application/pdf", "text/plain"))
        self.assert_fails_with("no filtra por `application/pdf`")

    def test_rejects_a_servicemenu_below_the_top_level(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("TopLevel", "Normal"))
        self.assert_fails_with("X-KDE-Priority=TopLevel")

    def test_rejects_a_servicemenu_that_accepts_several_files(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("Urls=1", "Urls=2"))
        self.assert_fails_with("RequiredNumberOfUrls=1")

    def test_rejects_an_exec_with_a_file_list(self) -> None:
        self.write_servicemenu(SERVICEMENU.replace("%f", "%F"))
        self.assert_fails_with("`%F`")

    def test_rejects_a_servicemenu_without_the_execute_bit(self) -> None:
        self.write_servicemenu(SERVICEMENU, executable=False)
        self.assert_fails_with("bit de ejecucion")

    def test_rejects_a_package_that_does_not_install_the_servicemenu(self) -> None:
        conf = copy.deepcopy(CONF)
        conf["bundle"]["linux"]["deb"]["files"] = {}
        self.write_conf(conf)
        self.assert_fails_with("no instala")

    def test_rejects_the_servicemenu_inside_the_flatpak(self) -> None:
        self.write(launchers.FLATPAK_MANIFEST, "- /app/share/kio/servicemenus/x\n")
        self.assert_fails_with("instala el servicemenu")


if __name__ == "__main__":
    unittest.main()
