import { readFileSync } from "node:fs";
import react from "@vitejs/plugin-react";
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
export default defineConfig({
  plugins: [react()],
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
