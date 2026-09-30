// Carga los matchers de jest-dom (`toBeInTheDocument`, …) en cada fichero de
// prueba. Lo referencia `setupFiles` de vite.config.ts.
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Testing-library solo se autolimpia cuando `globals` está activo, y aquí no
// lo está (vite.config.ts). Sin esto, el segundo `render` de un fichero
// encuentra dos veces cada texto y falla con «found multiple elements».
afterEach(cleanup);

vi.mock("@tauri-apps/api/app", () => ({ getVersion: () => Promise.resolve("0.1.0") }));
