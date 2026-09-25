// Las pruebas de lo que juzgan los guiones de lote de la sede con lo que devuelve el cliente.

import assert from "node:assert/strict";
import { beforeEach, describe, it } from "node:test";

import { declaringTheConditions } from "../lib/events.mjs";
import { theLocalBatchWithoutADialogueConditions } from "../scripts/batch.mjs";

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
