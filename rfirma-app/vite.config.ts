import { readdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import react from "@vitejs/plugin-react";
import type { Plugin } from "vite";
// De `vitest/config`, no de `vite`: es el `defineConfig` que conoce la clave
// `test`. Con el de `vite` esto falla en `tsc -b` con TS2769.
import { defineConfig } from "vitest/config";

// La versión que enseña «Acerca de» sale de aquí y no de una constante escrita
// a mano: package.json es la única versión del frontend, y una copia acabaría
// divergiendo el día que se publique.
const { version } = JSON.parse(readFileSync("./package.json", "utf8")) as { version: string };

// El puerto es fijo y `strictPort` está puesto porque `tauri dev` apunta a
// devUrl en tauri.conf.json: si vite se moviera de puerto al encontrarlo
// ocupado, la ventana abriría en blanco sin decir por qué.
/**
 * Dónde busca `pdf.js` las catorce fuentes estándar. Es la misma ruta en la
 * ventana y en `vite dev`, y la que se le pasa como `standardFontDataUrl` en
 * `viewer/pdfjsLoader.ts`.
 */
const STANDARD_FONTS = "/standard_fonts/";

/**
 * Empaqueta las `standard_fonts` de `pdfjs-dist` (ID-112).
 *
 * La apariencia del sello usa Courier, una de las catorce. Sin ellas `pdf.js`
 * sustituye por una fuente del sistema —pinta igual, está medido, pero con
 * otras métricas—, así que el corte de línea de la vista previa dejaría de ser
 * el del compositor. Se copian en vez de aceptar por escrito que es aproximado,
 * que era la otra mitad del ID-112.
 *
 * No hay complemento de terceros porque no hace falta: en la compilación son
 * catorce `emitFile`, y en desarrollo un intermediario de diez líneas que las
 * sirve desde `node_modules`. La ruta del paquete se resuelve con `require`
 * para que los enlaces de `pnpm` no importen.
 */
function standardFonts(): Plugin {
  const directory = join(
    dirname(createRequire(import.meta.url).resolve("pdfjs-dist/package.json")),
    "standard_fonts",
  );
  const fonts = () => readdirSync(directory).filter((name) => /^[\w.-]+$/.test(name));

  return {
    name: "rfirma:pdfjs-standard-fonts",
    generateBundle() {
      for (const name of fonts()) {
        this.emitFile({
          type: "asset",
          fileName: `standard_fonts/${name}`,
          source: readFileSync(join(directory, name)),
        });
      }
    },
    configureServer(server) {
      const served = new Set(fonts());
      server.middlewares.use((request, response, next) => {
        const path = (request.url ?? "").split("?")[0] ?? "";
        const name = path.startsWith(STANDARD_FONTS) ? path.slice(STANDARD_FONTS.length) : "";
        if (!served.has(name)) return next();
        response.setHeader("Content-Type", "application/octet-stream");
        response.end(readFileSync(join(directory, name)));
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), standardFonts()],
  define: { __APP_VERSION__: JSON.stringify(version) },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
  },
  test: {
    environment: "jsdom",
    globals: false,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
