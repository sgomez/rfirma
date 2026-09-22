// Los guiones de lote de la sede publicada, remotos contra los servlets del banco y locales.

import { createServer } from "node:http";
import { createServer as createTcpServer } from "node:net";

import {
  aCondition,
  aMeasuredConditionEvent,
  bytesOf,
  emit,
  settle,
  settlingTheError,
} from "../lib/events.mjs";
import {
  theFrozen,
  thePdfOfTheTest,
  theReferenceSignature,
  theXmlDocument,
} from "../lib/fixtures.mjs";
import { theCmsSignature } from "../lib/cms.mjs";
import {
  withJsonbatchCapitalised,
  withLocalBatchProcessOnAnXmlBatch,
  withoutNeedcertInTheBatch,
} from "../lib/patches.mjs";
import { isABarePkcs1 } from "../lib/pkcs1.mjs";
import { aPublishedScript } from "../lib/script.mjs";

const THROUGH_BOTH_SERVLETS = "through-both-servlets";
const THE_PRESIGNER_GETS_THE_CHAIN = "the-presigner-gets-the-batch-and-the-chain";
const THE_POSTSIGNER_GETS_PK1 = "the-postsigner-gets-every-item-with-pk1";
const THE_RESULT_AS_IT_CAME = "the-result-as-it-came";
const THE_CERTIFICATE_IN_THE_ANSWER = "the-certificate-in-the-answer";
const THE_FAILURE_MARKED_FOR_THE_POSTSIGNER = "the-failure-marked-for-the-postsigner";
const THE_ERRORS_WITHOUT_POSTSIGNING = "the-errors-without-postsigning";
const THE_SIGNS_DOCUMENT_AS_IT_CAME = "the-signs-document-as-it-came";
const EVERY_ITEM_SIGNED = "every-item-signed";
const EACH_ITEM_IN_ITS_FORMAT = "each-item-in-its-format";
const THE_SIGNED_ROLLED_BACK = "the-signed-rolled-back";
const THE_REST_SIGNED = "the-rest-signed";
const THE_SUBOPERATION_DONE = "the-suboperation-done";
const THE_ALGORITHM_OF_THE_KEY = "the-algorithm-of-the-key";
const THE_URL_NEITHER_FETCHED_NOR_SIGNED = "the-url-neither-fetched-nor-signed";
const ONLY_THE_RESULT = "only-the-result";
const THE_BATCH_READ_AS_XML = "the-batch-read-as-xml";
const THE_ITEM_A_BARE_PKCS1 = "the-item-a-bare-pkcs1";
const THE_ITEM_EXTRAPARAMS_REPLACE_THE_BATCH_ONES = "the-item-extraparams-replace-the-batch-ones";

/** Los parámetros de la query y los del cuerpo del POST, donde `UrlHttpManagerImpl` los manda. */
async function theServletParameters(request) {
  const parameters = new URL(request.url, "http://127.0.0.2").searchParams;
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  for (const [name, value] of new URLSearchParams(Buffer.concat(chunks).toString("utf8"))) {
    parameters.append(name, value);
  }
  return parameters;
}

/** Un servlet del lote sirviendo HTTP en un puerto libre del loopback, y su URL absoluta. */
export function servletServing(answering) {
  return new Promise((resolve) => {
    const server = createServer(async (request, response) => {
      const { status, body } = answering(await theServletParameters(request));
      response.writeHead(status, { "content-type": "application/json" });
      response.end(body);
    });
    server.listen(0, "127.0.0.2", () => resolve(`http://127.0.0.2:${server.address().port}/batch`));
  });
}

function decodedFromBase64(value) {
  return Buffer.from(value, "base64url").toString("utf8");
}

/** Lo que llegó a recibir cada servlet del lote, para medirlo al cerrar el trámite. */
const whatTheServletsReceived = { presign: null, postsign: null };

/** Lo que ambos servlets exigen del original: el lote en `json` y la cadena en `certs`. */
function missingBatchFields(query) {
  if (!query.get("json")) return "json";
  if (!query.get("certs")) return "certs";
  return null;
}

/** El presigner: comprueba el lote y los `certs`, y devuelve el `TriphaseData` congelado. */
function thePresigner(query) {
  const missing = missingBatchFields(query);
  if (missing) {
    emit({ event: "presign", missing });
    return { status: 400, body: `falta '${missing}'` };
  }

  const lote = JSON.parse(decodedFromBase64(query.get("json")));
  const certs = query.get("certs").split(";").length;
  whatTheServletsReceived.presign = { signs: lote.singlesigns.length, certs };
  emit({
    event: "presign",
    signs: String(lote.singlesigns.length),
    certs: String(certs),
    algorithm: String(lote.algorithm),
  });
  return { status: 200, body: theFrozen("batch-presign-response.json") };
}

