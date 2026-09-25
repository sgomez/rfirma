// Las pruebas de los analizadores de firma de la sede, sobre el banco de referencia y muestras propias.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

import {
  signsTheData,
  theCertificatesIn,
  theCmsSignature,
  theCmsVerification,
  theDigestsOf,
  theKeyFamilyOf,
  thePublicKeyAlgorithmOf,
  theShapeOf,
} from "../lib/cms.mjs";
import {
  isASignedPdf,
  isAVisibleArea,
  thePadesSigners,
  thePadesVerification,
  theSignatureRectangles,
} from "../lib/pades.mjs";
import { isABarePkcs1 } from "../lib/pkcs1.mjs";
import {
  isAXadesSignature,
  signsTheRoleAndThePlace,
  theXadesEnvelope,
  theXadesSignatureDigest,
  theXadesSigners,
  theXadesVerification,
} from "../lib/xades.mjs";
import { theZipEntries } from "../lib/zip.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const aReference = (name) => readFileSync(join(here, "../../reference", name));
const aSample = (name) => readFileSync(join(here, "samples", name));
const theChallenge = aReference("challenge.bin");
const aCertificate = (name) => readFileSync(join(here, "../certificates", name));
const theSigner = aCertificate("active-rsa.der");
const anotherCertificate = aCertificate("pseudonym-rsa.der");

function withAByteFlippedAt(bytes, at) {
  const altered = Buffer.from(bytes);
  altered[at] ^= 0x01;
  return altered;
}

const SHA256_OID = Buffer.from("0609608648016503040201", "hex");
const SHA224_OID = Buffer.from("0609608648016503040204", "hex");
const RSA_ENCRYPTION_OID = Buffer.from("06092a864886f70d010101", "hex");
const RSASSA_PSS_OID = Buffer.from("06092a864886f70d01010a", "hex");

