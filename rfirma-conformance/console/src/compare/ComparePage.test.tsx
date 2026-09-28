import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ReceivedRequest } from "../contract/ReceivedRequest";
import type { RequestDifference } from "../contract/RequestDifference";
import type { RequestsComparison } from "../contract/RequestsComparison";
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
        requests_differing: 0,
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

  it("shows a check with the same result whose requests differ, with the requests of each side and what differs", async () => {
    renderConsoleAt("/comparar?a=uno&b=dos", aSnapshot(), (fake) => {
      fake.comparison = {
        a: { client: "/usr/bin/autofirma", kind: "autofirma", client_version: "1.9.2" },
        b: { client: "/usr/bin/rfirma", kind: "rfirma", client_version: "0.10.0" },
        rows: [
          {
            ...row("a_remote_batch", "lote", "CONFORME", "CONFORME", false),
            requests: "differ",
            a_requests: [aPresignerRequest({ query: [], body: ["certs", "json"] })],
            b_requests: [aPresignerRequest({ query: ["certs", "json"], body: [] })],
            request_differences: [
              {
                service: "presigner",
                differences: [
                  "parámetros: en el cuerpo: certs, json frente a en la query: certs, json",
                ],
              },
            ],
          },
        ],
        differing: 0,
        requests_differing: 1,
      };
    });

    const table = await screen.findByRole("table");

    expect(within(table).getByText("a_remote_batch")).toBeInTheDocument();
    expect(within(table).getByText("envío: difieren")).toBeInTheDocument();
    expect(
      within(table).getByText(
        "parámetros: en el cuerpo: certs, json frente a en la query: certs, json",
      ),
    ).toBeInTheDocument();
    expect(within(table).getByRole("list", { name: "Peticiones de A" })).toHaveTextContent(
      "prefirmador POST /batch · query: — · cuerpo: certs, json",
    );
    expect(within(table).getByRole("list", { name: "Peticiones de B" })).toHaveTextContent(
      "query: certs, json · cuerpo: —",
    );
  });

  it("filters the table to only the checks whose requests differ", async () => {
    renderConsoleAt("/comparar?a=uno&b=dos", aSnapshot(), (fake) => {
      fake.comparison = {
        a: { client: "/usr/bin/autofirma", kind: "autofirma", client_version: "1.9.2" },
        b: { client: "/usr/bin/rfirma", kind: "rfirma", client_version: "0.10.0" },
        rows: [
          row("empty_uri_rejected", "errores", "NO CONFORME", "CONFORME", true),
          {
            ...row("a_remote_batch", "lote", "CONFORME", "CONFORME", false),
            requests: "differ",
            a_requests: [aPresignerRequest({ query: [], body: ["certs"] })],
            b_requests: [aPresignerRequest({ query: ["certs"], body: [] })],
            request_differences: [
              {
                service: "presigner",
                differences: ["parámetros: en el cuerpo: certs frente a en la query: certs"],
              },
            ],
          },
        ],
        differing: 1,
        requests_differing: 1,
      };
    });

    const table = await screen.findByRole("table");
    expect(within(table).getByText("empty_uri_rejected")).toBeInTheDocument();
    expect(within(table).getByText("a_remote_batch")).toBeInTheDocument();

    screen.getByLabelText("Solo las comprobaciones cuyo envío difiere").click();

    expect(within(table).queryByText("empty_uri_rejected")).not.toBeInTheDocument();
    expect(within(table).getByText("a_remote_batch")).toBeInTheDocument();
  });
});

function aPresignerRequest({ query, body }: { query: string[]; body: string[] }): ReceivedRequest {
  return {
    service: "presigner",
    method: "POST",
    path: "/batch",
    query,
    body,
    content_type: "application/x-www-form-urlencoded",
    headers: { origin: null, authorization: null, accept: null },
  };
}

function row(
  id: string,
  set: string,
  a: "CONFORME" | "NO CONFORME",
  b: "CONFORME" | "NO CONFORME",
  differ: boolean,
) {
  return {
    id,
    set,
    chapter: "05",
    citation: "A.java:1",
    a,
    b,
    differ,
    requests: "match" as RequestsComparison,
    a_requests: [],
    b_requests: [],
    request_differences: [] as RequestDifference[],
  };
}
