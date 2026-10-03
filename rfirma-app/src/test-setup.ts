//! La preparación que vitest carga antes de cada fichero de prueba: los matchers de jest-dom y la limpieza tras cada prueba.

// Carga los matchers de jest-dom (`toBeInTheDocument`, …) en cada fichero de
// prueba. Lo referencia `setupFiles` de vite.config.ts.
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Testing-library solo se autolimpia cuando `globals` está activo, y aquí no
// lo está (vite.config.ts). Sin esto, el segundo `render` de un fichero
// encuentra dos veces cada texto y falla con «found multiple elements».
afterEach(cleanup);
