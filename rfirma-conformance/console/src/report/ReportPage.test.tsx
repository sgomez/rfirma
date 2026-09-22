import { act, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { aReportView, aSnapshot } from "../test/fixtures";
import { renderConsoleAt } from "../test/render";

describe("a report page", () => {
  it("shows the report without any control that runs something", async () => {
    const snapshot = aSnapshot({ report_name: "otro", report: aReportView() });
    renderConsoleAt("/informe/af-linux-prueba", snapshot, (server) =>
      server.reportViews.set("af-linux-prueba", aReportView()),
    );

    expect(await screen.findByText("empty_uri_rejected")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Ejecutar/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Pendientes/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Todo" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "1. Cliente" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Validar contra referencia…" })).toBeInTheDocument();
  });

  it("reads the report from the session while the session runs it", async () => {
    const snapshot = aSnapshot({
      running: { ids: ["empty_uri_rejected"], elapsed_ms: 0 },
    });
    const { server } = renderConsoleAt("/informe/af-linux-prueba", snapshot);

    expect(await screen.findByText("en directo")).toBeInTheDocument();
    expect(server.received.some((call) => call.path === "/api/report-view")).toBe(false);
    expect(screen.getByRole("img", { name: "en curso" })).toBeInTheDocument();
  });

  it("says so when the report does not exist", async () => {
    renderConsoleAt("/informe/nadie", aSnapshot());

    expect(await screen.findByText("no existe el informe «nadie»")).toBeInTheDocument();
  });

  it("shows the log of the chosen check filtered by provenance", async () => {
    const { user } = renderConsoleAt(
      "/informe/af-linux-prueba",
      aSnapshot({ report_name: "otro" }),
      (server) => {
        server.reportViews.set("af-linux-prueba", aReportView());
        server.logs.set(
          "greeting_echoes",
          [
            "00:00.10 sede    abre el canal",
            "00:00.20 cliente responde",
            "00:00.30 suite   CONFORME",
          ].join("\n"),
        );
      },
    );

    await user.click(await screen.findByText("greeting_echoes"));
    expect(await screen.findByText("abre el canal")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "sede" }));

    expect(screen.queryByText("abre el canal")).not.toBeInTheDocument();
    expect(screen.getByText("responde")).toBeInTheDocument();
  });

  it("holds the live lines while the log is paused and lets them through on resume", async () => {
    const { server, user } = renderConsoleAt(
      "/",
      aSnapshot({ running: { ids: ["greeting_echoes"], elapsed_ms: 0 } }),
    );
    await screen.findByText("Esperando la primera línea…");

    await user.keyboard("l");
    act(() => server.say({ check: "greeting_echoes", line: "00:00.10 sede    abre el canal" }));
    expect(screen.queryByText("abre el canal")).not.toBeInTheDocument();

    await user.click(await screen.findByRole("button", { name: /Reanudar \(1\)/ }));
    expect(await screen.findByText("abre el canal")).toBeInTheDocument();
  });
});
