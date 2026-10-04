import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import { localePath, locales } from "./i18n";

const site = "https://rfirma.sgomez.me";
const dist = fileURLToPath(new URL("../dist/", import.meta.url));

function published(path: string): string {
  const file = join(dist, path);
  if (!existsSync(file)) {
    throw new Error(`${path} no está en dist/: compila la landing con \`astro build\` antes`);
  }
  return readFileSync(file, "utf8");
}

function page(path: string): Document {
  return new JSDOM(published(path)).window.document;
}

function landingFile(locale: (typeof locales)[number]): string {
  return `${localePath(locale).slice(1)}index.html`;
}

function attribute(document: Document, selector: string, name: string): string | null {
  return document.querySelector(selector)?.getAttribute(name) ?? null;
}

const landings = locales.map((locale) => ({
  name: `landing ${locale}`,
  file: landingFile(locale),
}));
const pages = [...landings, { name: "404", file: "404.html" }];

describe.each(pages)("$name", ({ file }) => {
  const document = page(file);

  it("has a title and a description", () => {
    expect(document.title.trim()).not.toBe("");
    expect(attribute(document, "meta[name='description']", "content")?.trim()).toBeTruthy();
  });

  it("declares an absolute canonical on the site", () => {
    const canonical = attribute(document, "link[rel='canonical']", "href") ?? "";
    expect(new URL(canonical).origin).toBe(site);
  });

  it("has a single h1", () => {
    expect(document.querySelectorAll("h1")).toHaveLength(1);
  });

  it("links a favicon in SVG and PNG and an apple-touch-icon that are published", () => {
    const icons = [
      "link[rel='icon'][type='image/svg+xml']",
      "link[rel='icon'][type='image/png']",
      "link[rel='apple-touch-icon']",
    ].map((selector) => attribute(document, selector, "href"));
    for (const href of icons) {
      expect(href).toMatch(/^\//);
      expect(existsSync(join(dist, href ?? ""))).toBe(true);
    }
  });
});

describe.each(locales)("landing %s", (locale) => {
  const document = page(landingFile(locale));
  const description = attribute(document, "meta[name='description']", "content");

  it("names rFirma in the h1", () => {
    expect(document.querySelector("h1")?.textContent).toContain("rFirma");
  });

  it("does not repeat the hero body as its description", () => {
    const heroBody = document.querySelector("h1")?.parentElement?.querySelector("p")?.textContent;
    expect(heroBody?.trim()).toBeTruthy();
    expect(description).not.toBe(heroBody?.trim());
  });

  it("shares its description with Open Graph and Twitter", () => {
    expect(attribute(document, "meta[property='og:description']", "content")).toBe(description);
    expect(attribute(document, "meta[name='twitter:description']", "content")).toBe(description);
  });

  it("declares hreflang for the five languages and x-default", () => {
    const alternates = Array.from(
      document.querySelectorAll("link[rel='alternate'][hreflang]"),
      (link) => link.getAttribute("hreflang"),
    );
    expect(alternates.sort()).toEqual([...locales, "x-default"].sort());
  });

  it("describes itself as a free SoftwareApplication in JSON-LD", () => {
    const script = document.querySelector("script[type='application/ld+json']");
    const data = JSON.parse(script?.textContent ?? "");
    expect(data).toMatchObject({
      "@context": "https://schema.org",
      "@type": "SoftwareApplication",
      name: "rFirma",
      description,
      operatingSystem: expect.stringMatching(/Linux.*Windows.*macOS/),
      applicationCategory: expect.any(String),
      offers: { "@type": "Offer", price: "0" },
      downloadUrl: expect.stringMatching(/^https:\/\//),
      isBasedOn: { codeRepository: "https://github.com/sgomez/rfirma" },
    });
  });
});

describe.each(landings)("screenshots of $name", ({ file }) => {
  const document = page(file);
  const screenshots = Array.from(document.querySelectorAll("img[src*='/_astro/']"));

  it("are served as AVIF or WebP with a srcset and sizes", () => {
    expect(screenshots.length).toBeGreaterThan(0);
    for (const image of screenshots) {
      expect(image.getAttribute("src")).toMatch(/\.(avif|webp)$/);
      expect(image.getAttribute("srcset")).toMatch(/\.(avif|webp) \d+w/);
      expect(image.getAttribute("sizes")).toBeTruthy();
    }
  });
});

describe("404", () => {
  const document = page("404.html");

  it("links to the landing in every language", () => {
    const links = Array.from(document.querySelectorAll("a[href]"), (a) => a.getAttribute("href"));
    expect(links).toEqual(expect.arrayContaining(locales.map(localePath)));
  });

  it("asks search engines not to index it", () => {
    expect(attribute(document, "meta[name='robots']", "content")).toContain("noindex");
  });
});

describe("sitemap", () => {
  const sitemaps = readdirSync(dist).filter((name) => /^sitemap-\d+\.xml$/.test(name));
  const urls = sitemaps.flatMap((name) =>
    Array.from(published(name).matchAll(/<loc>([^<]+)<\/loc>/g), (match) => match[1]),
  );

  it("lists the landing in the five languages", () => {
    expect(urls).toEqual(
      expect.arrayContaining(locales.map((locale) => `${site}${localePath(locale)}`)),
    );
  });

  it("leaves the 404 out", () => {
    expect(urls.filter((url) => url.includes("404"))).toEqual([]);
  });

  it("is announced by robots.txt", () => {
    const robots = published("robots.txt");
    const announced = robots.match(/^Sitemap: (\S+)$/m)?.[1] ?? "";
    expect(announced).toBe(`${site}/sitemap-index.xml`);
    expect(published("sitemap-index.xml")).toContain(`${site}/${sitemaps[0]}`);
  });
});
