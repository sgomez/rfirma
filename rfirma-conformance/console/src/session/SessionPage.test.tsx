import { act, cleanup, screen, waitFor, within } from "@testing-library/react";
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
      { selection: "errores", mode: "reuse" },
      { selection: "saludo", mode: "relaunch" },
    ]);
  });

  it("runs one check from its own button", async () => {
    const { server, user } = renderConsoleAt("/");

    await user.click(await screen.findByRole("button", { name: "Ejecutar empty_uri_rejected" }));

    expect(server.posted("/api/run")).toEqual([{ selection: "empty_uri_rejected", mode: "reuse" }]);
  });

  it("resets a settled check from its own button, and not one that is still pending", async () => {
    const { server, user } = renderConsoleAt("/");

    await user.click(
      await screen.findByRole("button", { name: "Resetear unsupported_protocol_uri_rejected" }),
    );

    expect(server.posted("/api/reset")).toEqual([
      { selection: "unsupported_protocol_uri_rejected" },
    ]);
    expect(screen.getByRole("button", { name: "Resetear empty_uri_rejected" })).toBeDisabled();
  });

  it("runs each tranche from its own button with the count of the session, and all of them with p", async () => {
    const { server, user } = renderConsoleAt(
      "/",
      aSnapshot({ pending_by_tranches: { unattended: 4, attended: 3, all: 7 } }),
    );

    await user.click(await screen.findByRole("button", { name: /Solo las automáticas \(4\)/ }));
    await user.click(screen.getByRole("button", { name: /Las que te necesitan \(3\)/ }));
    await user.click(screen.getByRole("button", { name: /Ejecutar todas \(7\)/ }));
    await user.keyboard("p");

    expect(server.posted("/api/run")).toEqual([
      { selection: "", tranches: "unattended" },
      { selection: "", tranches: "attended" },
      { selection: "", tranches: "all" },
      { selection: "", tranches: "all" },
    ]);
  });

  it("starts a check that needs the person without asking, and says on the desktop what to do", async () => {
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
    const report = aReportView();
    const running = report.sets[1]?.checks[2];
    if (running) {
      running.warning = "Fíjate en si el cliente pide confirmación.";
      running.assistance = "person";
    }
    renderConsoleAt(
      "/",
      aSnapshot({
        report,
        running: { ids: ["empty_uri_rejected"], elapsed_ms: 0 },
        batch: { total: 1, done: 0, running: 1, elapsed_ms: 0 },
      }),
    );

    expect(await screen.findByText("Lo que te toca")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Empezar|Estoy/ })).not.toBeInTheDocument();
    expect(shown).toEqual(["Fíjate en si el cliente pide confirmación."]);
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

  it("shows the batch the session counts only while it runs, and skips and stops from it", async () => {
    const { server, user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");
    expect(screen.queryByRole("region", { name: "En curso" })).not.toBeInTheDocument();

    act(() =>
      server.publish(
        aSnapshot({
          running: { ids: ["empty_uri_rejected"], elapsed_ms: 1200 },
          queued: ["greeting_echoes"],
          batch: { total: 5, done: 3, running: 1, elapsed_ms: 9000 },
        }),
      ),
    );
    const bar = await screen.findByRole("region", { name: "En curso" });
    expect(within(bar).getByText("errores")).toBeInTheDocument();
    expect(within(bar).getByText("3/5")).toBeInTheDocument();

    await user.click(within(bar).getByRole("button", { name: /Saltar esta/ }));
    await user.keyboard("X");

    expect(server.posted("/api/skip")).toEqual([{}]);
    expect(server.posted("/api/stop")).toEqual([{}]);

    act(() => server.publish(aSnapshot()));
    await waitFor(() =>
      expect(screen.queryByRole("region", { name: "En curso" })).not.toBeInTheDocument(),
    );
  });

  it("keeps the instructions of the running check in view once it has started", async () => {
    const report = aReportView();
    const running = report.sets[1]?.checks[2];
    if (running) {
      running.warning = "Cancela el diálogo de guardar.";
      running.assistance = "click";
    }
    renderConsoleAt(
      "/",
      aSnapshot({
        report,
        running: { ids: ["empty_uri_rejected"], elapsed_ms: 0 },
        batch: { total: 1, done: 0, running: 1, elapsed_ms: 0 },
      }),
    );

    const bar = await screen.findByRole("region", { name: "En curso" });
    expect(within(bar).getByText("Cancela el diálogo de guardar.")).toBeInTheDocument();
  });

  it("offers the reports to open without a client and keeps the run step disabled", async () => {
    renderConsoleAt("/", aSnapshot({ client: null, report_name: null, report: null }));

    const report = await screen.findByRole("region", { name: "2. Informe" });
    expect(report).not.toHaveAttribute("aria-disabled", "true");
    expect(within(report).getByRole("button", { name: "Abrir" })).toBeEnabled();
    expect(within(report).getByRole("button", { name: "Informe nuevo…" })).toBeDisabled();
    expect(screen.getByRole("region", { name: "3. Ejecutar" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("creates a report from the form filled with what the session proposes, with the title changed", async () => {
    const { server, user } = renderConsoleAt("/", aSnapshot({ report_name: null, report: null }));

    await user.click(await screen.findByRole("button", { name: "Informe nuevo…" }));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() =>
      expect(within(dialog).getByLabelText("Nombre")).toHaveValue("autofirma-1.9.2-2026-10-24"),
    );
    expect(within(dialog).getByLabelText("Título")).toHaveValue("autofirma 1.9.2");
    expect(within(dialog).getByLabelText("Versión del cliente")).toHaveValue("1.9.2");
    expect(within(dialog).getByLabelText("Sistema")).toHaveValue("Linux");
    await user.clear(within(dialog).getByLabelText("Título"));
    await user.type(within(dialog).getByLabelText("Título"), "Prueba de rades");
    await user.click(within(dialog).getByRole("button", { name: "Crear" }));

    expect(server.posted("/api/reports")).toEqual([
      {
        name: "autofirma-1.9.2-2026-10-24",
        title: "Prueba de rades",
        notes: "",
        client_version: "1.9.2",
        os: "Linux",
        os_version: "6.8.0",
        scope: null,
      },
    ]);
  });

  it("marks the tree from what is typed in the scope field, and rewrites the field from the tree", async () => {
    const { user } = renderConsoleAt("/", aSnapshot({ report_name: null, report: null }));

    await user.click(await screen.findByRole("button", { name: "Informe nuevo…" }));
    const dialog = await screen.findByRole("dialog");
    const field = within(dialog).getByLabelText("Alcance");
    await user.type(field, "errores");

    await waitFor(() =>
      expect(within(dialog).getByRole("checkbox", { name: "Marcar errores" })).toBeChecked(),
    );
    expect(
      within(dialog).getByRole("checkbox", { name: "Marcar empty_uri_rejected" }),
    ).toBeChecked();
    expect(within(dialog).getByRole("checkbox", { name: "Marcar saludo" })).not.toBeChecked();

    await user.click(within(dialog).getByRole("checkbox", { name: "Marcar errores" }));
    await waitFor(() => expect(field).toHaveValue(""));
    await user.click(within(dialog).getByRole("checkbox", { name: "Marcar greeting_echoes" }));
    await waitFor(() => expect(field).toHaveValue("greeting_echoes"));
    await waitFor(() =>
      expect(
        within(dialog).getByRole("checkbox", { name: "Marcar greeting_echoes" }),
      ).toBeChecked(),
    );
    await user.click(
      within(dialog).getByRole("checkbox", { name: "Marcar greeting_opens_the_channel" }),
    );
    await waitFor(() => expect(field).toHaveValue("saludo"));
  });

  it("previews what the scope resolves and refuses to create while a name does not match", async () => {
    const { server, user } = renderConsoleAt("/", aSnapshot({ report_name: null, report: null }));

    await user.click(await screen.findByRole("button", { name: "Informe nuevo…" }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText("Alcance"), "greeting_echoes");

    const preview = await within(dialog).findByText(/2 comprobaciones/);
    expect(preview).toHaveTextContent("greeting_opens_the_channel");
    expect(within(dialog).getByRole("button", { name: "Crear" })).toBeEnabled();

    await user.type(within(dialog).getByLabelText("Alcance"), " typo");
    await within(dialog).findByText(/No casan: typo/);
    expect(within(dialog).getByRole("button", { name: "Crear" })).toBeDisabled();
    expect(server.posted("/api/reports")).toEqual([]);
  });

  it("corrects the scope of the open report with the same field", async () => {
    const view = aReportView();
    view.header.scope = "saludo";
    const { server, user } = renderConsoleAt("/", aSnapshot({ report: view }));

    await user.click(await screen.findByRole("button", { name: "Editar cabecera" }));
    const dialog = await screen.findByRole("dialog");
    const field = within(dialog).getByLabelText("Alcance");
    expect(field).toHaveValue("saludo");
    await waitFor(() =>
      expect(within(dialog).getByRole("checkbox", { name: "Marcar saludo" })).toBeChecked(),
    );
    await user.click(within(dialog).getByRole("checkbox", { name: "Marcar errores" }));
    await waitFor(() => expect(field).toHaveValue("saludo errores"));
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));

    expect(server.posted("/api/report/header")).toMatchObject([{ scope: "saludo errores" }]);
  });

  it("corrects the header of the open report from the same fields", async () => {
    const view = aReportView();
    view.header.title = "Antes";
    view.header.notes = "Una nota";
    view.header.scope = "saludo";
    const { server, user } = renderConsoleAt("/", aSnapshot({ report: view }));

    await user.click(await screen.findByRole("button", { name: "Editar cabecera" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByLabelText("Título")).toHaveValue("Antes");
    expect(within(dialog).getByLabelText("Notas")).toHaveValue("Una nota");
    await user.clear(within(dialog).getByLabelText("Versión del cliente"));
    await user.type(within(dialog).getByLabelText("Versión del cliente"), "1.9.3");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));

    expect(server.posted("/api/report/header")).toEqual([
      {
        title: "Antes",
        notes: "Una nota",
        client_version: "1.9.3",
        os: "Linux",
        os_version: "6.8.0",
        scope: "saludo",
      },
    ]);
  });

  it("lists the reports that can be continued apart from those that are read-only, with the reason and the title", async () => {
    const snapshot = aSnapshot({ report_name: null, report: null });
    const [first] = snapshot.reports;
    if (!first) throw new Error("fixture without reports");
    snapshot.reports = [
      { ...first, name: "can", title: "Mi prueba" },
      {
        ...first,
        name: "cannot",
        client_version: "1.8.0",
        continuable: false,
        why_not: "es de la versión 1.8.0",
      },
    ];
    renderConsoleAt("/", snapshot);

    const continuable = await screen.findByRole("list", { name: "Se pueden continuar" });
    const readOnly = screen.getByRole("list", { name: "Solo lectura" });
    expect(within(continuable).getByText("can")).toBeInTheDocument();
    expect(within(continuable).getByText("Mi prueba")).toBeInTheDocument();
    expect(within(readOnly).getByText("cannot")).toBeInTheDocument();
    expect(within(readOnly).getByText("es de la versión 1.8.0")).toBeInTheDocument();
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

  it("offers rFirma first, lists its candidates and makes one the client without typing a path", async () => {
    const { server, user } = renderConsoleAt(
      "/",
      aSnapshot({
        client: null,
        report_name: null,
        report: null,
        candidates: [
          {
            kind: "rfirma",
            origin: "local_build",
            binary: "/work/target/release/rfirma",
            library_dir: "/work/lib",
            identity: { version: "0.9.0", channel: "dev" },
          },
          {
            kind: "autofirma",
            origin: "installed",
            binary: "/usr/bin/autofirma",
            library_dir: null,
            identity: { version: null, channel: null },
          },
        ],
      }),
    );

    expect(await screen.findByRole("radio", { name: "rFirma" })).toBeChecked();
    expect(screen.queryByRole("radio", { name: /\/usr\/bin\/autofirma/ })).toBeNull();
    await user.click(
      screen.getByRole("radio", { name: /build local.*0\.9\.0.*\/work\/target\/release\/rfirma/ }),
    );

    expect(server.posted("/api/client")).toEqual([
      { kind: "rfirma", binary: "/work/target/release/rfirma", trust_root: null },
    ]);
  });

  it("filters the view with the selection field, combined with the result chips", async () => {
    const { user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    await user.type(screen.getByRole("searchbox", { name: "Selección" }), "errores");
    await waitFor(() => expect(screen.queryByText("greeting_echoes")).not.toBeInTheDocument());
    expect(screen.getByText("empty_uri_rejected")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^PENDIENTE\s*\d/ }));
    expect(screen.queryByText("empty_uri_rejected")).not.toBeInTheDocument();
    expect(screen.getByText("unknown_operation_rejected")).toBeInTheDocument();
  });

  it("runs what is seen with e and relaunches it with E, naming the checks once a chip hides some", async () => {
    const { server, user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    await user.type(screen.getByRole("searchbox", { name: "Selección" }), "errores");
    await waitFor(() => expect(screen.queryByText("greeting_echoes")).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: /Ejecutar lo visible/ }));
    await user.click(screen.getByRole("button", { name: /^PENDIENTE\s*\d/ }));
    await user.keyboard("e");
    await user.keyboard("E");

    expect(server.posted("/api/run")).toEqual([
      { selection: "errores", mode: "reuse" },
      { selection: "unsupported_protocol_uri_rejected unknown_operation_rejected", mode: "reuse" },
      {
        selection: "unsupported_protocol_uri_rejected unknown_operation_rejected",
        mode: "relaunch",
      },
    ]);
  });

  it("takes the focus to the selection field with /", async () => {
    const { user } = renderConsoleAt("/");
    await screen.findByText("greeting_echoes");

    await user.keyboard("/");

    const field = screen.getByRole("searchbox", { name: "Selección" });
    expect(field).toHaveFocus();
    expect(field).toHaveValue("");
  });

  it("previews the reset of what is seen, with the checks outside that share their errand", async () => {
    const { server, user } = renderConsoleAt("/", aSnapshot(), (fake) => {
      fake.sharingAnErrand = ["greeting_echoes"];
    });
    await screen.findByText("greeting_echoes");

    await user.type(screen.getByRole("searchbox", { name: "Selección" }), "errores");
    await waitFor(() => expect(screen.queryByText("greeting_echoes")).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: /Reiniciar lo visible/ }));
    const dialog = await screen.findByRole("dialog", { name: "Reiniciar lo visible" });
    expect(within(dialog).getByText(/3 comprobaciones/)).toBeInTheDocument();
    expect(await within(dialog).findByText("greeting_echoes")).toBeInTheDocument();
    expect(server.posted("/api/reset")).toEqual([]);

    await user.click(within(dialog).getByRole("button", { name: "Reiniciar" }));

    expect(server.posted("/api/reset")).toEqual([{ selection: "errores" }]);
  });

  describe("the log", () => {
    const running = (id: string) =>
      aSnapshot({
        running: { ids: [id], elapsed_ms: 0 },
        batch: { total: 3, done: 0, running: 1, elapsed_ms: 0 },
      });
    const logOf = () => screen.getByLabelText("Registro").querySelector(".log-check");

    it("turns the tracking on when a batch starts even if it had been switched off", async () => {
      const { server, user } = renderConsoleAt("/");
      await screen.findByText("greeting_echoes");
      await user.click(screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }));
      expect(
        screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }),
      ).not.toBeChecked();

      act(() => server.publish(running("greeting_echoes")));

      await waitFor(() => expect(logOf()).toHaveTextContent("greeting_echoes"));
      expect(screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" })).toBeChecked();
    });

    it("suspends the tracking on looking at another check and takes it back with the next one", async () => {
      const { server, user } = renderConsoleAt("/", running("greeting_echoes"));
      await waitFor(() => expect(logOf()).toHaveTextContent("greeting_echoes"));

      await user.click(screen.getByText("unknown_operation_rejected"));
      expect(logOf()).toHaveTextContent("unknown_operation_rejected");
      expect(screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" })).toBeChecked();

      act(() => server.publish(running("empty_uri_rejected")));
      await waitFor(() => expect(logOf()).toHaveTextContent("empty_uri_rejected"));
    });

    it("remembers the view preferences across a reload, and works without storage", async () => {
      const first = renderConsoleAt("/");
      await screen.findByText("greeting_echoes");
      await first.user.click(
        screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }),
      );
      await first.user.click(screen.getByRole("button", { name: /^sede$/ }));
      await first.user.keyboard("l");
      await first.user.click(screen.getByRole("button", { name: /^PENDIENTE\s*\d/ }));
      cleanup();

      renderConsoleAt("/");
      await screen.findByRole("checkbox", { name: "Ir a la comprobación en curso" });
      expect(
        screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }),
      ).not.toBeChecked();
      expect(screen.getByRole("button", { name: /^sede$/ })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
      expect(screen.getByRole("button", { name: /Reanudar/ })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^PENDIENTE\s*\d/ })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
      cleanup();

      vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("sin almacenamiento");
      });
      vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("sin almacenamiento");
      });
      const third = renderConsoleAt("/");
      await screen.findByText("greeting_echoes");
      await third.user.click(
        screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }),
      );
      expect(
        screen.getByRole("checkbox", { name: "Ir a la comprobación en curso" }),
      ).not.toBeChecked();
      vi.restoreAllMocks();
    });
  });
});
