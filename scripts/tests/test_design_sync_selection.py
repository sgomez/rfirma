"""La selección de Claude Design que `design_sync_selection.py` deriva de un árbol de historias temporal."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent.parent / "design_sync_selection.py"

CONFIG = {
    "pkg": "rfirma-app",
    "entry": "rfirma-app/design-sync.entry.ts",
    "titleMap": {"Viejo": "Viejo"},
    "provider": {"component": "DesignRoot"},
}

EXPORT_LINE = re.compile(r'^export \{ ([^}]+) \} from "([^"]+)";$', re.MULTILINE)


def story(title: str, component: str, imports: str, parameters: str = "") -> str:
    extra = f"  parameters: {{ {parameters} }},\n" if parameters else ""
    return (
        'import type { Meta, StoryObj } from "@storybook/react-vite";\n'
        f"{imports}\n\n"
        f'const meta = {{\n  title: "{title}",\n  component: {component},\n{extra}}} '
        f"satisfies Meta<typeof {component}>;\n\n"
        "export default meta;\n"
    )


class SelectionTest(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp())
        (self.root / ".design-sync").mkdir()
        (self.root / ".design-sync/config.json").write_text(
            json.dumps(CONFIG, indent=2) + "\n"
        )
        (self.root / "rfirma-app/src").mkdir(parents=True)

    def tearDown(self) -> None:
        shutil.rmtree(self.root)

    def add_story(self, path: str, content: str) -> None:
        file = self.root / "rfirma-app/src" / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)

    def run_script(self, verb: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), verb, "--root", str(self.root)],
            capture_output=True,
            text=True,
            check=False,
        )

    def write(self) -> None:
        result = self.run_script("write")
        self.assertEqual(result.returncode, 0, result.stderr)

    def exported(self) -> dict[str, str]:
        entry = (self.root / "rfirma-app/design-sync.entry.ts").read_text()
        return {
            name.strip(): module
            for names, module in EXPORT_LINE.findall(entry)
            for name in names.split(",")
            if name.strip() != "DesignRoot"
        }

    def title_map(self) -> dict[str, str]:
        return json.loads((self.root / ".design-sync/config.json").read_text())[
            "titleMap"
        ]

    def test_publishable_layers_enter_by_regenerating(self) -> None:
        self.add_story(
            "design-system/Button.stories.tsx",
            story("Primitivos/Button", "Button", 'import { Button } from "./Button";'),
        )
        self.add_story(
            "documents/RecentRows.stories.tsx",
            story(
                "Dominio/Documentos/RecentRows",
                "RecentsSection",
                'import { RecentRows, RecentsSection } from "./RecentRows";',
            ),
        )
        self.add_story(
            "sede/SedeWaiting.stories.tsx",
            story(
                "Flujos/Sede/Marcar la firma",
                "SedeView",
                'import type { SedeView } from "./SedeView";',
            ),
        )

        self.write()

        self.assertEqual(
            self.exported(),
            {
                "Button": "./src/design-system/Button",
                "RecentRows": "./src/documents/RecentRows",
                "RecentsSection": "./src/documents/RecentRows",
                "SedeView": "./src/sede/SedeView",
            },
        )
        self.assertEqual(
            self.title_map(),
            {
                "Button": "Button",
                "RecentRows": "RecentsSection",
                "Marcarlafirma": "SedeView",
            },
        )

    def test_provider_and_icons_always_enter(self) -> None:
        self.write()

        entry = (self.root / "rfirma-app/design-sync.entry.ts").read_text()
        self.assertIn("export { DesignRoot }", entry)
        self.assertIn("export * from", entry)

    def test_screens_never_enter(self) -> None:
        self.add_story(
            "status/StatusView.stories.tsx",
            story(
                "Pantallas/Estado/1 · Panel",
                "StatusView",
                'import { StatusView } from "./StatusView";',
            ),
        )

        self.write()

        self.assertNotIn("StatusView", self.exported())
        self.assertEqual(self.title_map(), {})

    def test_demo_component_resolves_to_the_title_piece(self) -> None:
        self.add_story(
            "design-system/Menu.stories.tsx",
            story(
                "Primitivos/Menu",
                "MenuDemo",
                'import { Menu, MenuItem } from "./Menu";',
            ),
        )

        self.write()

        self.assertEqual(
            self.exported(),
            {
                "Menu": "./src/design-system/Menu",
                "MenuItem": "./src/design-system/Menu",
            },
        )
        self.assertEqual(self.title_map(), {"Menu": "Menu"})

    def test_component_from_a_testing_folder_is_importable(self) -> None:
        self.add_story(
            "signing/SigningPanel.stories.tsx",
            story(
                "Flujos/Firma/SigningPanel",
                "SigningPanelHarness",
                'import { SigningPanelHarness } from "./testing/harness";\n'
                'import { certificate } from "./testing/fixtures";',
            ),
        )

        self.write()

        self.assertEqual(
            self.exported(), {"SigningPanelHarness": "./src/signing/testing/harness"}
        )

    def assert_fails_naming(self, *culprits: str) -> None:
        result = self.run_script("write")
        self.assertNotEqual(result.returncode, 0)
        for culprit in culprits:
            self.assertIn(culprit, result.stderr)
        self.assertEqual(self.title_map(), {"Viejo": "Viejo"})

    def test_unknown_layer_fails(self) -> None:
        self.add_story(
            "signing/Panel.stories.tsx",
            story("Firma/Panel", "Panel", 'import { Panel } from "./Panel";'),
        )

        self.assert_fails_naming("signing/Panel.stories.tsx", "Firma")

    def test_title_out_of_convention_fails(self) -> None:
        self.add_story(
            "signing/Panel.stories.tsx",
            story("Flujos/Panel", "Panel", 'import { Panel } from "./Panel";'),
        )
        self.add_story(
            "signing/Dialog.stories.tsx",
            story(
                "Flujos/Firma/2 · Dialog",
                "Dialog",
                'import { Dialog } from "./Dialog";',
            ),
        )

        self.assert_fails_naming(
            "signing/Panel.stories.tsx", "signing/Dialog.stories.tsx"
        )

    def test_unmapped_publishable_component_fails(self) -> None:
        self.add_story(
            "signing/Panel.stories.tsx",
            story(
                "Flujos/Firma/Panel", "PanelDemo", 'import { Other } from "./Other";'
            ),
        )

        self.assert_fails_naming("signing/Panel.stories.tsx", "PanelDemo")

    def test_colliding_title_pieces_fail(self) -> None:
        self.add_story(
            "signing/Panel.stories.tsx",
            story("Flujos/Firma/Panel", "Panel", 'import { Panel } from "./Panel";'),
        )
        self.add_story(
            "status/Panel.stories.tsx",
            story("Dominio/Estado/Panel", "Panel", 'import { Panel } from "./Panel";'),
        )

        self.assert_fails_naming(
            "signing/Panel.stories.tsx", "status/Panel.stories.tsx"
        )

    def test_seal_detects_a_forgotten_regeneration(self) -> None:
        self.add_story(
            "design-system/Button.stories.tsx",
            story("Primitivos/Button", "Button", 'import { Button } from "./Button";'),
        )
        self.write()
        self.assertEqual(self.run_script("check").returncode, 0)

        self.add_story(
            "design-system/Card.stories.tsx",
            story("Primitivos/Card", "Card", 'import { Card } from "./Card";'),
        )

        result = self.run_script("check")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("just design-sync-selection", result.stderr)

    def test_seal_detects_a_hand_edited_selection(self) -> None:
        self.add_story(
            "design-system/Button.stories.tsx",
            story("Primitivos/Button", "Button", 'import { Button } from "./Button";'),
        )
        self.write()
        entry = self.root / "rfirma-app/design-sync.entry.ts"
        entry.write_text(
            entry.read_text() + 'export { Card } from "./src/design-system/Card";\n'
        )

        result = self.run_script("check")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("design-sync.entry.ts", result.stderr)

    def overrides(self) -> dict[str, dict[str, str]]:
        return json.loads((self.root / ".design-sync/config.json").read_text())[
            "overrides"
        ]

    def add_dialog_story(self, parameters: str) -> None:
        self.add_story(
            "design-system/Dialog.stories.tsx",
            story(
                "Primitivos/Dialog",
                "Dialog",
                'import { Dialog } from "./Dialog";',
                parameters,
            ),
        )

    def test_override_declared_in_the_story_reaches_the_config(self) -> None:
        self.add_dialog_story(
            'layout: "centered", designSync: { cardMode: "single", '
            'primaryStory: "Closable", viewport: "1340x780" }'
        )

        self.write()

        self.assertEqual(
            self.overrides(),
            {
                "Dialog": {
                    "cardMode": "single",
                    "primaryStory": "Closable",
                    "viewport": "1340x780",
                }
            },
        )

    def test_stale_override_is_dropped_on_regeneration(self) -> None:
        config = self.root / ".design-sync/config.json"
        config.write_text(
            json.dumps({**CONFIG, "overrides": {"Gone": {"cardMode": "column"}}})
        )

        self.write()

        self.assertEqual(self.overrides(), {})

    def test_override_on_a_screen_fails(self) -> None:
        self.add_story(
            "status/StatusView.stories.tsx",
            story(
                "Pantallas/Estado/1 · Panel",
                "StatusView",
                'import { StatusView } from "./StatusView";',
                'designSync: { cardMode: "column" }',
            ),
        )

        self.assert_fails_naming("status/StatusView.stories.tsx", "designSync")

    def test_unknown_override_key_fails(self) -> None:
        self.add_dialog_story('designSync: { colour: "red" }')

        self.assert_fails_naming("design-system/Dialog.stories.tsx", "colour")

    def test_conflicting_overrides_for_one_component_fail(self) -> None:
        for name, mode in (("SedeA", "single"), ("SedeB", "column")):
            self.add_story(
                f"sede/{name}.stories.tsx",
                story(
                    f"Flujos/Sede/{name}",
                    "SedeView",
                    'import { SedeView } from "./SedeView";',
                    f'designSync: {{ cardMode: "{mode}" }}',
                ),
            )

        self.assert_fails_naming("sede/SedeA.stories.tsx", "sede/SedeB.stories.tsx")

    def test_seal_detects_a_hand_edited_override(self) -> None:
        self.add_dialog_story('designSync: { cardMode: "single" }')
        self.write()
        config = self.root / ".design-sync/config.json"
        edited = json.loads(config.read_text())
        edited["overrides"]["Dialog"]["cardMode"] = "column"
        config.write_text(json.dumps(edited))

        result = self.run_script("check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("overrides", result.stderr)

    def use_conventions(self, prose: str) -> Path:
        config = self.root / ".design-sync/config.json"
        config.write_text(
            json.dumps({**CONFIG, "readmeHeader": ".design-sync/conventions.md"})
        )
        header = self.root / ".design-sync/conventions.md"
        header.write_text(prose)
        return header

    def test_catalog_tables_regenerate_between_markers(self) -> None:
        header = self.use_conventions(
            "Prosa antes.\n\n<!-- design-sync:catalog:start -->\nviejo\n"
            "<!-- design-sync:catalog:end -->\n\nProsa despues.\n"
        )
        self.add_story(
            "design-system/Button.stories.tsx",
            story("Primitivos/Button", "Button", 'import { Button } from "./Button";'),
        )
        self.add_story(
            "sede/SedeWaiting.stories.tsx",
            story(
                "Flujos/Sede/Espera",
                "SedeView",
                'import { SedeView } from "./SedeView";',
            ),
        )
        self.add_story(
            "status/StatusView.stories.tsx",
            story(
                "Pantallas/Estado/1 · Panel",
                "StatusView",
                'import { StatusView } from "./StatusView";',
            ),
        )

        self.write()

        text = header.read_text()
        self.assertTrue(text.startswith("Prosa antes.\n"))
        self.assertTrue(text.endswith("\nProsa despues.\n"))
        self.assertNotIn("viejo", text)
        self.assertIn("| Primitivos | Button | `Button` |", text)
        self.assertIn("| Flujos | Sede / Espera | `SedeView` |", text)
        self.assertNotIn("StatusView", text)
        self.assertEqual(self.run_script("check").returncode, 0)

    def test_seal_detects_a_hand_edited_catalog(self) -> None:
        header = self.use_conventions(
            "<!-- design-sync:catalog:start -->\n<!-- design-sync:catalog:end -->\n"
        )
        self.write()
        header.write_text(
            header.read_text().replace(
                "<!-- design-sync:catalog:end -->",
                "editado\n<!-- design-sync:catalog:end -->",
            )
        )

        result = self.run_script("check")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("catálogo", result.stderr)

    def test_missing_catalog_markers_fail(self) -> None:
        self.use_conventions("sin marcadores\n")

        self.assert_fails_naming("conventions.md", "marcadores")


if __name__ == "__main__":
    unittest.main()
