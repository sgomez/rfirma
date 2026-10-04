import { fileURLToPath } from "node:url";

import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

/** El sistema de diseño vive fuera de la raíz del sitio y Vite no lo sirve sin permiso. */
const designSystem = fileURLToPath(
  new URL("../../../rfirma-app/src/design-system", import.meta.url),
);

// El sitio es estático puro: lo sirve Caddy desde la imagen (ADR-0015), y sharp
// optimiza las imágenes al compilar.
// Sin `i18n` de Astro: Starlight no lo admite junto a sus `locales`, y la landing no lo usa.
export default defineConfig({
  site: "https://rfirma.sgomez.me",
  trailingSlash: "always",
  build: { format: "directory" },
  integrations: [
    starlight({
      title: "rFirma",
      description:
        "Manual de uso de rFirma: firma electrónica con certificado digital o DNIe, alternativa a AutoFirma.",
      locales: { root: { label: "Español", lang: "es" } },
      disable404Route: true,
      favicon: "/favicon.svg",
      head: [
        {
          tag: "link",
          attrs: { rel: "icon", type: "image/png", sizes: "32x32", href: "/favicon.png" },
        },
        { tag: "link", attrs: { rel: "apple-touch-icon", href: "/apple-touch-icon.png" } },
        { tag: "meta", attrs: { name: "theme-color", content: "#067781" } },
      ],
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/sgomez/rfirma" }],
      customCss: ["./src/styles/manual.css"],
      sidebar: [
        { label: "Presentación", slug: "manual" },
        { label: "Instalación", slug: "manual/instalacion" },
        { label: "Firmar un PDF", slug: "manual/firmar-un-pdf" },
        { label: "Ver las firmas", slug: "manual/ver-las-firmas" },
        { label: "Firmar en una sede electrónica", slug: "manual/firmar-en-una-sede" },
        { label: "Línea de órdenes", slug: "manual/linea-de-ordenes" },
        { label: "Preferencias", slug: "manual/preferencias" },
        { label: "Problemas frecuentes", slug: "manual/problemas-frecuentes" },
        { label: "Si vienes de AutoFirma", slug: "manual/si-vienes-de-autofirma" },
      ],
    }),
    sitemap(),
  ],
  vite: { server: { fs: { allow: [".", designSystem] } } },
});
