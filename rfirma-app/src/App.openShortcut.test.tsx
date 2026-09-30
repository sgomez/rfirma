import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { aDestination, document, pdfsOf, renderApp } from "./App.testSupport";
import { isOpenShortcut } from "./App.useOpenShortcut";
import { inMemoryDocumentDrops } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import { inMemoryRecents } from "./documents/recents";
import { inMemoryPreferences, type Preferences } from "./preferences/preferences";
import { emptyCertificateStore } from "./signing/certificate";
import { unavailableOpener } from "./signing/destination";
import { unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker } from "./signing/rubric";
import { unavailableStampComposer } from "./signing/stampPreview";
import { DEFAULT_VISIBLE_SIGNATURE } from "./signing/visibleSignature";
import { renderWithCatalog } from "./testing/render";
import { inMemoryVersionCheck } from "./updates/newVersion";

const LINUX = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15";
const MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15";

const keys = (overrides: Partial<Parameters<typeof isOpenShortcut>[0]> = {}) => ({
  key: "o",
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  repeat: false,
  ...overrides,
});

const aPreferences: Preferences = {
  theme: "system",
  destination: "Documentos",
  destinationMode: "next_to_the_original",
  offersOriginalFolder: false,
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  setupWizardSeen: false,
  consentCountdown: true,
  honourAutomaticSelection: false,
};

function openTabs() {
  const tabs = screen.getByRole("navigation", { name: "Documentos abiertos" });
  return within(tabs).queryAllByRole("tab");
}

describe("isOpenShortcut", () => {
  it("is Ctrl+O on Linux and Windows", () => {
    expect(isOpenShortcut(keys({ ctrlKey: true }), LINUX)).toBe(true);
    expect(isOpenShortcut(keys({ ctrlKey: true, key: "O" }), LINUX)).toBe(true);
    expect(isOpenShortcut(keys({ metaKey: true }), LINUX)).toBe(false);
  });

  it("is Cmd+O on macOS, not Ctrl+O", () => {
    expect(isOpenShortcut(keys({ metaKey: true }), MAC)).toBe(true);
    expect(isOpenShortcut(keys({ ctrlKey: true }), MAC)).toBe(false);
  });

  it("ignores O alone, other modifiers, other keys and a held key", () => {
    expect(isOpenShortcut(keys(), LINUX)).toBe(false);
    expect(isOpenShortcut(keys({ ctrlKey: true, shiftKey: true }), LINUX)).toBe(false);
    expect(isOpenShortcut(keys({ ctrlKey: true, altKey: true }), LINUX)).toBe(false);
    expect(isOpenShortcut(keys({ ctrlKey: true, key: "p" }), LINUX)).toBe(false);
    expect(isOpenShortcut(keys({ ctrlKey: true, repeat: true }), LINUX)).toBe(false);
  });
});

// Grada A: el atajo sobre la aplicación entera, con el mismo selector que el menú.
describe("App, el atajo Ctrl+O", () => {
  it("opens the system dialog and puts the chosen PDF in a tab", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 2 }));

    await user.keyboard("{Control>}o{/Control}");

    expect(await screen.findByRole("tab", { name: "factura.pdf", selected: true })).toBeVisible();
  });

  it("does nothing while About is open", async () => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 2 }));
    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: "Acerca de rFirma" }));
    const about = screen.getByRole("dialog");

    await user.keyboard("{Control>}o{/Control}");
    await user.click(within(about).getByRole("button", { name: "Cerrar" }));

    expect(openTabs()).toHaveLength(0);
    await user.keyboard("{Control>}o{/Control}");
    expect(await screen.findByRole("tab", { name: "factura.pdf" })).toBeVisible();
  });

  it.each([
    ["Preferencias…", "region", "Preferencias"],
    ["Estado de rFirma", "heading", "Estado de rFirma"],
  ] as const)("does nothing in %s", async (entry, role, name) => {
    const user = userEvent.setup();
    renderApp(inMemoryRecents(), [document("factura.pdf")], pdfsOf({ "factura.pdf": 2 }));
    await user.click(screen.getByRole("button", { name: "Menú" }));
    await user.click(screen.getByRole("menuitem", { name: entry }));
    await screen.findByRole(role, { name });

    await user.keyboard("{Control>}o{/Control}");
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    await waitFor(() => expect(openTabs()).toHaveLength(0));
    await user.keyboard("{Control>}o{/Control}");
    expect(await screen.findByRole("tab", { name: "factura.pdf" })).toBeVisible();
  });

  it("does nothing while the first-run wizard covers the window", async () => {
    const user = userEvent.setup();
    let asked = 0;
    const picker: DocumentPicker = {
      choose: async () => {
        asked += 1;
        return document("factura.pdf");
      },
    };
    renderWithCatalog(
      <App
        recents={inMemoryRecents()}
        picker={picker}
        drops={inMemoryDocumentDrops()}
        pdfs={pdfsOf({ "factura.pdf": 2 })}
        preferences={inMemoryPreferences(aPreferences)}
        destinations={aDestination()}
        certificates={emptyCertificateStore()}
        rubrics={emptyRubricPicker()}
        stamps={unavailableStampComposer()}
        signer={unavailableSigningBackend()}
        opener={unavailableOpener()}
        initialSignature={DEFAULT_VISIBLE_SIGNATURE}
        versions={inMemoryVersionCheck()}
        version="0.1.0"
        menuAnchor="header"
        covered
      />,
    );

    await user.keyboard("{Control>}o{/Control}");

    expect(asked).toBe(0);
    expect(openTabs()).toHaveLength(0);
  });
});
