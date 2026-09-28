// Pruebas de que los tres servidores falsos de servlet leen los parámetros solo del cuerpo.

import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { anIntermediateServer } from "../scripts/relay.mjs";
import { servletServing } from "../scripts/batch.mjs";
import { aTriphaseServerUrl } from "../scripts/signature.mjs";

const AS_A_FORM = { "content-type": "application/x-www-form-urlencoded" };

/** Una petición con `parameters` en el cuerpo, como un formulario. */
function withTheParametersInTheBody(url, parameters) {
  return fetch(url, { method: "POST", headers: AS_A_FORM, body: parameters });
}

/** Una petición con `parameters` en la query, sin cuerpo. */
function withTheParametersInTheQuery(url, parameters) {
  const withQuery = new URL(url);
  withQuery.search = parameters;
  return fetch(withQuery, { method: "POST" });
}

describe("el servlet del lote", () => {
  const answering = (parameters) => ({
    status: parameters.get("json") ? 200 : 400,
    body: "",
  });

  it("acepta el lote que llega como formulario en el cuerpo", async () => {
    const url = await servletServing(answering);
    const response = await withTheParametersInTheBody(url, "json=uno");
    assert.equal(response.status, 200);
  });

  it("rechaza el lote que solo llega en la query", async () => {
    const url = await servletServing(answering);
    const response = await withTheParametersInTheQuery(url, "json=uno");
    assert.equal(response.status, 400);
  });
});

describe("el servidor intermedio", () => {
  it("acepta el op que llega como formulario en el cuerpo", async () => {
    const server = await anIntermediateServer();
    const response = await withTheParametersInTheBody(server.storage, "op=put&id=uno&dat=hola");
    assert.equal(response.status, 200);
  });

  it("rechaza el op que solo llega en la query", async () => {
    const server = await anIntermediateServer();
    const response = await withTheParametersInTheQuery(server.storage, "op=put&id=uno&dat=hola");
    assert.equal(response.status, 400);
  });

  it("contesta al op=check que la página manda por GET en la query", async () => {
    const server = await anIntermediateServer();
    for (const servlet of [server.storage, server.retrieve]) {
      const withCheck = new URL(servlet);
      withCheck.search = "op=check";
      const response = await fetch(withCheck);
      assert.equal(response.status, 200);
      assert.equal(await response.text(), "OK\n");
    }
  });
});

describe("el servidor trifásico", () => {
  it("acepta la operación que llega como formulario en el cuerpo", async () => {
    const url = await aTriphaseServerUrl();
    const response = await withTheParametersInTheBody(url, "op=pre&format=CAdES");
    assert.equal(response.status, 200);
  });

  it("rechaza la operación que solo llega en la query", async () => {
    const url = await aTriphaseServerUrl();
    const response = await withTheParametersInTheQuery(url, "op=pre&format=CAdES");
    assert.equal(response.status, 400);
  });
});
