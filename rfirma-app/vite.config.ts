import react from "@vitejs/plugin-react";
// De `vitest/config`, no de `vite`: es el `defineConfig` que conoce la clave
// `test`. Con el de `vite` esto falla en `tsc -b` con TS2769.
import { defineConfig } from "vitest/config";

// El puerto es fijo y `strictPort` está puesto porque `tauri dev` apunta a
// devUrl en tauri.conf.json: si vite se moviera de puerto al encontrarlo
// ocupado, la ventana abriría en blanco sin decir por qué.
export default defineConfig({
  plugins: [react()],
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