/** El postsigner: exige `tridata` con `PK1` en cada firma y devuelve el resultado congelado. */
function thePostsigner(query) {
  const missing = missingBatchFields(query) ?? (query.get("tridata") ? null : "tridata");
  if (missing) {
    emit({ event: "postsign", missing });
    return { status: 400, body: `falta '${missing}'` };
  }

  const tridata = JSON.parse(decodedFromBase64(query.get("tridata")));
  const signs = tridata.signinfo;
  const signed = signs.filter((sign) => !!sign.params.PK1);
  if (signed.length !== signs.length) {
    emit({ event: "postsign", missing: "PK1" });
    return { status: 400, body: "falta 'PK1' en alguna firma del 'tridata'" };
  }

  const withPre = signs.filter((sign) => !!sign.params.PRE).length;
  whatTheServletsReceived.postsign = {
    lote: JSON.parse(decodedFromBase64(query.get("json"))),
    ids: signs.map((sign) => sign.id),
    withPre,
  };
  emit({
    event: "postsign",
    signs: String(signs.length),
    pre: String(withPre),
  });
  return { status: 200, body: theFrozen("batch-postsign-result.json") };
}

/** El presigner que sólo prefirma «uno» y devuelve «dos» como error de prefirma. */
function thePartialPresigner(query) {
  const answer = thePresigner(query);
  if (answer.status !== 200) return answer;

  const partial = JSON.parse(answer.body);
  partial.td.signinfo = partial.td.signinfo.filter((sign) => sign.id === "uno");
  partial.results = [{ id: "dos", result: "ERROR_PRE", description: "no se pudo prefirmar" }];
  return { status: 200, body: JSON.stringify(partial) };
}

/** El presigner que no prefirma nada: sin `td`, sólo el error de cada documento. */
function theFailingPresigner(query) {
  const answer = thePresigner(query);
  if (answer.status !== 200) return answer;

  const results = ["uno", "dos"].map((id) => ({
    id,
    result: "ERROR_PRE",
    description: "no se pudo prefirmar",
  }));
  return { status: 200, body: JSON.stringify({ results }) };
}

/** Los dos callbacks de un lote: el de éxito emite lo que mida `measuring`, y los dos cierran. */
function theBatchCallbacks(measuring) {
  return [
    (result, certificate) => {
      for (const condition of measuring ? measuring(result, String(certificate)) : []) {
        emit({ event: "condition", ...condition });
      }
      settle({
        event: "success",
        result: Buffer.from(JSON.stringify(result), "utf8").toString("base64"),
        certificate: String(certificate),
      });
    },
    settlingTheError,
  ];
}

/** El certificado de la respuesta es un DER suelto: su primer byte abre una `SEQUENCE`. */
function isADerCertificate(certificate) {
  const der = bytesOf(certificate);
  return der.length > 0 && der[0] === 0x30;
}

/** Lo que el lote remoto JSON deja medir al cerrarse: los dos servlets, `PK1`, el resultado y `needcert`. */
function theRemoteBatchConditions(result, certificate) {
  const { presign, postsign } = whatTheServletsReceived;
  const throughBoth = presign !== null && postsign !== null;
  const withTheChain = presign !== null && presign.signs === 2 && presign.certs > 0;
  const signedWithPk1 = postsign !== null && postsign.ids.length === 2 && postsign.withPre === 1;
  const asItCame =
    JSON.stringify(result) === JSON.stringify(JSON.parse(theFrozen("batch-postsign-result.json")));
  const withTheCertificate = isADerCertificate(certificate);
  return [
    aCondition(
      THROUGH_BOTH_SERVLETS,
      throughBoth,
      throughBoth ? "el lote pasó por los dos servlets" : "el lote no llegó a los dos servlets",
    ),
    aCondition(
      THE_PRESIGNER_GETS_THE_CHAIN,
      withTheChain,
      withTheChain
        ? "el presigner recibió los dos documentos y la cadena del firmante"
        : "el presigner no recibió el lote entero con su cadena",
    ),
    aCondition(
      THE_POSTSIGNER_GETS_PK1,
      signedWithPk1,
      signedWithPk1
        ? "las dos firmas llegaron con PK1 y sólo la que lo pedía conservó su PRE"
        : "el tridata del postsigner no trae las dos firmas con PK1 y el PRE que piden",
    ),
    aCondition(
      THE_RESULT_AS_IT_CAME,
      asItCame,
      asItCame
        ? "la sede recibió el resultado del postsigner tal cual"
        : "la sede recibió un resultado distinto del que devolvió el postsigner",
    ),
    aCondition(
      THE_CERTIFICATE_IN_THE_ANSWER,
      withTheCertificate,
      withTheCertificate
        ? "la respuesta trae el certificado del firmante en DER"
        : "la respuesta no trae el certificado del firmante",
    ),
  ];
}

