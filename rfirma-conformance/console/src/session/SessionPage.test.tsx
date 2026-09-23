import { act, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { aReportView, aSnapshot } from "../test/fixtures";
import { renderConsoleAt } from "../test/render";

async function theSet(name: string) {
  const heading = await screen.findByRole("heading", { name });
  return heading.closest("section") as HTMLElement;
}

describe("the session", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("runs the pending checks of the set whose button was pressed and nothing else", async () => {
    const { server, user } = renderConsoleAt("/");

    await user.click(
      within(await theSet("errores")).getByRole("button", {
        name: "Ejecutar las pendientes de errores",
      }),
    );
    await user.click(
      within(await theSet("saludo")).getByRole("button", { name: "Ejecutar todo saludo" }),
    );

    expect(server.posted("/api/run")).toEqual([
      { set: "errores", pending: true },
      { set: "saludo" },
    ]);
  });

  it("runs one check from its own button", async () => {
    const { server, user } = renderConsoleAt("/");

    await user.click(await screen.findByRole("button", { name: "Ejecutar empty_uri_rejected" }));

    expect(server.posted("/api/run")).toEqual([{ check: "empty_uri_rejected" }]);
  });

  it("runs each tranche of the pending checks from its own button, and all of them with p", async () => {
    const report = aReportView();
    const attended = report.sets[1]?.checks[2];
    if (attended) attended.assistance = "click";
    const { server, user } = renderConsoleAt("/", aSnapshot({ report }));

    await user.click(await screen.findByRole("button", { name: /Solo las automáticas \(1\)/ }));
    await user.click(screen.getByRole("button", { name: /Las que te necesitan \(1\)/ }));
    await user.click(screen.getByRole("button", { name: /Ejecutar todas \(2\)/ }));
    await user.keyboard("p");

    expect(server.posted("/api/run")).toEqual([
      { tranches: "unattended" },
      { tranches: "attended" },
      { tranches: "all" },
      { tranches: "all" },
    ]);
  });

  it("stops between tranches until the person is there, and says so on the desktop", async () => {
    const shown: string[] = [];
    vi.stubGlobal(
      "Notification",
      class {
        static permission = "granted";
        constructor(_title: string, options?: NotificationOptions) {
          shown.push(options?.body ?? "");
        }
      },
    );
    const prompt = "Las siguientes abren diálogos del cliente de firma.";
    const { server, user } = renderConsoleAt(
      "/",
      aSnapshot({
        running: { ids: ["empty_uri_rejected"], elapsed_ms: 0 },
        question: { check: "empty_uri_rejected", prompt, kind: "tranche" },
      }),
    );

    await user.click(await screen.findByRole("button", { name: /Estoy/ }));

    expect(screen.getByText("Te necesitamos delante")).toBeInTheDocument();
    expect(shown).toEqual([prompt]);
    expect(server.posted("/api/answer")).toEqual([{ answer: "s" }]);
  });

  it("warns of a failure of the suite naming its check", async () => {
    const { server } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    act(() => server.fail({ check: "empty_uri_rejected", why: "agotó su espera" }));

    expect(
      await screen.findByText("Fallo de la suite en empty_uri_rejected: agotó su espera"),
    ).toBeInTheDocument();
  });

  it("folds and unfolds every set at once, by button and by key", async () => {
    const { user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    await user.click(screen.getByRole("button", { name: /Plegar todo/ }));
    expect(screen.queryByText("greeting_echoes")).not.toBeInTheDocument();
    expect(screen.queryByText("empty_uri_rejected")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Desplegar todo/ }));
    expect(screen.getByText("greeting_echoes")).toBeInTheDocument();

    await user.keyboard("[[");
    expect(screen.queryByText("empty_uri_rejected")).not.toBeInTheDocument();
    await user.keyboard("]");
    expect(screen.getByText("empty_uri_rejected")).toBeInTheDocument();
  });

  it("folds only the set whose own toggle was pressed", async () => {
    const { user } = renderConsoleAt("/");

    await user.click(await screen.findByRole("button", { name: "errores" }));

    expect(screen.queryByText("empty_uri_rejected")).not.toBeInTheDocument();
    expect(screen.getByText("greeting_echoes")).toBeInTheDocument();
  });

  it("hides the checks whose result is filtered out", async () => {
    const { user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    await user.click(screen.getByRole("button", { name: /^PENDIENTE\s*\d/ }));

    expect(screen.queryByText("greeting_echoes")).not.toBeInTheDocument();
    expect(screen.getByText("greeting_opens_the_channel")).toBeInTheDocument();
  });

  it("shows the batch bar only while something runs, and skips and stops from it", async () => {
    const { server, user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");
    expect(screen.queryByRole("region", { name: "En curso" })).not.toBeInTheDocument();

    act(() =>
      server.publish(
        aSnapshot({
          running: { ids: ["empty_uri_rejected"], elapsed_ms: 1200 },
          queued: ["greeting_echoes"],
        }),
      ),
    );
    const bar = await screen.findByRole("region", { name: "En curso" });
    expect(within(bar).getByText("errores")).toBeInTheDocument();
    expect(within(bar).getByText("0/2")).toBeInTheDocument();

    await user.click(within(bar).getByRole("button", { name: /Saltar esta/ }));
    await user.keyboard("X");

    expect(server.posted("/api/skip")).toEqual([{}]);
    expect(server.posted("/api/stop")).toEqual([{}]);

    act(() => server.publish(aSnapshot()));
    await waitFor(() =>
      expect(screen.queryByRole("region", { name: "En curso" })).not.toBeInTheDocument(),
    );
  });

  it("answers the question in flight with s, n or Escape", async () => {
    const { server, user } = renderConsoleAt(
      "/",
      aSnapshot({
        running: { ids: ["empty_uri_rejected"], elapsed_ms: 0 },
        question: {
          check: "empty_uri_rejected",
          prompt: "¿Se abrió el diálogo? [s/n]",
          kind: "outcome",
        },
      }),
    );
    expect(await screen.findByText("¿Se abrió el diálogo?")).toBeInTheDocument();

    await user.keyboard("n");

    expect(server.posted("/api/answer")).toEqual([{ answer: "n" }]);
  });

  it("keeps the report and run steps disabled until the client is resolved", async () => {
    renderConsoleAt("/", aSnapshot({ client: null, report_name: null, report: null }));

    const report = await screen.findByRole("region", { name: "2. Informe" });
    expect(report).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("region", { name: "3. Ejecutar" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("marks the client step while its changes are not resolved", async () => {
    const { user } = renderConsoleAt("/");

    await user.click(
      within(await screen.findByRole("region", { name: "1. Cliente" })).getByRole("button", {
        name: "Cambiar",
      }),
    );
    await user.click(screen.getByRole("radio", { name: "rFirma" }));

    expect(screen.getByText("cambios sin aplicar")).toBeInTheDocument();
  });

  it("empties the binary and the trust root of another client, and gives them back to its own", async () => {
    const { user } = renderConsoleAt("/");

    await user.click(
      within(await screen.findByRole("region", { name: "1. Cliente" })).getByRole("button", {
        name: "Cambiar",
      }),
    );
    const binary = screen.getByRole("textbox", { name: "Binario" });
    const root = screen.getByRole("textbox", { name: "Raíz de confianza" });
    const resolved = [(binary as HTMLInputElement).value, (root as HTMLInputElement).value];
    await user.click(screen.getByRole("radio", { name: "rFirma" }));

    expect(binary).toHaveValue("");
    expect(root).toHaveValue("");

    await user.click(screen.getByRole("radio", { name: "AutoFirma" }));

    expect([binary, root].map((field) => (field as HTMLInputElement).value)).toEqual(resolved);
  });
});
