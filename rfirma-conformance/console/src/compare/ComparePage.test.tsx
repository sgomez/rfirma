import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { aSnapshot } from "../test/fixtures";
import { renderConsoleAt } from "../test/render";

describe("the comparison", () => {
  it("compares the two reports named in the address and shows only what differs, by set", async () => {
    const { server } = renderConsoleAt("/comparar?a=uno&b=dos", aSnapshot(), (fake) => {
      fake.comparison = {
        a: { client: "/usr/bin/autofirma", kind: "autofirma", client_version: "1.9.2" },
        b: { client: "/usr/bin/rfirma", kind: "rfirma", client_version: "0.10.0" },
        rows: [
          row("greeting_echoes", "saludo", "CONFORME", "CONFORME", false),
          row("empty_uri_rejected", "errores", "NO CONFORME", "CONFORME", true),
        ],
        differing: 1,
      };
    });

    const table = await screen.findByRole("table");

    expect(server.received.find((call) => call.path === "/api/compare")?.params).toMatchObject({
      a: "uno",
      b: "dos",
    });
    expect(within(table).getByText("empty_uri_rejected")).toBeInTheDocument();
    expect(within(table).queryByText("greeting_echoes")).not.toBeInTheDocument();
    expect(within(table).getByRole("rowheader", { name: /errores/ })).toBeInTheDocument();
  });
});

function row(
  id: string,
  set: string,
  a: "CONFORME" | "NO CONFORME",
  b: "CONFORME" | "NO CONFORME",
  differ: boolean,
) {
  return { id, set, chapter: "05", citation: "A.java:1", a, b, differ };
}