/** El postsigner recibe el lote con «dos» marcado como error de prefirma, y sólo «uno» en el `tridata`. */
function thePartialBatchConditions() {
  const { postsign } = whatTheServletsReceived;
  const failed = postsign?.lote.singlesigns.find((sign) => sign.id === "dos");
  const marked =
    postsign !== null &&
    failed?.result === "ERROR_PRE" &&
    postsign.ids.length === 1 &&
    postsign.ids[0] === "uno";
  return [
    aCondition(
      THE_FAILURE_MARKED_FOR_THE_POSTSIGNER,
      marked,
      marked
        ? "el postsigner recibió «dos» marcado como ERROR_PRE y sólo «uno» firmado"
        : "el postsigner no recibió el lote marcado con el error de prefirma",
    ),
  ];
}

/** Sin nada prefirmado, el lote no llega al postsigner y la sede recibe los errores de prefirma. */
function theFailedBatchConditions(result) {
  const signs = result?.signs ?? [];
  const answered =
    whatTheServletsReceived.postsign === null &&
    signs.length === 2 &&
    signs.every((sign) => sign.result === "ERROR_PRE");
  return [
    aCondition(
      THE_ERRORS_WITHOUT_POSTSIGNING,
      answered,
      answered
        ? "la sede recibió los dos errores de prefirma sin pasar por el postsigner"
        : "el lote fallido no se respondió con sus errores de prefirma sin postsigner",
    ),
  ];
}

/** Un lote de dos documentos con `signBatchJSON` contra los dos servlets, con el presigner dado. */
async function theBatchScript(presigning = thePresigner, measuring = theRemoteBatchConditions) {
  const presigner = await servletServing(presigning);
  const postsigner = await servletServing(thePostsigner);

  AutoScript.createBatch("SHA256", "CAdES", "sign");
  AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
  AutoScript.addDocumentToBatch("dos", Buffer.from("segundo documento").toString("base64"));
  AutoScript.signBatchProcess(true, presigner, postsigner, null, ...theBatchCallbacks(measuring));
}

/** Lo que exige el XML heredado del original: el lote en `xml` y la cadena en `certs`. */
function missingXmlBatchFields(query) {
  if (!query.get("xml")) return "xml";
  if (!query.get("certs")) return "certs";
  return null;
}

/** El presigner del lote XML heredado: comprueba el lote y los `certs` y devuelve el XML congelado. */
function theXmlPresigner(query) {
  const missing = missingXmlBatchFields(query);
  if (missing) {
    emit({ event: "presign", missing });
    return { status: 400, body: `falta '${missing}'` };
  }

  const lote = decodedFromBase64(query.get("xml"));
  const signs = lote.match(/<singlesign\b/g) ?? [];
  const algorithm = /\balgorithm="([^"]+)"/.exec(lote)?.[1];
  whatTheServletsReceived.presign = {
    signs: signs.length,
    certs: query.get("certs").split(";").length,
  };
  emit({
    event: "presign",
    signs: String(signs.length),
    certs: String(query.get("certs").split(";").length),
    algorithm: String(algorithm),
  });
  return { status: 200, body: theFrozen("batch-xml-presign-response.xml") };
}

/** El postsigner del lote XML heredado: exige `PK1` en cada firma y devuelve el resultado congelado. */
function theXmlPostsigner(query) {
  const missing = missingXmlBatchFields(query) ?? (query.get("tridata") ? null : "tridata");
  if (missing) {
    emit({ event: "postsign", missing });
    return { status: 400, body: `falta '${missing}'` };
  }

  const tridata = decodedFromBase64(query.get("tridata"));
  const signs = tridata.match(/<firma\b/g) ?? [];
  const withPk1 = tridata.match(/<param n="PK1">/g) ?? [];
  if (withPk1.length !== signs.length) {
    emit({ event: "postsign", missing: "PK1" });
    return { status: 400, body: "falta 'PK1' en alguna firma del 'tridata'" };
  }

  const withPre = tridata.match(/<param n="PRE">/g) ?? [];
  whatTheServletsReceived.postsign = {
    lote: null,
    ids: [...tridata.matchAll(/<firma\b[^>]*\bId="([^"]+)"/g)].map((match) => match[1]),
    withPre: withPre.length,
  };
  emit({
    event: "postsign",
    signs: String(signs.length),
    pre: String(withPre.length),
  });
  return { status: 200, body: theFrozen("batch-xml-postsign-result.xml") };
}

