import { describe, expect, it } from "vitest";
import {
  inMemoryUrlHandlers,
  theBannerHasSomethingToAsk,
  type UrlHandlers,
  weAlreadyHandleThem,
} from "./urlHandlers";

function whoHandles(overrides: Partial<UrlHandlers> = {}): UrlHandlers {
  return {
    available: true,
    handlers: [
      { id: "rfirma.desktop", name: "rFirma" },
      { id: "otra.desktop", name: "La otra" },
    ],
    current: "otra.desktop",
    ours: "rfirma.desktop",
    ...overrides,
  };
}

// Grada A: reglas puras, sin ventana y sin backend.
describe("who opens afirma:// links", () => {
  it("recognises rFirma by the desktop file it was given, not by its name", () => {
    expect(weAlreadyHandleThem(whoHandles({ current: "rfirma.desktop" }))).toBe(true);
    expect(weAlreadyHandleThem(whoHandles())).toBe(false);
  });

  it("asks at startup when somebody else opens them", () => {
    expect(theBannerHasSomethingToAsk(whoHandles(), true)).toBe(true);
  });

  it("asks when nobody has been chosen yet", () => {
    expect(theBannerHasSomethingToAsk(whoHandles({ current: null }), true)).toBe(true);
  });

  it("says nothing once rFirma already opens them", () => {
    expect(theBannerHasSomethingToAsk(whoHandles({ current: "rfirma.desktop" }), true)).toBe(false);
  });

  /** ID-240: dentro del flatpak no hay banner, porque no hay nada que elegir. */
  it("says nothing where the choice cannot be made at all", () => {
    expect(
      theBannerHasSomethingToAsk({ ...whoHandles(), available: false, handlers: [] }, true),
    ).toBe(false);
  });

  it("says nothing after «do not ask again»", () => {
    expect(theBannerHasSomethingToAsk(whoHandles(), false)).toBe(false);
  });

  it("says nothing while the desktop has not answered", () => {
    expect(theBannerHasSomethingToAsk(null, true)).toBe(false);
  });

  it("keeps the choice it was asked to write", async () => {
    const handlers = inMemoryUrlHandlers(whoHandles());

    await handlers.choose("rfirma.desktop");

    expect((await handlers.who()).current).toBe("rfirma.desktop");
  });
});
