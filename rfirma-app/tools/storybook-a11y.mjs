/**
 * `storybook-a11y`: axe en navegador sobre todas las historias, en claro y en oscuro, con contraste.
 *
 * Construye Storybook, lo sirve en local y abre cada historia en Chrome (o en
 * `RFIRMA_BROWSER`, la ruta de un ejecutable). Imprime una línea por historia y
 * tema, y sale con 1 si alguna tiene fallos. No es del CI: la app corre en
 * WebKitGTK y WebView2, así que comprueba reglas, no el motor real.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import { chromium } from "playwright-core";

const THEMES = ["light", "dark"];
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
};

const out = mkdtempSync(join(tmpdir(), "rfirma-storybook-"));
execFileSync("pnpm", ["exec", "storybook", "build", "--quiet", "-o", out], { stdio: "inherit" });

const server = createServer((request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, "http://x").pathname));
  try {
    const file = join(out, path.endsWith("/") ? `${path}index.html` : path);
    const body = readFileSync(file);
    response.setHeader("Content-Type", TYPES[extname(file)] ?? "application/octet-stream");
    response.end(body);
  } catch {
    response.statusCode = 404;
    response.end();
  }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const base = `http://127.0.0.1:${server.address().port}`;

const axeSource = readFileSync(
  createRequire(import.meta.url).resolve("axe-core/axe.min.js"),
  "utf8",
);
const index = await (await fetch(`${base}/index.json`)).json();
const stories = Object.values(index.entries).filter((entry) => entry.type === "story");

const executablePath = process.env.RFIRMA_BROWSER;
const browser = await chromium.launch(executablePath ? { executablePath } : { channel: "chrome" });
const page = await browser.newPage();
let failing = 0;

for (const story of stories) {
  for (const theme of THEMES) {
    await page.goto(`${base}/iframe.html?id=${story.id}&viewMode=story&globals=theme:${theme}`);
    await page.waitForSelector("#storybook-root > *", { timeout: 15_000 });
    await page.evaluate(axeSource);
    const { violations } = await page.evaluate(() => globalThis.axe.run("#storybook-root"));
    const verdict = violations.length === 0 ? "ok" : `${violations.length} fallo(s)`;
    console.log(`${story.title} · ${story.name} [${theme}]: ${verdict}`);
    for (const violation of violations) {
      failing += 1;
      console.log(`  ${violation.id} (${violation.impact}): ${violation.nodes[0]?.html}`);
    }
  }
}

await browser.close();
server.close();
rmSync(out, { recursive: true, force: true });
console.log(`\n${stories.length} historias, ${THEMES.length} temas, ${failing} fallo(s).`);
process.exit(failing === 0 ? 0 : 1);
