import { useEffect, useState } from "react";
import { MoonIcon, SunIcon } from "./icons";

export type Theme = "light" | "dark";

export const THEME_KEY = "rfirma-conformance-theme";

function chosen(): Theme | null {
  const stored = window.localStorage.getItem(THEME_KEY);
  return stored === "light" || stored === "dark" ? stored : null;
}

function preferred(): Theme {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(theme: Theme = chosen() ?? preferred()): Theme {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
  return theme;
}

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(() => applyTheme());

  useEffect(() => {
    const query = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!query) return;
    const follow = () => {
      if (!chosen()) setTheme(applyTheme());
    };
    query.addEventListener("change", follow);
    return () => query.removeEventListener("change", follow);
  }, []);

  const other: Theme = theme === "dark" ? "light" : "dark";
  const toggle = () => {
    window.localStorage.setItem(THEME_KEY, other);
    setTheme(applyTheme(other));
  };

  return (
    <button
      type="button"
      className="icon-button"
      onClick={toggle}
      aria-label={other === "dark" ? "Cambiar a tema oscuro" : "Cambiar a tema claro"}
      title={other === "dark" ? "Tema oscuro" : "Tema claro"}
    >
      {theme === "dark" ? <SunIcon /> : <MoonIcon />}
    </button>
  );
}
