import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { composeStories } from "@storybook/react-vite";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ICON_FAMILIES } from "./families";
import * as catalogueStories from "./IconCatalogue.stories";
import { ICON_NAMES } from "./names";

/** **Grada A**: cada icono de cada familia declara familia y licencia, y fuera del directorio nadie dibuja uno ni elige familia. */

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, "..", "..");

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(path);
    return /\.(tsx?|svg)$/.test(entry.name) && !/\.test\./.test(entry.name) ? [path] : [];
  });
}

const outsideIcons = sourceFiles(src).filter((path) => !path.startsWith(here));

describe("icon families", () => {
  it("offers the current family and Heroicons as candidates", () => {
    expect(ICON_FAMILIES.map((family) => family.name)).toEqual(["rFirma", "Heroicons"]);
  });

  it.each(ICON_FAMILIES)("draws every role in the $name family", (family) => {
    expect(Object.keys(family.glyphs).sort()).toEqual([...ICON_NAMES].sort());
    for (const name of ICON_NAMES) {
      const { container } = render(family.glyphs[name].Glyph({ size: 16 }));
      expect(container.querySelector("svg"), name).not.toBeNull();
    }
  });

  it.each(ICON_FAMILIES)("declares a family and a license for every icon of $name", (family) => {
    for (const [name, glyph] of Object.entries(family.glyphs)) {
      expect(glyph.origin.family.trim(), name).not.toBe("");
      expect(glyph.origin.license.trim(), name).not.toBe("");
    }
  });

  it("takes the smart card icon from Heroicons, under MIT, in every family", () => {
    for (const family of ICON_FAMILIES) {
      expect(family.glyphs.smartCard.origin).toEqual({ family: "Heroicons", license: "MIT" });
    }
  });

  it("draws the Heroicons candidate from Heroicons wherever Heroicons has the icon", () => {
    const heroicons = ICON_FAMILIES.find((family) => family.name === "Heroicons");
    const own = Object.entries(heroicons?.glyphs ?? {})
      .filter(([, glyph]) => glyph.origin.family !== "Heroicons")
      .map(([name]) => name);
    expect(own.sort()).toEqual(["loading", "rubric"]);
  });

  it("paints every role of every family side by side in the catalogue story", () => {
    const { Catalogue } = composeStories(catalogueStories);
    render(<Catalogue />);

    const [header, ...rows] = screen.getAllByRole("row");
    expect(
      within(header as HTMLElement)
        .getAllByRole("columnheader")
        .map((cell) => cell.textContent),
    ).toEqual(["Papel", "rFirma", "Heroicons"]);
    expect(rows.map((row) => within(row).getByRole("rowheader").textContent)).toEqual([
      ...ICON_NAMES,
    ]);
    for (const row of rows) {
      const cells = within(row).getAllByRole("cell");
      expect(cells.map((cell) => cell.querySelector("svg") !== null)).toEqual([true, true]);
    }
    const smartCard = rows[ICON_NAMES.indexOf("smartCard")] as HTMLElement;
    expect(
      within(smartCard)
        .getAllByRole("cell")
        .map((cell) => cell.textContent),
    ).toEqual(["Heroicons · MIT", "Heroicons · MIT"]);
  });

  it("keeps every svg outside the icons directory out of the interface", () => {
    const stray = outsideIcons.filter(
      (path) =>
        !/\.stories\./.test(path) &&
        (path.endsWith(".svg") ||
          (path.endsWith(".tsx") && /<svg\b/.test(readFileSync(path, "utf8")))),
    );
    expect(stray).toEqual([]);
  });

  it("lets no module outside the icons directory pick a family", () => {
    const picking = outsideIcons.filter((path) =>
      /from "[^"]*icons\/families/.test(readFileSync(path, "utf8")),
    );
    expect(picking).toEqual([]);
  });
});