/** Un lote de dos documentos firmado con el `signBatch` heredado contra los dos servlets del banco. */
async function theBatchXmlScript() {
  const presigner = await servletServing(theXmlPresigner);
  const postsigner = await servletServing(theXmlPostsigner);

  const lote =
    '<signbatch algorithm="SHA256" stoponerror="false">' +
    '<singlesign id="uno"/><singlesign id="dos"/></signbatch>';
  const batchB64 = Buffer.from(lote, "utf8").toString("base64");

  AutoScript.signBatch(
    batchB64,
    presigner,
    postsigner,
    null,
    (result, certificate) => {
      for (const condition of theXmlBatchConditions(String(result), String(certificate))) {
        emit({ event: "condition", ...condition });
      }
      settle({
        // El `signBatch` heredado nunca decodifica `result`: el original hace pasar el
        // resultado por `AfirmaUtils.parseJSONData`, que revienta con XML y lo deja en base64.
        event: "success",
        result: String(result),
        certificate: String(certificate),
      });
    },
    settlingTheError,
  );
}

/** El lote XML se mide como el JSON: los dos servlets, la cadena, `PK1`, el resultado y el certificado. */
function theXmlBatchConditions(result, certificate) {
  const { presign, postsign } = whatTheServletsReceived;
  const throughBoth = presign !== null && postsign !== null;
  const withTheChain = presign !== null && presign.signs === 2 && presign.certs > 0;
  const signedWithPk1 = postsign !== null && postsign.ids.length === 2 && postsign.withPre === 1;
  const withTheCertificate = isADerCertificate(certificate);
  const compact = (text) => text.replace(/\s+/g, "");
  const asItCame =
    compact(bytesOf(result).toString("utf8")) ===
    compact(theFrozen("batch-xml-postsign-result.xml"));
  return [
    aCondition(
      THROUGH_BOTH_SERVLETS,
      throughBoth,
      throughBoth
        ? "el lote XML pasó por los dos servlets"
        : "el lote XML no llegó a los dos servlets",
    ),
    aCondition(
      THE_SIGNS_DOCUMENT_AS_IT_CAME,
      asItCame,
      asItCame
        ? "la sede recibió el <signs> del postsigner tal cual"
        : "la sede recibió algo distinto del <signs> que devolvió el postsigner",
    ),
    aCondition(
      THE_PRESIGNER_GETS_THE_CHAIN,
      withTheChain,
      withTheChain
        ? "el presigner recibió los dos documentos en xml y la cadena del firmante"
        : "el presigner no recibió el lote XML entero con su cadena",
    ),
    aCondition(
      THE_POSTSIGNER_GETS_PK1,
      signedWithPk1,
      signedWithPk1
        ? "las dos firmas llegaron con PK1 y sólo la que lo pedía conservó su PRE"
        : "el tridata XML no trae las dos firmas con PK1 y el PRE que piden",
    ),
    aCondition(
      THE_CERTIFICATE_IN_THE_ANSWER,
      withTheCertificate,
      withTheCertificate
        ? "la respuesta trae el certificado del firmante en DER"
        : "la respuesta no trae el certificado del firmante",
    ),
  ];
}

/** El binario del lote local: nunca es un PDF, así que declararlo `PAdES` lo vuelve ilegible. */
function theLocalBatchBinary() {
  return Buffer.from("contenido binario del lote local, sin PDF ni XML dentro", "utf8");
}

/** Un documento del lote local: su id, su contenido en Base64 y, si los declara, su formato y sus `extraParams`. */
function anItem(id, content, format, extraParams) {
  return [id, content.toString("base64"), format, extraParams];
}

const thePdfItem = (format = "PAdES") => anItem("pdf", thePdfOfTheTest(), format);
const theBinaryItem = (format) => anItem("bin", theLocalBatchBinary(), format);
const theXmlItem = (format = "XAdES") => anItem("xml", theXmlDocument(), format);

/** Un lote local de `setLocalBatchProcess(true)` sobre `items`, sin presigner ni postsigner. */
function aLocalBatch({
  format = "CAdES",
  suboperation = "sign",
  extraParams = null,
  stopOnError,
  items,
  callbacks,
}) {
  AutoScript.setLocalBatchProcess(true);
  AutoScript.createBatch("SHA256", format, suboperation, extraParams);
  for (const [id, content, itemFormat, extraParams] of items) {
    AutoScript.addDocumentToBatch(id, content, itemFormat, undefined, extraParams);
  }
  AutoScript.signBatchProcess(stopOnError, null, null, null, ...callbacks);
}

function theLocalItems(result) {
  return new Map((result?.signs ?? []).map((sign) => [sign.id, sign]));
}

/** Si el elemento salió firmado y su firma es de la clase dada: un PDF, un CMS o un XML. */
function signedAs(item, kind) {
  if (item?.result !== "DONE_AND_SAVED" || !item.signature) return false;
  const bytes = bytesOf(item.signature);
  if (kind === "pdf") return bytes.subarray(0, 5).toString("latin1") === "%PDF-";
  if (kind === "cms") return bytes.length > 0 && bytes[0] === 0x30;
  return bytes.toString("utf8").trimStart().startsWith("<");
}

