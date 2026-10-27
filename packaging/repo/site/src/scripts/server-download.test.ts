import { describe, expect, it } from "vitest";
import { packageUrl } from "./server-download";

const origin = "https://rfirma.com";

describe("packageUrl", () => {
  it("offers the Windows installer the feed announces", () => {
    const feed = {
      platforms: { "windows-x86_64": { url: `${origin}/windows/rFirma_1.0.0_x64-setup.exe` } },
    };
    expect(packageUrl(feed, "windows", origin)).toBe(
      `${origin}/windows/rFirma_1.0.0_x64-setup.exe`,
    );
  });

  it("offers the macOS dmg next to the update the feed announces", () => {
    const feed = {
      platforms: { "darwin-aarch64": { url: `${origin}/macos/rfirma_1.0.0_aarch64.app.tar.gz` } },
    };
    expect(packageUrl(feed, "macos", origin)).toBe(`${origin}/macos/rfirma_1.0.0_aarch64.dmg`);
  });

  it("serves the installer from the page's own server whatever domain the feed names", () => {
    const feed = {
      platforms: {
        "windows-x86_64": { url: "https://rfirma.sgomez.me/windows/rFirma_1.0.0_x64-setup.exe" },
      },
    };
    expect(packageUrl(feed, "windows", origin)).toBe(
      `${origin}/windows/rFirma_1.0.0_x64-setup.exe`,
    );
  });

  it("keeps the Release when the feed points outside its own folder", () => {
    const feed = {
      platforms: {
        "darwin-aarch64": { url: `${origin}/downloads/rfirma_1.0.0_aarch64.app.tar.gz` },
      },
    };
    expect(packageUrl(feed, "macos", origin)).toBeUndefined();
  });

  it("keeps the Release when the feed announces something that is not a URL", () => {
    const feed = { platforms: { "windows-x86_64": { url: "rFirma_1.0.0_x64-setup.exe" } } };
    expect(packageUrl(feed, "windows", origin)).toBeUndefined();
  });

  it("keeps the Release when the feed lacks the platform", () => {
    expect(packageUrl({ platforms: {} }, "macos", origin)).toBeUndefined();
  });
});
