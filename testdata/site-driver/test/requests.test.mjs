// Las pruebas de la forma que emiten el servidor intermedio, el trifásico y la descarga del dat.

import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { THE_PAGE_REQUEST_HEADER } from "../lib/browser.mjs";
import { aBatchServlet } from "../scripts/batch.mjs";
import { anIntermediateServer } from "../scripts/relay.mjs";
import { aDocumentServed } from "../scripts/request.mjs";
import { theTriphaseServer } from "../scripts/signature.mjs";

describe("the request the intermediate server tells", () => {
  it("names the storage service for the StorageService path", async () => {
    const told = [];
    const server = await anIntermediateServer({ telling: (event) => told.push(event) });

    try {
      await fetch(`${server.storage}?op=put&id=uno&dat=SG9sYQ`, { method: "POST" });
    } finally {
      await server.close();
    }

    assert.equal(told.length, 1);
    assert.equal(told[0].service, "intermediate_storage");
    assert.equal(told[0].path, "/afirma-signature-storage/StorageService");
  });

  it("names the retrieval service for the RetrieveService path", async () => {
    const told = [];
    const server = await anIntermediateServer({ telling: (event) => told.push(event) });

    try {
      await fetch(`${server.retrieve}?op=get&id=uno`, { method: "POST" });
    } finally {
      await server.close();
    }

    assert.equal(told.length, 1);
    assert.equal(told[0].service, "intermediate_retrieval");
    assert.equal(told[0].path, "/afirma-signature-retriever/RetrieveService");
  });

  it("does not name a request the page marked as its own", async () => {
    const told = [];
    const server = await anIntermediateServer({ telling: (event) => told.push(event) });

    try {
      await fetch(`${server.storage}?op=put&id=uno&dat=SG9sYQ`, {
        method: "POST",
        headers: { [THE_PAGE_REQUEST_HEADER]: "1" },
      });
    } finally {
      await server.close();
    }

    assert.equal(told.length, 0);
  });
});

describe("the request the triphase server tells", () => {
  it("names the triphase service", async () => {
    const told = [];
    const server = await aBatchServlet(theTriphaseServer({ signature: () => Buffer.from("x") }), {
      service: "triphase",
      telling: (event) => told.push(event),
    });

    try {
      const { port } = server.address();
      await fetch(`http://127.0.0.2:${port}/batch?op=pre&cop=sign&format=CAdEStri&doc=SG9sYQ&cert=MII`);
    } finally {
      await new Promise((resolve) => server.close(resolve));
    }

    assert.equal(told.length, 1);
    assert.equal(told[0].service, "triphase");
    assert.deepEqual(told[0].query, ["cert", "cop", "doc", "format", "op"]);
  });
});

describe("the request the dat download tells", () => {
  it("names the dat download service", async () => {
    const told = [];
    const served = await aDocumentServed(Buffer.from("documento"), {
      telling: (event) => told.push(event),
    });

    try {
      await fetch(served.url);
    } finally {
      await served.close();
    }

    assert.equal(told.length, 1);
    assert.equal(told[0].service, "dat_download");
    assert.equal(told[0].method, "GET");
    assert.equal(told[0].path, "/documento.bin");
  });
});
