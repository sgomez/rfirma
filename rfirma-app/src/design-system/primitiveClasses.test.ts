import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * **Grada A** (`vitest`, carril rápido).
 *
 * Una clase `rf-*` que ya tiene primitivo no se escribe a mano fuera del
 * sistema de diseño: se compone con `Button`, `Card`, `Field`, `Badge`,
 * `Dialog`, `ProgressBar`, `Stack` o `Row`. Las clases sin primitivo —texto, `rf-input`,
 * `rf-divider`…— siguen siendo clases.
 */

const srcRoot = `${resolve(process.cwd(), "src")}/`;

const covered =
  /\brf-(btn|card|field|badge|progress|dialog|scrim|stack|row|gap)(?:--[a-z]+|-[a-z]+)?\b/g;

const primitiveOf: Record<string, string> = {
  btn: "Button",
  card: "Card",
  field: "Field",
  badge: "Badge",
  progress: "ProgressBar",
  dialog: "Dialog",
  scrim: "Dialog",
  stack: "Stack",
  row: "Row",
  gap: "Stack o Row",
};

const NOT_A_DIV = "el primitivo pinta un div y aquí el elemento es otro";

/**
 * Lo que las tandas dejaron escrito a mano, con su motivo. Es por fichero y
 * por clase: un `rf-btn` nuevo en uno de estos ficheros sigue fallando.
 */
const exceptions: Record<string, { classes: string[]; reason: string }> = {
  "signing/PanelFooter.tsx": { classes: ["rf-row", "rf-gap-xs"], reason: NOT_A_DIV },
  "signing/SignedPanel.tsx": { classes: ["rf-row", "rf-gap-xs"], reason: NOT_A_DIV },
  "signing/SignatureCards.tsx": { classes: ["rf-card"], reason: NOT_A_DIV },
  "setup/SetupWizard.tsx": { classes: ["rf-stack", "rf-row", "rf-gap-xs"], reason: NOT_A_DIV },
  "sede/SedeFrame.tsx": { classes: ["rf-row", "rf-gap-xs"], reason: NOT_A_DIV },
  "sede/SedeConsent.tsx": { classes: ["rf-stack"], reason: NOT_A_DIV },
  "sede/SedeWaiting.tsx": { classes: ["rf-stack", "rf-row", "rf-gap-xs"], reason: NOT_A_DIV },
  "preferences/PreferencesSections.tsx": {
    classes: ["rf-row", "rf-gap-xs"],
    reason: NOT_A_DIV,
  },
  "status/WithdrawCertificateView.tsx": {
    classes: ["rf-stack", "rf-row", "rf-gap-xs"],
    reason: NOT_A_DIV,
  },
};

function sourcesOutsideTheDesignSystem(): string[] {
  const listed = execFileSync("git", ["ls-files", "--", "*.tsx"], {
    cwd: srcRoot,
    encoding: "utf8",
  });
  return listed
    .split("\n")
    .filter(Boolean)
    .filter((path) => !path.startsWith("design-system/") && !path.endsWith(".test.tsx"));
}

const withoutCommentLines = (source: string) =>
  source
    .split("\n")
    .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
    .join("\n");

function handWrittenClasses(path: string): string[] {
  const found = withoutCommentLines(readFileSync(srcRoot + path, "utf8")).match(covered) ?? [];
  return [...new Set(found)];
}

describe("las clases que ya tienen primitivo", () => {
  it("no se escriben a mano fuera del sistema de diseño", () => {
    const offences: string[] = [];
    for (const path of sourcesOutsideTheDesignSystem()) {
      const allowed = exceptions[path]?.classes ?? [];
      for (const className of handWrittenClasses(path)) {
        if (allowed.includes(className)) continue;
        const family = /^rf-([a-z]+)/.exec(className)?.[1] ?? "";
        offences.push(`${path}: .${className} se compone con ${primitiveOf[family]}`);
      }
    }

    expect(offences).toEqual([]);
  });

  it("no deja excepciones que ya no hacen falta", () => {
    const stale: string[] = [];
    const sources = new Set(sourcesOutsideTheDesignSystem());
    for (const [path, { classes }] of Object.entries(exceptions)) {
      const used = sources.has(path) ? handWrittenClasses(path) : [];
      for (const className of classes) {
        if (!used.includes(className)) stale.push(`${path}: .${className}`);
      }
    }

    expect(stale).toEqual([]);
  });
});
