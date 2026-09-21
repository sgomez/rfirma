import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { App } from "./App";
import { browserWire, suiteOver } from "./suite/suite";
import { applyTheme } from "./ui/theme";
import "./styles.css";

applyTheme();

const token = new URLSearchParams(window.location.search).get("token") ?? "";
const root = document.getElementById("root");
if (!root) throw new Error("falta #root en la página");

createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <App suite={suiteOver(browserWire, token)} />
    </BrowserRouter>
  </StrictMode>,
);
