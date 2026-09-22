import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { aSnapshot } from "../test/fixtures";
import { renderConsoleAt } from "../test/render";
import { THEME_KEY } from "../ui/theme";

describe("the shell", () => {
  it("switches the theme and remembers the choice", async () => {
    const { user } = renderConsoleAt("/");
    const initial = document.documentElement.dataset.theme;
    expect(initial).toBe("light");

    await user.click(screen.getByRole("button", { name: "Cambiar a tema oscuro" }));

    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(window.localStorage.getItem(THEME_KEY)).toBe("dark");

    await user.click(screen.getByRole("button", { name: "Cambiar a tema claro" }));
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(window.localStorage.getItem(THEME_KEY)).toBe("light");
  });

  it("starts from the remembered theme", () => {
    window.localStorage.setItem(THEME_KEY, "dark");

    renderConsoleAt("/");

    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("carries the token in every link to another view", async () => {
    renderConsoleAt("/", aSnapshot());

    expect(await screen.findByRole("link", { name: "Comparar" })).toHaveAttribute(
      "href",
      "/comparar?token=t0k",
    );
    expect(screen.getByRole("link", { name: /Abrir en otra ventana/ })).toHaveAttribute(
      "href",
      "/informe/af-linux-prueba?token=t0k",
    );
  });

  it("opens the shortcut help with ?", async () => {
    const { user } = renderConsoleAt("/");

    await user.keyboard("?");

    expect(await screen.findByRole("dialog", { name: "Atajos de teclado" })).toBeInTheDocument();
  });
});