function eachInItsFormat(items) {
  return (
    signedAs(items.get("pdf"), "pdf") &&
    signedAs(items.get("bin"), "cms") &&
    signedAs(items.get("xml"), "xml")
  );
}

/** El lote local firmó los tres documentos, cada uno con su firma y en el formato que le toca. */
function theLocalBatchConditions(result) {
  const items = theLocalItems(result);
  const everySigned = ["pdf", "bin", "xml"].every(
    (id) => items.get(id)?.result === "DONE_AND_SAVED" && !!items.get(id)?.signature,
  );
  const inTheirFormats = eachInItsFormat(items);
  return [
    aCondition(
      EVERY_ITEM_SIGNED,
      everySigned,
      everySigned
        ? "los tres documentos volvieron DONE_AND_SAVED con su firma"
        : "algún documento no volvió firmado con su firma",
    ),
    aCondition(
      EACH_ITEM_IN_ITS_FORMAT,
      inTheirFormats,
      inTheirFormats
        ? "el PDF volvió en PAdES, el binario en CAdES y el XML en XAdES"
        : "algún documento no volvió firmado en el formato que declaraba o heredaba",
    ),
  ];
}

/** El lote local: un PDF en `PAdES`, un binario que hereda `CAdES` y un XML en `XAdES`. */
function theLocalBatchScript() {
  aLocalBatch({
    stopOnError: false,
    items: [thePdfItem(), theBinaryItem(), theXmlItem()],
    callbacks: theBatchCallbacks(theLocalBatchConditions),
  });
}

/** El mismo lote local con el binario declarado `PAdES`, ilegible, y `stoponerror=true`. */
function theLocalBatchWithAnIllegibleItemScript() {
  aLocalBatch({
    stopOnError: true,
    items: [thePdfItem(), theBinaryItem("PAdES"), theXmlItem()],
    callbacks: theBatchCallbacks((result) => {
      const items = theLocalItems(result);
      const rolledBack =
        items.get("pdf")?.result === "SKIPPED" &&
        !items.get("pdf")?.signature &&
        items.get("bin")?.result === "ERROR_PRE" &&
        items.get("xml")?.result === "SKIPPED";
      return [
        aCondition(
          THE_SIGNED_ROLLED_BACK,
          rolledBack,
          rolledBack
            ? "el PDF firmado se deshizo, el binario falló y el XML se saltó"
            : "el lote no deshizo lo firmado ni saltó lo que quedaba",
        ),
      ];
    }),
  });
}

/** El mismo lote con el binario ilegible y `stoponerror=false`: el fallo no para a los demás. */
function theLocalBatchContinuingPastAnIllegibleItemScript() {
  aLocalBatch({
    stopOnError: false,
    items: [thePdfItem(), theBinaryItem("PAdES"), theXmlItem()],
    callbacks: theBatchCallbacks((result) => {
      const items = theLocalItems(result);
      const carriedOn =
        signedAs(items.get("pdf"), "pdf") &&
        items.get("bin")?.result === "ERROR_PRE" &&
        signedAs(items.get("xml"), "xml");
      return [
        aCondition(
          THE_REST_SIGNED,
          carriedOn,
          carriedOn
            ? "el binario falló y el PDF y el XML salieron firmados"
            : "el fallo del binario arrastró a los demás documentos",
        ),
      ];
    }),
  });
}

/** El lote local con `format=auto`: ningún documento declara el suyo. */
function theLocalBatchInFormatAutoScript() {
  aLocalBatch({
    format: "auto",
    stopOnError: false,
    items: [thePdfItem(null), theBinaryItem(), theXmlItem(null)],
    callbacks: theBatchCallbacks((result) => {
      const resolved = eachInItsFormat(theLocalItems(result));
      return [
        aCondition(
          EACH_ITEM_IN_ITS_FORMAT,
          resolved,
          resolved
            ? "el PDF salió en PAdES, el binario en CAdES y el XML en XAdES"
            : "format=auto no se resolvió por el contenido de cada documento",
        ),
      ];
    }),
  });
}

/** El lote local de una sola firma de referencia con la suboperación dada. */
function theLocalBatchOfASignatureScript(suboperation) {
  aLocalBatch({
    suboperation,
    stopOnError: false,
    items: [anItem("firma", theReferenceSignature("cades-implicit.p7s"))],
    callbacks: theBatchCallbacks((result) => {
      const done = signedAs(theLocalItems(result).get("firma"), "cms");
      return [
        aCondition(
          THE_SUBOPERATION_DONE,
          done,
          done
            ? `la firma de referencia volvió DONE_AND_SAVED con ${suboperation}`
            : `el lote no hizo ${suboperation} sobre la firma de referencia`,
        ),
      ];
    }),
  });
}