function withTheOidReplaced(bytes, from, to) {
  const altered = Buffer.from(bytes);
  for (let at = altered.indexOf(from); at >= 0; at = altered.indexOf(from, at + 1)) {
    to.copy(altered, at);
  }
  return altered;
}

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

  it("reads the digests a signer signs with", () => {
    const [signer] = theCmsSignature(aReference("cades-implicit.p7s")).signers;
    assert.deepEqual(theDigestsOf(signer), ["sha256", "sha256"]);
    const [ecdsa] = theCmsSignature(aSample("cms-ecdsa.p7s")).signers;
    assert.deepEqual(theDigestsOf(ecdsa), ["sha256", "sha256"]);
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

  it("reads the digest of the signature method, not the one of the references", () => {
    const xml = anXml("xades-enveloping.xml");
    assert.equal(theXadesSignatureDigest(xml), "sha256");
    const sha512 = xml.replace(
      "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
      "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512",
    );
    assert.equal(theXadesSignatureDigest(sha512), "sha512");
    assert.equal(theXadesSignatureDigest(anXml("document.xml")), null);
  });

  it("counts the signatures of a cosignature and the countersignatures among them", () => {
    assert.deepEqual(theXadesSigners(anXml("xades-enveloping.cosign.xml")), {
      signatures: 2,
      countersignatures: 0,
    });
    assert.deepEqual(theXadesSigners(anXml("xades-enveloping.countersign-tree.xml")), {
      signatures: 2,
      countersignatures: 1,
    });
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

  it("reads the signers of the signatures that cover the whole PDF", () => {
    const signers = thePadesSigners(aSample("pades-rsa.pdf"));
    assert.equal(signers.length, 1);
    assert.deepEqual(theDigestsOf(signers[0]), ["sha256", "sha256"]);
  });

  it("refuses a PDF without signature and what is not a PDF", () => {
    assert.equal(isASignedPdf(aPdf("1 0 obj\n<< /Type /Catalog >>\nendobj")), false);
    assert.equal(isASignedPdf(aReference("cades-implicit.p7s")), false);
  });
});

describe("the signature field reader", () => {
  const aPdfWith = (field) =>
    Buffer.from(
      `%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n7 0 obj\n${field}\nendobj\n`,
      "latin1",
    );

  it("reads the area of a visible signature field", () => {
    const field =
      "<</F 132/Type/Annot/Subtype/Widget/Rect[100 100.5 300 200]/FT/Sig/T(Signature1)>>";
    const [area] = theSignatureRectangles(aPdfWith(field));
    assert.deepEqual(area, [100, 100.5, 300, 200]);
    assert.equal(isAVisibleArea(area), true);
  });

  it("tells an invisible signature by its empty area", () => {
    const field = "<< /Type /Annot /Subtype /Widget /Rect [0 0 0 0] /FT /Sig >>";
    const [area] = theSignatureRectangles(aPdfWith(field));
    assert.equal(isAVisibleArea(area), false);
  });

  it("ignores rectangles that are no signature field", () => {
    const link = "<< /Type /Annot /Subtype /Link /Rect [10 10 50 50] >>";
    assert.deepEqual(theSignatureRectangles(aPdfWith(link)), []);
  });

  it("reads a field compressed inside an object stream", () => {
    const objects = "<</Type/Catalog>><</Type/Annot/Subtype/Widget/Rect[100 100 300 200]/FT/Sig>>";
    const header = "3 0 4 17 ";
    const stream = deflateSync(Buffer.from(header + objects, "latin1"));
    const pdf = Buffer.concat([
      Buffer.from(
        `%PDF-1.7\n5 0 obj\n<</Type/ObjStm/N 2/First ${header.length}/Filter/FlateDecode/Length ${stream.length}>>stream\n`,
        "latin1",
      ),
      stream,
      Buffer.from("\nendstream\nendobj\n%%EOF\n", "latin1"),
    ]);
    assert.deepEqual(theSignatureRectangles(pdf), [[100, 100, 300, 200]]);
  });
});

describe("the bare PKCS#1 verifier", () => {
  it("verifies a PKCS#1 signature of the data with the key of the certificate", () => {
    const signature = aSample("pkcs1-rsa.sig");
    assert.equal(isABarePkcs1(theChallenge, signature, aSample("pkcs1-rsa.cer")), true);
  });

  it("refuses other data, a CMS and a certificate that is not DER", () => {
    const certificate = aSample("pkcs1-rsa.cer");
    assert.equal(
      isABarePkcs1(Buffer.from("otros datos"), aSample("pkcs1-rsa.sig"), certificate),
      false,
    );
    assert.equal(isABarePkcs1(theChallenge, aReference("cades-implicit.p7s"), certificate), false);
    assert.equal(isABarePkcs1(theChallenge, aSample("pkcs1-rsa.sig"), Buffer.from("x")), false);
  });
});

describe("the CMS verifier", () => {
  it("verifies an implicit CAdES with the key of the returned certificate", () => {
    const verification = theCmsVerification(aReference("cades-implicit.p7s"), theSigner);
    assert.equal(verification.verified, true, verification.reason);
  });

  it("verifies an explicit CAdES over the data the site sent", () => {
    const cms = aReference("cades-explicit.p7s");
    assert.equal(theCmsVerification(cms, theSigner, theChallenge).verified, true);
    assert.equal(theCmsVerification(cms, theSigner, Buffer.from("otros datos")).verified, false);
    assert.equal(theCmsVerification(cms, theSigner).verified, false);
  });

  it("verifies every signer of a cosignature and of a countersignature tree", () => {
    for (const name of [
      "cades-implicit.cosign.p7s",
      "cades-implicit.countersign-tree.p7s",
      "cades-implicit.countersign-leafs.p7s",
    ]) {
      const verification = theCmsVerification(aReference(name), theSigner);
      assert.equal(verification.verified, true, `${name}: ${verification.reason}`);
    }
  });

  it("verifies an ECDSA signer with the certificate it carries", () => {
    const cms = aSample("cms-ecdsa.p7s");
    const verification = theCmsVerification(cms, theCertificatesIn(cms)[0], theChallenge);
    assert.equal(verification.verified, true, verification.reason);
  });

  it("refuses an implicit CAdES whose content has one byte altered", () => {
    const cms = aReference("cades-implicit.p7s");
    const altered = withAByteFlippedAt(cms, cms.indexOf(theChallenge) + 10);
    const verification = theCmsVerification(altered, theSigner);
    assert.equal(verification.verified, false);
    assert.match(verification.reason, /messageDigest/);
  });

  it("refuses a CAdES whose signature value has one byte altered", () => {
    const cms = aReference("cades-implicit.p7s");
    const [signer] = theCmsSignature(cms).signers;
    const altered = withAByteFlippedAt(cms, cms.indexOf(signer.signature) + 10);
    const verification = theCmsVerification(altered, theSigner);
    assert.equal(verification.verified, false);
    assert.match(verification.reason, /no verifica con ninguna clave/);
  });

  it("refuses a valid CAdES when the returned certificate signed none of it", () => {
    const verification = theCmsVerification(aReference("cades-implicit.p7s"), anotherCertificate);
    assert.equal(verification.verified, false);
  });

  it("leaves unmeasured a digest algorithm it does not implement", () => {
    const altered = withTheOidReplaced(aReference("cades-implicit.p7s"), SHA256_OID, SHA224_OID);
    const verification = theCmsVerification(altered, theSigner);
    assert.equal(verification.verified, null, verification.reason);
    assert.match(verification.reason, /2\.16\.840\.1\.101\.3\.4\.2\.4/);
  });

  it("names the unknown digest of a signer without signed attributes", () => {
    const cms = aSample("cms-sha224-without-attributes.p7s");
    const verification = theCmsVerification(cms, theCertificatesIn(cms)[0]);
    assert.equal(verification.verified, null, verification.reason);
    assert.match(verification.reason, /2\.16\.840\.1\.101\.3\.4\.2\.4/);
  });

  it("refuses a signer whose messageDigest fails behind an unverifiable signature algorithm", () => {
    const cms = aReference("cades-implicit.p7s");
    const [signer] = theCmsSignature(cms).signers;
    const unverifiable = Buffer.from(cms);
    RSASSA_PSS_OID.copy(
      unverifiable,
      cms.lastIndexOf(RSA_ENCRYPTION_OID, cms.indexOf(signer.signature)),
    );
    assert.equal(theCmsVerification(unverifiable, theSigner).verified, null);
    const altered = withAByteFlippedAt(unverifiable, cms.indexOf(theChallenge) + 10);
    const verification = theCmsVerification(altered, theSigner);
    assert.equal(verification.verified, false, verification.reason);
    assert.match(verification.reason, /messageDigest/);
  });

  it("refuses a cosignature whose second signer fails behind an unverifiable first one", () => {
    const cms = aReference("cades-implicit.cosign.p7s");
    const [first, second] = theCmsSignature(cms).signers;
    const digestAt = cms.lastIndexOf(SHA256_OID, cms.indexOf(first.signedAttributes.subarray(2)));
    const unverifiableFirst = Buffer.from(cms);
    SHA224_OID.copy(unverifiableFirst, digestAt);
    assert.equal(theCmsVerification(unverifiableFirst, theSigner).verified, null);
    const alsoBrokenSecond = withAByteFlippedAt(
      unverifiableFirst,
      cms.indexOf(second.signature) + 10,
    );
    const verification = theCmsVerification(alsoBrokenSecond, theSigner);
    assert.equal(verification.verified, false, verification.reason);
    assert.match(verification.reason, /no verifica con ninguna clave/);
  });
});

describe("the PAdES verifier", () => {
  const thePdf = aSample("pades-rsa.pdf");

  it("verifies the CMS over the byte range with the returned certificate", () => {
    const verification = thePadesVerification(thePdf, theSigner);
    assert.equal(verification.verified, true, verification.reason);
  });

  it("refuses a PDF with one byte of its signed range altered", () => {
    const altered = withAByteFlippedAt(thePdf, thePdf.indexOf("suite de conformidad"));
    const verification = thePadesVerification(altered, theSigner);
    assert.equal(verification.verified, false);
    assert.match(verification.reason, /messageDigest/);
  });

  it("refuses a PDF with bytes appended after its signed range", () => {
    const appended = Buffer.concat([thePdf, Buffer.from("\n% cola\n", "latin1")]);
    assert.equal(thePadesVerification(appended, theSigner).verified, false);
  });

  it("refuses a signed PDF when the returned certificate did not sign it", () => {
    assert.equal(thePadesVerification(thePdf, anotherCertificate).verified, false);
  });

  it("leaves unmeasured a digest algorithm it does not implement", () => {
    const hex = (oid) => Buffer.from(oid.toString("hex"), "latin1");
    const altered = withTheOidReplaced(thePdf, hex(SHA256_OID), hex(SHA224_OID));
    const verification = thePadesVerification(altered, theSigner);
    assert.equal(verification.verified, null, verification.reason);
  });
});

describe("the XAdES verifier", () => {
  const anXml = (name) => aReference(name).toString("utf8");

  it("verifies each reference XAdES with the returned certificate", () => {
    for (const name of [
      "xades-enveloping.xml",
      "xades-enveloped.xml",
      "xades-detached.xml",
      "xades-enveloping.cosign.xml",
      "xades-enveloping.countersign-tree.xml",
      "xades-enveloping.countersign-leafs.xml",
      "facturae.xsig",
    ]) {
      const verification = theXadesVerification(anXml(name), theSigner);
      assert.equal(verification.verified, true, `${name}: ${verification.reason}`);
    }
  });

  it("refuses a XAdES whose signed document has one byte altered", () => {
    for (const name of ["xades-enveloping.xml", "xades-enveloped.xml", "xades-detached.xml"]) {
      const altered = anXml(name).replace("Contenido determinista", "Contenido determinisTa");
      assert.equal(theXadesVerification(altered, theSigner).verified, false, name);
    }
  });

  it("refuses a XAdES whose SignedInfo has one byte altered", () => {
    const altered = anXml("xades-enveloping.xml").replace('Id="Reference-3', 'Id="Reference-4');
    const verification = theXadesVerification(altered, theSigner);
    assert.equal(verification.verified, false);
    assert.match(verification.reason, /SignedInfo/);
  });

  it("refuses a valid XAdES when the returned certificate signed none of it", () => {
    assert.equal(
      theXadesVerification(anXml("xades-enveloping.xml"), anotherCertificate).verified,
      false,
    );
  });

  it("leaves unmeasured a canonicalization it does not implement", () => {
    const exclusive = anXml("xades-enveloping.xml").replace(
      /REC-xml-c14n-20010315/,
      "../2001/10/xml-exc-c14n#",
    );
    assert.equal(theXadesVerification(exclusive, theSigner).verified, null);
  });

  it("leaves unmeasured a signature or digest algorithm it does not implement", () => {
    const xml = anXml("xades-enveloping.xml");
    for (const altered of [
      xml.replace("xmldsig-more#rsa-sha256", "xmldsig-more#rsa-sha224"),
      aSample("xades-sha224-reference.xml").toString("utf8"),
    ]) {
      const verification = theXadesVerification(altered, theSigner);
      assert.equal(verification.verified, null, verification.reason);
    }
  });

  it("refuses a Signature whose broken Reference sits beside an unverifiable one", () => {
    const xml = anXml("xades-enveloping.xml");
    const second = xml.indexOf("xmlenc#sha512", xml.indexOf("xmlenc#sha512") + 1);
    const unverifiable = `${xml.slice(0, second)}xmldsig-more#sha224${xml.slice(second + 13)}`;
    const altered = unverifiable.replace("<ds:DigestValue>F", "<ds:DigestValue>G");
    const verification = theXadesVerification(altered, theSigner);
    assert.equal(verification.verified, false, verification.reason);
    assert.match(verification.reason, /Reference/);
  });

  it("refuses a Signature whose SignedInfo fails beside an unverifiable Reference", () => {
    const xml = aSample("xades-sha224-reference.xml").toString("utf8");
    const at = xml.indexOf('-SignatureValue">') + '-SignatureValue">'.length;
    const altered = `${xml.slice(0, at)}${xml[at] === "A" ? "B" : "A"}${xml.slice(at + 1)}`;
    const verification = theXadesVerification(altered, theSigner);
    assert.equal(verification.verified, false, verification.reason);
    assert.match(verification.reason, /SignedInfo/);
  });

  it("refuses a cosignature whose second Signature fails behind an unverifiable first one", () => {
    const xml = anXml("xades-enveloping.cosign.xml").replace(
      "xmldsig-more#rsa-sha256",
      "xmldsig-more#rsa-sha224",
    );
    assert.equal(theXadesVerification(xml, theSigner).verified, null);
    const at = xml.lastIndexOf('-SignatureValue">') + '-SignatureValue">'.length;
    const altered = `${xml.slice(0, at)}${xml[at] === "A" ? "B" : "A"}${xml.slice(at + 1)}`;
    const verification = theXadesVerification(altered, theSigner);
    assert.equal(verification.verified, false, verification.reason);
    assert.match(verification.reason, /SignedInfo/);
  });
});
