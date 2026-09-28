// Las pruebas de lo que juzgan los guiones de lote de la sede y de la petición que cuentan sus servlets.

import assert from "node:assert/strict";
import { beforeEach, describe, it } from "node:test";

import { declaringTheConditions } from "../lib/events.mjs";
import {
  aBatchServlet,
  aPostsignerSlowerThan,
  BATCH_SCRIPTS,
  theLocalBatchWithoutADialogueConditions,
  theRemoteBatchConditions,
} from "../scripts/batch.mjs";

const EVERY_DOCUMENT_SIGNED = "every-document-signed-without-a-dialogue";

function aBatchWithThePdf(pdf) {
  return { signs: [{ id: "pdf", ...pdf }] };
}

function theVerdictOf(result) {
  const [condition] = theLocalBatchWithoutADialogueConditions(result);
  assert.equal(condition.name, EVERY_DOCUMENT_SIGNED);
  return condition.verdict;
}

describe("the local batch asking for a visible signature", () => {
  beforeEach(() => declaringTheConditions([EVERY_DOCUMENT_SIGNED]));

  it("holds when the pdf came back signed", () => {
    const signature = Buffer.from("%PDF-1.7\n").toString("base64");
    const signed = aBatchWithThePdf({ result: "DONE_AND_SAVED", signature });
    assert.equal(theVerdictOf(signed), "compliant");
  });

  it("does not hold when the pdf failed without a dialogue", () => {
    assert.equal(theVerdictOf(aBatchWithThePdf({ result: "ERROR_PRE" })), "discrepant");
  });

  it("does not hold when the batch came back without the pdf", () => {
    assert.equal(theVerdictOf({ signs: [] }), "discrepant");
  });
});

describe("the request the batch presigner tells", () => {
  async function theRequestTold({ query = "", body = null, headers = {} }) {
    const told = [];
    const server = await aBatchServlet(() => ({ status: 200, body: "{}" }), {
      service: "presigner",
      telling: (event) => told.push(event),
    });
    try {
      await fetch(`http://127.0.0.2:${server.address().port}/batch${query}`, {
        method: "POST",
        headers,
        body,
      });
    } finally {
      server.closeAllConnections();
      server.close();
    }
    assert.equal(told.length, 1);
    return told[0];
  }

  const A_FORM = { "content-type": "application/x-www-form-urlencoded; charset=UTF-8" };

  it("names the parameters in the query without their values", async () => {
    const request = await theRequestTold({ query: "?op=pre&json=eyJ9&certs=MII&json=x" });

    assert.deepEqual(request, {
      event: "request",
      service: "presigner",
      method: "POST",
      path: "/batch",
      query: ["certs", "json", "op"],
      body: [],
      content_type: null,
      headers: { origin: null, authorization: null, accept: "*/*" },
    });
  });

  it("names the parameters in the body and drops the charset of its content type", async () => {
    const request = await theRequestTold({ body: "json=eyJ9&certs=MII", headers: A_FORM });

    assert.equal(request.path, "/batch");
    assert.deepEqual(request.query, []);
    assert.deepEqual(request.body, ["certs", "json"]);
    assert.equal(request.content_type, "application/x-www-form-urlencoded");
  });

  it("names the parameters in both places, each on its side", async () => {
    const request = await theRequestTold({
      query: "?op=pre",
      body: "json=eyJ9&certs=MII",
      headers: A_FORM,
    });

    assert.deepEqual(request.query, ["op"]);
    assert.deepEqual(request.body, ["certs", "json"]);
  });

  it("keeps the origin as its scheme and host and the authorization as its scheme", async () => {
    const request = await theRequestTold({
      body: "json=eyJ9",
      headers: {
        ...A_FORM,
        origin: "https://sede.example:8443",
        authorization: "Bearer un-secreto",
        accept: "Application/JSON",
      },
    });

    assert.deepEqual(request.headers, {
      origin: "https://sede.example",
      authorization: "bearer",
      accept: "application/json",
    });
  });
});

describe("the postsigner slower than the connect limit", () => {
  it("answers its servlet only after the delay it was given", async () => {
    const server = await aBatchServlet(aPostsignerSlowerThan(150), { service: "postsigner" });
    const started = Date.now();
    try {
      const answered = await fetch(`http://127.0.0.2:${server.address().port}/batch`, {
        method: "POST",
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: "",
      });
      assert.ok(Date.now() - started >= 150, "no debe contestar antes de su retraso");
      assert.equal(answered.status, 400, "sin parámetros el postsigner sigue rechazando");
    } finally {
      server.closeAllConnections();
      server.close();
    }
  });
});

describe("the batch through a servlet with a self-signed certificate", () => {
  it("declares every condition its remote batch measure emits", () => {
    declaringTheConditions(BATCH_SCRIPTS.batchuntrustedcertificate.conditions);
    assert.doesNotThrow(() => theRemoteBatchConditions({}, ""));
  });
});
