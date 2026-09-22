// Las pruebas de los analizadores de firma de la sede, sobre el banco de referencia y muestras propias.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import {
  signsTheData,
  theCertificatesIn,
  theCmsSignature,
  theKeyFamilyOf,
  thePublicKeyAlgorithmOf,
  theShapeOf,
} from "../lib/cms.mjs";
import { isASignedPdf } from "../lib/pades.mjs";
import { isAXadesSignature, signsTheRoleAndThePlace, theXadesEnvelope } from "../lib/xades.mjs";
import { theZipEntries } from "../lib/zip.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const aReference = (name) => readFileSync(join(here, "../../reference", name));
const aSample = (name) => readFileSync(join(here, "samples", name));
const theChallenge = aReference("challenge.bin");

describe("the CMS analyzer", () => {
  it("reads an implicit CAdES as one signer with the data inside", () => {
    const cms = theCmsSignature(aReference("cades-implicit.p7s"));
    assert.equal(theShapeOf(cms), "[]");
    assert.deepEqual(cms.content, theChallenge);
  });

  it("reads an explicit CAdES without content", () => {
    assert.equal(theCmsSignature(aReference("cades-explicit.p7s")).content, null);
  });

  it("reads a cosignature as two parallel signers", () => {
    assert.equal(theShapeOf(theCmsSignature(aReference("cades-implicit.cosign.p7s"))), "[][]");
  });

  it("reads a countersignature nested in its signer", () => {
    const tree = theCmsSignature(aReference("cades-implicit.countersign-tree.p7s"));
    assert.equal(theShapeOf(tree), "[[]]");
  });

  it("orders the shape canonically so tree and leaves are told apart", () => {
    const aSigner = (...countersigners) => ({ countersigners });
    const tree = { signers: [aSigner(aSigner(), aSigner(aSigner()))] };
    const leaves = { signers: [aSigner(aSigner(aSigner()))] };
    assert.equal(theShapeOf(tree), "[[[]][]]");
    assert.equal(theShapeOf(leaves), "[[[]]]");
  });

  it("follows the indefinite lengths of BER", () => {
    const cms = theCmsSignature(aSample("cms-ber.p7s"));
    assert.equal(theShapeOf(cms), "[]");
    assert.deepEqual(cms.content, theChallenge);
  });

  it("checks the message digest against the data it signs", () => {
    const [signer] = theCmsSignature(aReference("cades-explicit.p7s")).signers;
    assert.equal(signsTheData(signer, theChallenge), true);
    assert.equal(signsTheData(signer, Buffer.from("otros datos")), false);
  });

  it("tells a CAdES signer from a bare CMS one", () => {
    assert.equal(theCmsSignature(aReference("cades-implicit.p7s")).signers[0].cades, true);
    assert.equal(
      theCmsSignature(readFileSync(join(here, "../cms-implicit.p7s"))).signers[0].cades,
      false,
    );
  });

  it("reads the key family of the signature and of the certificate", () => {
    for (const [bytes, family] of [
      [aReference("cades-implicit.p7s"), "rsa"],
      [aSample("cms-ecdsa.p7s"), "ec"],
    ]) {
      const [signer] = theCmsSignature(bytes).signers;
      assert.equal(theKeyFamilyOf(signer.signatureAlgorithm), family);
      assert.equal(theKeyFamilyOf(thePublicKeyAlgorithmOf(theCertificatesIn(bytes)[0])), family);
    }
  });

  it("answers null for what is not a SignedData", () => {
    assert.equal(theCmsSignature(aReference("document.xml")), null);
    assert.equal(theCmsSignature(theChallenge), null);
  });
});

describe("the ZIP reader", () => {
  it("reads stored and deflated entries of an ASiC-S container", () => {
    const entries = theZipEntries(aSample("cades-asics.zip"));
    assert.deepEqual([...entries.keys()], ["mimetype", "dataobject.bin", "META-INF/signature.p7s"]);
    assert.deepEqual(entries.get("dataobject.bin"), theChallenge);
    assert.deepEqual(entries.get("META-INF/signature.p7s"), aReference("cades-explicit.p7s"));
  });

  it("answers null for what is not a ZIP", () => {
    assert.equal(theZipEntries(aReference("cades-implicit.p7s")), null);
  });
});

describe("the XML signature analyzer", () => {
  const anXml = (name) => aReference(name).toString("utf8");

  it("reads the envelope of each reference XAdES", () => {
    assert.equal(theXadesEnvelope(anXml("xades-enveloping.xml"), "documento"), "enveloping");
    assert.equal(theXadesEnvelope(anXml("xades-enveloped.xml"), "documento"), "enveloped");
    assert.equal(theXadesEnvelope(anXml("xades-detached.xml"), "documento"), "detached");
  });

  it("reads an externally detached signature by the URI of its reference", () => {
    const uri = "https://sede.example/documento.xml";
    const xml = anXml("xades-enveloping.xml").replace(/URI="#Object-[^"]+"/, `URI="${uri}"`);
    assert.equal(theXadesEnvelope(xml, "documento", uri), "externally-detached");
  });

  it("finds no envelope in a document without signature", () => {
    assert.equal(theXadesEnvelope(anXml("document.xml"), "documento"), null);
  });

  it("tells a XAdES signature from a bare document", () => {
    assert.equal(isAXadesSignature(anXml("xades-enveloping.xml")), true);
    assert.equal(isAXadesSignature(anXml("document.xml")), false);
  });

  it("finds the claimed role and the production city", () => {
    const place =
      "<xades:SignatureProductionPlace><xades:City>Madrid</xades:City></xades:SignatureProductionPlace>";
    const withThePlace = anXml("facturae.xsig").replace(
      "</xades:SignerRole>",
      `</xades:SignerRole>${place}`,
    );
    assert.equal(signsTheRoleAndThePlace(withThePlace, "emisor", "Madrid"), true);
    assert.equal(signsTheRoleAndThePlace(withThePlace, "emisor", "Sevilla"), false);
    assert.equal(signsTheRoleAndThePlace(anXml("facturae.xsig"), "emisor", "Madrid"), false);
  });
});

describe("the signed PDF analyzer", () => {
  const aPdf = (body) => Buffer.from(`%PDF-1.4\n${body}\n%%EOF\n`, "latin1");

  it("finds a signature dictionary with its byte range", () => {
    const dictionary =
      "<< /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 10 20 30] /Contents <3082> >>";
    assert.equal(isASignedPdf(aPdf(`5 0 obj\n${dictionary}\nendobj`)), true);
  });

  it("refuses a PDF without signature and what is not a PDF", () => {
    assert.equal(isASignedPdf(aPdf("1 0 obj\n<< /Type /Catalog >>\nendobj")), false);
    assert.equal(isASignedPdf(aReference("cades-implicit.p7s")), false);
  });
});
