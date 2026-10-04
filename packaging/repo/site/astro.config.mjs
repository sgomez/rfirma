import { fileURLToPath } from "node:url";

import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

/** El sistema de diseño vive fuera de la raíz del sitio y Vite no lo sirve sin permiso. */
const designSystem = fileURLToPath(
  new URL("../../../rfirma-app/src/design-system", import.meta.url),
);

/** Una página del índice del manual que aún no está escrita: se ve en la barra lateral y no se publica. */
function pending(label) {
  return { label, items: [], collapsed: true, badge: { text: "Próximamente", variant: "note" } };
}

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
        pending("Instalación"),
        pending("Firmar un PDF"),
        pending("Ver las firmas"),
        pending("Firmar en una sede electrónica"),
        { label: "Línea de órdenes", slug: "manual/linea-de-ordenes" },
        pending("Preferencias"),
        pending("Problemas frecuentes"),
        pending("Si vienes de AutoFirma"),
      ],
    }),
    sitemap(),
  ],
  vite: { server: { fs: { allow: [".", designSystem] } } },
});
