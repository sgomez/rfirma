import { defineConfig, passthroughImageService } from "astro/config";

// El sitio es estático puro: lo sirve Caddy desde la imagen (ADR-0015), así que
// no hay servidor de imágenes que optimice nada en caliente.
export default defineConfig({
  site: "https://rfirma.sgomez.me",
  trailingSlash: "always",
  build: { format: "directory" },
  image: { service: passthroughImageService() },
  i18n: {
    defaultLocale: "es",
    locales: ["es", "ca", "eu", "gl", "en"],
    routing: { prefixDefaultLocale: false },
  },
});