/** El lote local de un PDF que pide `visibleSignature=want`, que el lote tiene que ignorar. */
function theLocalBatchAskingForAVisibleSignatureScript() {
  aLocalBatch({
    stopOnError: false,
    items: [anItem("pdf", thePdfOfTheTest(), "PAdES", "visibleSignature=want")],
    callbacks: theBatchCallbacks(),
  });
}

/** El OID `ecdsa-with-SHA256` (1.2.840.10045.4.3.2) con su etiqueta y su longitud DER. */
const ECDSA_WITH_SHA256 = Buffer.from([0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]);

/** El lote local de un binario con `SHA256` a secas: el algoritmo lo completa la clave elegida. */
function theLocalBatchWithAnEllipticKeyScript() {
  aLocalBatch({
    stopOnError: false,
    items: [theBinaryItem()],
    callbacks: theBatchCallbacks((result) => {
      const item = theLocalItems(result).get("bin");
      const composed = signedAs(item, "cms") && bytesOf(item.signature).includes(ECDSA_WITH_SHA256);
      return [
        aCondition(
          THE_ALGORITHM_OF_THE_KEY,
          composed,
          composed
            ? "el binario volvió firmado con ecdsa-with-SHA256"
            : "la firma del binario no es ecdsa-with-SHA256",
        ),
      ];
    }),
  });
}

/** El lote local con una URL servida aquí en `datareference`: se cumple si nadie la pide. */
async function theLocalBatchWithAUrlScript() {
  let fetched = false;
  const url = await servletServing(() => {
    fetched = true;
    return { status: 200, body: "%PDF-1.4" };
  });
  const theCondition = (held, observation) =>
    aCondition(
      THE_URL_NEITHER_FETCHED_NOR_SIGNED,
      held && !fetched,
      fetched ? "el sujeto descargó la URL de datareference" : observation,
    );

  const [succeeded, failed] = theBatchCallbacks((result) => {
    const signed = theLocalItems(result).get("url")?.result === "DONE_AND_SAVED";
    return [
      theCondition(
        !signed,
        signed ? "el documento de la URL salió firmado" : "el documento de la URL no salió firmado",
      ),
    ];
  });
  AutoScript.setLocalBatchProcess(true);
  AutoScript.createBatch("SHA256", "PAdES", "sign", null);
  AutoScript.addDocumentToBatch("url", url);
  AutoScript.signBatchProcess(false, null, null, null, succeeded, (type, message) => {
    if (String(message).includes("SAF_")) {
      emit({
        event: "condition",
        ...theCondition(true, "el lote se rechazó sin descargar la URL"),
      });
    }
    failed(type, message);
  });
}

/** Sin `needcert`, el lote vuelve sólo con el resultado: el cliente publicado no ve certificado. */
function theResultAlone(result, certificate) {
  const alone =
    ["null", "undefined", ""].includes(certificate) &&
    JSON.stringify(result) === JSON.stringify(JSON.parse(theFrozen("batch-postsign-result.json")));
  return [
    aCondition(
      ONLY_THE_RESULT,
      alone,
      alone
        ? "la respuesta del lote trae sólo el resultado del postsigner"
        : `la respuesta del lote trae certificado (${certificate.slice(0, 16)}…) o un resultado distinto`,
    ),
  ];
}

/** Con `jsonBatch`, el lote se lee como XML heredado: al presigner le llega `xml` y no `json`. */
async function theBatchWithJsonbatchCapitalisedScript() {
  let received = null;
  const presigner = await servletServing((query) => {
    received = { xml: query.has("xml"), json: query.has("json") };
    return { status: 400, body: "el lote de esta prueba no se prefirma" };
  });
  const theCondition = () =>
    aMeasuredConditionEvent(
      THE_BATCH_READ_AS_XML,
      received === null ? null : received.xml && !received.json,
      received === null
        ? "el lote no llegó al presigner"
        : `el presigner recibió ${received.json ? "json" : received.xml ? "xml" : "ni json ni xml"}`,
    );

  AutoScript.createBatch("SHA256", "CAdES", "sign");
  AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
  AutoScript.signBatchProcess(
    true,
    presigner,
    presigner,
    null,
    (result, certificate) => {
      emit(theCondition());
      settle({ event: "success", result: String(result), certificate: String(certificate) });
    },
    (type, message) => {
      emit(theCondition());
      settlingTheError(type, message);
    },
  );
}

