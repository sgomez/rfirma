import { fileURLToPath } from "node:url";

import { defineConfig, passthroughImageService } from "astro/config";

/** El sistema de diseño vive fuera de la raíz del sitio y Vite no lo sirve sin permiso. */
const designSystem = fileURLToPath(new URL("../../../rfirma-app/src/design-system", import.meta.url));

// El sitio es estático puro: lo sirve Caddy desde la imagen (ADR-0015), así que
// no hay servidor de imágenes que optimice nada en caliente.
export default defineConfig({
  site: "https://rfirma.sgomez.me",
  trailingSlash: "always",
  build: { format: "directory" },
  image: { service: passthroughImageService() },
  vite: { server: { fs: { allow: [".", designSystem] } } },
  i18n: {
    defaultLocale: "es",
    locales: ["es", "ca", "eu", "gl", "en"],
    routing: { prefixDefaultLocale: false },
  },
});
