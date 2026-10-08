import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { ICON_ORIGINS } from "./origins";

/** **Grada A**: cada icono del directorio declara familia y licencia, y ningún otro módulo dibuja uno. */

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, "..", "..");

const exportedIcons = [
  ...readFileSync(join(here, "index.tsx"), "utf8").matchAll(/^export function (\w+Icon)\b/gm),
].map(([, name]) => name as string);

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(path);
    return /\.(tsx|svg)$/.test(entry.name) && !/\.(test|stories)\./.test(entry.name) ? [path] : [];
  });
}

describe("icon origins", () => {
  it("declares a family and a license for every exported icon", () => {
    expect(Object.keys(ICON_ORIGINS).sort()).toEqual([...exportedIcons].sort());
    for (const [name, origin] of Object.entries(ICON_ORIGINS)) {
      expect(origin.family.trim(), name).not.toBe("");
      expect(origin.license.trim(), name).not.toBe("");
    }
  });

  it("includes the Heroicons identification icon under MIT", () => {
    expect(exportedIcons).toContain("IdentificationIcon");
    expect(ICON_ORIGINS.IdentificationIcon).toEqual({ family: "Heroicons", license: "MIT" });
  });

  it("keeps every svg outside the icons directory out of the interface", () => {
    const stray = sourceFiles(src).filter(
      (path) =>
        !path.startsWith(here) &&
        (path.endsWith(".svg") || /<svg\b/.test(readFileSync(path, "utf8"))),
    );
    expect(stray).toEqual([]);
  });
});
