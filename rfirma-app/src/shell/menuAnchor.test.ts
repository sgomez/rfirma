import { describe, expect, it } from "vitest";
import { menuAnchorFor } from "./menuAnchor";

// Grada A: una función pura sobre una cadena.
describe("menuAnchorFor", () => {
  it("anchors the two entries in the header on linux", () => {
    expect(menuAnchorFor("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15")).toBe("header");
  });

  it("anchors the two entries in the header on windows", () => {
    expect(menuAnchorFor("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toBe("header");
  });

  it("hands the two entries to the native menu on macos", () => {
    expect(menuAnchorFor("Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)")).toBe("native");
  });
});