/** Un puerto del loopback que se ata y se suelta al momento, para que no lo atienda nadie. */
function anUnattendedPort() {
  return new Promise((resolve) => {
    const server = createTcpServer();
    server.listen(0, "127.0.0.2", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/** Un lote remoto contra un servlet en un puerto que nadie atiende: si se contacta, falla al momento. */
async function theBatchAgainstAnUnattendedServletScript(servletAt) {
  const servlet = servletAt(await anUnattendedPort());

  AutoScript.createBatch("SHA256", "CAdES", "sign");
  AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
  AutoScript.addDocumentToBatch("dos", Buffer.from("segundo documento").toString("base64"));
  AutoScript.signBatchProcess(true, servlet, servlet, null, ...theBatchCallbacks());
}

/** Un lote sin presigner escuchando, que tiene que acabar en `SAF_26`. */
function theBatchWithTheDownPresignerScript() {
  return theBatchAgainstAnUnattendedServletScript((port) => `http://127.0.0.2:${port}/batch`);
}

const anUnattendedServletAt = (url) => () => theBatchAgainstAnUnattendedServletScript(url);

/** El lote local en `NONE` sobre el binario: vuelve el PKCS#1 suelto que verifica el certificado. */
function theLocalBatchInFormatNoneScript() {
  aLocalBatch({
    format: "NONE",
    stopOnError: false,
    items: [theBinaryItem()],
    callbacks: theBatchCallbacks((result, certificate) => {
      const item = theLocalItems(result).get("bin");
      const bare =
        item?.result === "DONE_AND_SAVED" &&
        !!item.signature &&
        isABarePkcs1(theLocalBatchBinary(), bytesOf(item.signature), bytesOf(certificate));
      return [
        aCondition(
          THE_ITEM_A_BARE_PKCS1,
          bare,
          bare
            ? "el binario volvió con un PKCS#1 suelto que el certificado verifica"
            : "el binario no volvió con un PKCS#1 suelto de sus datos",
        ),
      ];
    }),
  });
}

/** Un presigner que contesta siempre con el estado HTTP dado, sin llegar a prefirmar. */
function aPresignerAnswering(status) {
  return async () => {
    const presigner = await servletServing(() => ({
      status,
      body: `el presigner responde ${status}`,
    }));
    AutoScript.createBatch("SHA256", "CAdES", "sign");
    AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
    AutoScript.signBatchProcess(true, presigner, presigner, null, ...theBatchCallbacks());
  };
}

/** Un lote local de un solo binario, para cancelar el diálogo de certificado que abre. */
function theLocalBatchToCancelScript() {
  aLocalBatch({ stopOnError: false, items: [theBinaryItem()], callbacks: theBatchCallbacks() });
}

/** Un lote local sin `format`: `createBatch` con el formato `undefined` lo deja fuera del JSON. */
function theLocalBatchWithoutAFormatScript() {
  AutoScript.setLocalBatchProcess(true);
  AutoScript.createBatch("SHA256", undefined, "sign", null);
  AutoScript.addDocumentToBatch("bin", theLocalBatchBinary().toString("base64"));
  AutoScript.signBatchProcess(false, null, null, null, ...theBatchCallbacks());
}

function carriesItsContent(item) {
  if (item?.result !== "DONE_AND_SAVED" || !item.signature) return null;
  return theCmsSignature(bytesOf(item.signature))?.content != null;
}

/** El lote pide `mode=implicit`; «propio» trae sus `extraParams` y «heredado» no, así que solo este lo hereda. */
function theLocalBatchWithItemExtraParamsScript() {
  aLocalBatch({
    extraParams: "mode=implicit",
    stopOnError: false,
    items: [
      anItem("heredado", theLocalBatchBinary()),
      anItem("propio", theLocalBatchBinary(), undefined, "contentDescription=documento propio"),
    ],
    callbacks: theBatchCallbacks((result) => {
      const items = theLocalItems(result);
      const inherited = carriesItsContent(items.get("heredado"));
      const own = carriesItsContent(items.get("propio"));
      const replaced = inherited === true && own === false;
      return [
        aCondition(
          THE_ITEM_EXTRAPARAMS_REPLACE_THE_BATCH_ONES,
          replaced,
          inherited === null || own === null
            ? "algún documento no volvió firmado en CAdES"
            : `el documento sin extraParams ${inherited ? "heredó" : "no heredó"} mode=implicit y el que trae los suyos ${own ? "también lo aplicó" : "no lo aplicó"}`,
        ),
      ];
    }),
  });
}

/** Un lote XML con `localBatchProcess=true` y sin servlets, que el publicado sólo manda en JSON. */
function theLocalXmlBatchScript() {
  const lote =
    '<signbatch algorithm="SHA256" stoponerror="false">' + '<singlesign id="uno"/></signbatch>';
  AutoScript.setLocalBatchProcess(true);
  AutoScript.signBatch(
    Buffer.from(lote, "utf8").toString("base64"),
    null,
    null,
    null,
    (result, certificate) =>
      settle({ event: "success", result: String(result), certificate: String(certificate) }),
    settlingTheError,
  );
}

export const BATCH_SCRIPTS = {
  batch: aPublishedScript(() => theBatchScript(), {
    conditions: [
      THROUGH_BOTH_SERVLETS,
      THE_PRESIGNER_GETS_THE_CHAIN,
      THE_POSTSIGNER_GETS_PK1,
      THE_RESULT_AS_IT_CAME,
      THE_CERTIFICATE_IN_THE_ANSWER,
    ],
  }),
  batchpartial: aPublishedScript(
    () => theBatchScript(thePartialPresigner, thePartialBatchConditions),
    { conditions: [THE_FAILURE_MARKED_FOR_THE_POSTSIGNER] },
  ),
  batchallfailed: aPublishedScript(
    () => theBatchScript(theFailingPresigner, theFailedBatchConditions),
    { conditions: [THE_ERRORS_WITHOUT_POSTSIGNING] },
  ),
  batchxml: aPublishedScript(theBatchXmlScript, {
    conditions: [
      THROUGH_BOTH_SERVLETS,
      THE_SIGNS_DOCUMENT_AS_IT_CAME,
      THE_PRESIGNER_GETS_THE_CHAIN,
      THE_POSTSIGNER_GETS_PK1,
      THE_CERTIFICATE_IN_THE_ANSWER,
    ],
  }),
  batchwithoutneedcert: aPublishedScript(() => theBatchScript(thePresigner, theResultAlone), {
    conditions: [ONLY_THE_RESULT],
    patch: withoutNeedcertInTheBatch,
  }),
  batchwithjsonbatchcapitalised: aPublishedScript(theBatchWithJsonbatchCapitalisedScript, {
    conditions: [THE_BATCH_READ_AS_XML],
    patch: withJsonbatchCapitalised,
  }),
  batchdown: aPublishedScript(theBatchWithTheDownPresignerScript),
  batchloopbackservlet: aPublishedScript(
    anUnattendedServletAt((port) => `http://127.0.0.1:${port}/batch`),
  ),
  batchservletwithparameters: aPublishedScript(
    anUnattendedServletAt((port) => `http://127.0.0.2:${port}/batch?op=pre`),
  ),
  batchlocal: aPublishedScript(theLocalBatchScript, {
    conditions: [EVERY_ITEM_SIGNED, EACH_ITEM_IN_ITS_FORMAT],
  }),
  batchlocalillegible: aPublishedScript(theLocalBatchWithAnIllegibleItemScript, {
    conditions: [THE_SIGNED_ROLLED_BACK],
  }),
  batchlocalcontinuing: aPublishedScript(theLocalBatchContinuingPastAnIllegibleItemScript, {
    conditions: [THE_REST_SIGNED],
  }),
  batchlocalauto: aPublishedScript(theLocalBatchInFormatAutoScript, {
    conditions: [EACH_ITEM_IN_ITS_FORMAT],
  }),
  batchlocalcosign: aPublishedScript(() => theLocalBatchOfASignatureScript("cosign"), {
    conditions: [THE_SUBOPERATION_DONE],
  }),
  batchlocalcountersign: aPublishedScript(() => theLocalBatchOfASignatureScript("countersign"), {
    conditions: [THE_SUBOPERATION_DONE],
  }),
  batchlocalvisible: aPublishedScript(theLocalBatchAskingForAVisibleSignatureScript),
  batchlocalecdsa: aPublishedScript(theLocalBatchWithAnEllipticKeyScript, {
    conditions: [THE_ALGORITHM_OF_THE_KEY],
  }),
  batchlocalurl: aPublishedScript(theLocalBatchWithAUrlScript, {
    conditions: [THE_URL_NEITHER_FETCHED_NOR_SIGNED],
  }),
  batchlocalnone: aPublishedScript(theLocalBatchInFormatNoneScript, {
    conditions: [THE_ITEM_A_BARE_PKCS1],
  }),
  batchpresigner400: aPublishedScript(aPresignerAnswering(400)),
  batchpresigner404: aPublishedScript(aPresignerAnswering(404)),
  batchpresigner500: aPublishedScript(aPresignerAnswering(500)),
  batchcancelled: aPublishedScript(theLocalBatchToCancelScript),
  batchlocalwithoutformat: aPublishedScript(theLocalBatchWithoutAFormatScript),
  batchlocalextraparams: aPublishedScript(theLocalBatchWithItemExtraParamsScript, {
    conditions: [THE_ITEM_EXTRAPARAMS_REPLACE_THE_BATCH_ONES],
  }),
  batchlocalxml: aPublishedScript(theLocalXmlBatchScript, {
    patch: withLocalBatchProcessOnAnXmlBatch,
  }),
};
