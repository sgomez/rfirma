// Los guiones de la sede publicada por servidor intermedio, con sus servlets servidos por HTTP.

import { createServer } from "node:http";

import { theLaunchesSoFar } from "../lib/browser.mjs";
import {
  aConditionEvent,
  aMeasuredConditionEvent,
  bytesOf,
  emit,
  settle,
  settlingTheError,
} from "../lib/events.mjs";
import { theInvoice } from "../lib/fixtures.mjs";
import { aPublishedScript } from "../lib/script.mjs";
import { BATCH_SCRIPTS } from "./batch.mjs";

const THE_RESULT_UPLOADED_CIPHERED = "the-result-uploaded-ciphered";
const WAIT_ANNOUNCED_EVERY_TEN_SECONDS = "wait-announced-every-ten-seconds";
const THE_REQUEST_RETRIEVED_BY_FILEID = "the-request-retrieved-by-fileid";
const EACH_OPERATION_LAUNCHED_APART = "each-operation-launched-apart";
const THE_RETRIEVAL_FAILURE_NOT_UPLOADED = "the-retrieval-failure-not-uploaded";
const A_SAF_AFTER_THE_START_TRAVELS_INTACT = "a-saf-after-the-start-travels-intact";
const A_SAF_BEFORE_THE_START_IS_NOT_UPLOADED = "a-saf-before-the-start-is-not-uploaded";
const THE_LOCAL_STORAGE_SERVLET_REFUSED = "the-local-storage-servlet-refused";
const THE_UNDECIPHERABLE_REQUEST_NOT_UPLOADED = "the-undecipherable-request-not-uploaded";
const THE_REFUSED_UPLOAD_ATTEMPTED = "the-refused-upload-attempted";

const THE_STORAGE_PATH = "/afirma-signature-storage/StorageService";
const THE_RETRIEVE_PATH = "/afirma-signature-retriever/RetrieveService";
const THE_WAIT_MARK = "#WAIT";

/** Lo que el RetrieveService entrega en lugar de la petición: no es Base64 y no descifra con ninguna clave. */
const AN_UNDECIPHERABLE_REQUEST = "0.QUJDREVG";

/** Lo que tarda como mucho entre dos avisos de espera, con holgura sobre los diez segundos. */
const THE_WAIT_PERIOD_MS = { from: 8000, to: 13000 };

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

function listening(server, host) {
  return new Promise((resolve) => server.listen(0, host, () => resolve(server.address().port)));
}

/**
 * El StorageService y el RetrieveService por HTTP en el loopback, con el registro de cada petición;
 * `retrieving` puede contestar un `op=get` en lugar de lo guardado y `refusingUploads` contesta cada
 * `op=put` con un 500.
 */
async function anIntermediateServer({ retrieving = () => undefined, refusingUploads = false } = {}) {
  const stored = new Map();
  const requests = [];
  const serving = (listener) => async (request, response) => {
    const parameters = await theServletParameters(request);
    const entry = {
      listener,
      service: new URL(request.url, "http://127.0.0.2").pathname,
      op: parameters.get("op"),
      id: parameters.get("id"),
      dat: parameters.get("dat"),
      at: Date.now(),
    };
    requests.push(entry);
    let answer = "OK";
    if (entry.op === "put" && refusingUploads) {
      response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
      response.end("err-00:= La sede no guarda nada");
      return;
    }
    if (entry.op === "put") {
      stored.set(entry.id, entry.dat);
      emit({ event: "stored", id: String(entry.id), dat: String(entry.dat) });
    } else if (entry.op === "get") {
      answer = retrieving(entry, requests) ?? stored.get(entry.id);
      stored.delete(entry.id);
      answer ??= "err-06:= No existe el identificador";
    }
    response.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
    response.end(answer);
  };
  const port = await listening(createServer(serving("remote")), "127.0.0.2");
  const loopbackPort = await listening(createServer(serving("loopback")), "127.0.0.1");
  const at = (host, portNumber, path) => `http://${host}:${portNumber}${path}`;
  return {
    storage: at("127.0.0.2", port, THE_STORAGE_PATH),
    retrieve: at("127.0.0.2", port, THE_RETRIEVE_PATH),
    loopbackStorage: at("127.0.0.1", loopbackPort, THE_STORAGE_PATH),
    requests,
    putsTo: (service) =>
      requests.filter((entry) => entry.op === "put" && entry.service === service),
    getsFrom: (service) =>
      requests.filter((entry) => entry.op === "get" && entry.service === service),
  };
}

/** Lo que la aplicación subió al StorageService que no es la marca de espera. */
function theUploadedResults(server) {
  return server.putsTo(THE_STORAGE_PATH).filter((entry) => entry.dat !== THE_WAIT_MARK);
}

/** Cada parte de la respuesta como la cifra `CypherDataManager`: su relleno, un punto y Base64. */
function isCipheredWithTheKey(uploaded) {
  return uploaded.split("|").every((part) => /^[0-7]\.[A-Za-z0-9_-]+=*$/.test(part));
}

function theCipheredResultCondition(server, certificate) {
  const results = theUploadedResults(server);
  const asCertificate = bytesOf(certificate);
  const ciphered =
    results.length > 0 &&
    results.every((entry) => isCipheredWithTheKey(entry.dat)) &&
    asCertificate[0] === 0x30;
  return aConditionEvent(
    THE_RESULT_UPLOADED_CIPHERED,
    ciphered,
    results.length === 0
      ? "la aplicación no subió ningún resultado"
      : `subió ${results.map((entry) => entry.dat.slice(0, 24)).join(", ")}…; la página descifró ${asCertificate.length} bytes`,
  );
}

function theWaitCondition(server) {
  const waits = server
    .putsTo(THE_STORAGE_PATH)
    .filter((entry) => entry.dat === THE_WAIT_MARK)
    .map((entry) => entry.at);
  if (waits.length < 2) {
    return aMeasuredConditionEvent(
      WAIT_ANNOUNCED_EVERY_TEN_SECONDS,
      null,
      `${waits.length} avisos de espera: la operación no duró lo bastante para medir el periodo`,
    );
  }
  const gaps = waits.slice(1).map((at, index) => at - waits[index]);
  return aConditionEvent(
    WAIT_ANNOUNCED_EVERY_TEN_SECONDS,
    gaps.every((gap) => gap >= THE_WAIT_PERIOD_MS.from && gap <= THE_WAIT_PERIOD_MS.to),
    `${waits.length} avisos de espera, separados ${gaps.map((gap) => `${Math.round(gap / 1000)} s`).join(", ")}`,
  );
}

/** Una selección de certificado por servidor intermedio, medida desde los servlets. */
async function theSelectionThroughTheServerScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.storage, server.retrieve);
  AutoScript.selectCertificate(
    "",
    (certificate) => {
      emit(theCipheredResultCondition(server, certificate));
      emit(theWaitCondition(server));
      settle({ event: "success", data: String(certificate) });
    },
    settlingTheError,
  );
}

/** Un documento que en base64 pasa de `MAX_LONG_GENERAL_URL` y obliga al servidor intermedio. */
function aDocumentTooLongForTheUrl() {
  return Buffer.from("%PDF-1.7\n".concat("d".repeat(3000)), "utf8");
}

/** La firma larga que la página sube al StorageService y la aplicación recupera con `fileid`. */
async function theRelayScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.storage, server.retrieve);
  AutoScript.sign(
    aDocumentTooLongForTheUrl().toString("base64"),
    "SHA256withRSA",
    "CAdES",
    "mode=explicit",
    (signature, certificate) => {
      const [upload] = server.putsTo(THE_STORAGE_PATH);
      const retrieved = server
        .getsFrom(THE_RETRIEVE_PATH)
        .some((entry) => upload && entry.id === upload.id);
      emit(
        aConditionEvent(
          THE_REQUEST_RETRIEVED_BY_FILEID,
          retrieved,
          retrieved
            ? `la aplicación recuperó la petición ${upload.id} del RetrieveService`
            : "nadie pidió al RetrieveService la petición que subió la página",
        ),
      );
      settle({ event: "success", result: String(signature), certificate: String(certificate) });
    },
    settlingTheError,
  );
}

/** La URL larga por servidor intermedio con una cofirma de factura, que no se admite. */
function theRelayRefusedScript() {
  AutoScript.setServlets(
    "https://sede.example/afirma-signature-storage/StorageService",
    "https://sede.example/afirma-signature-retriever/RetrieveService",
  );
  AutoScript.cosign(
    theInvoice().toString("base64"),
    "SHA256withRSA",
    "FacturaE",
    "",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

/** Una selección de certificado resuelta cuando contesta, con lo que contestó. */
function aSelection() {
  return new Promise((resolve) => {
    AutoScript.selectCertificate(
      "",
      (data) => resolve({ data: String(data) }),
      (type, message) => resolve({ type: String(type), message: String(message) }),
    );
  });
}

/** Dos selecciones seguidas por servidor intermedio: cada una tiene que invocar la aplicación. */
async function theTwoSelectionsScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.storage, server.retrieve);
  const first = await aSelection();
  const second = await aSelection();
  const launches = theLaunchesSoFar();
  const both = first.data !== undefined && second.data !== undefined;
  emit(
    aConditionEvent(
      EACH_OPERATION_LAUNCHED_APART,
      both && launches === 2,
      `${launches} invocaciones para dos selecciones; ` +
        `primera: ${first.data ? "certificado" : first.message}; segunda: ${second.data ? "certificado" : second.message}`,
    ),
  );
  settle(both ? { event: "success", data: second.data } : { event: "error", ...second });
}

/** La primera subida de la página, la de la petición que la aplicación recupera con `fileid`. */
function theFileidIn(requests) {
  return requests.find((entry) => entry.op === "put")?.id ?? null;
}

/**
 * Una firma larga cuyo `fileid` el RetrieveService contesta con `answer`: la aplicación lo pide y,
 * sin la petición, no tiene dónde subir nada.
 */
function aSpoiledRetrievalScript(answer, condition) {
  return async () => {
    const server = await anIntermediateServer({
      retrieving: (entry, requests) => (entry.id === theFileidIn(requests) ? answer : undefined),
    });
    AutoScript.setServlets(server.storage, server.retrieve);
    const settling = (event) => {
      const fileid = theFileidIn(server.requests);
      const asked = server.getsFrom(THE_RETRIEVE_PATH).some((entry) => entry.id === fileid);
      const uploaded = theUploadedResults(server).filter((entry) => entry.id !== fileid);
      emit(
        aMeasuredConditionEvent(
          condition,
          asked ? uploaded.length === 0 : null,
          !asked
            ? "la aplicación no llegó a pedir la petición al RetrieveService"
            : uploaded.length === 0
              ? `la aplicación pidió la petición, recibió ${answer} y no subió nada`
              : `la aplicación subió ${uploaded.map((entry) => entry.dat).join(", ")}`,
        ),
      );
      settle(event);
    };
    AutoScript.sign(
      aDocumentTooLongForTheUrl().toString("base64"),
      "SHA256withRSA",
      "CAdES",
      "mode=explicit",
      (signature) => settling({ event: "success", result: String(signature) }),
      (type, message) => settling({ event: "error", type: String(type), message: String(message) }),
    );
  };
}

/** Una firma por servidor intermedio resuelta cuando contesta, o al rendirse la página. */
function aSignature(data, algorithm, format) {
  return new Promise((resolve) => {
    AutoScript.sign(
      Buffer.from(data).toString("base64"),
      algorithm,
      format,
      "",
      (signature) => resolve({ signature: String(signature) }),
      (type, message) => resolve({ type: String(type), message: String(message) }),
    );
  });
}

/** Dos rechazos: uno de la firma ya empezada, que se sube, y uno de parámetros, que no. */
async function theRefusalsScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.storage, server.retrieve);
  const afterTheStart = await aSignature("rfirma", "SHA256withRSA", "NoSuchFormat");
  emit(
    aConditionEvent(
      A_SAF_AFTER_THE_START_TRAVELS_INTACT,
      afterTheStart.message?.startsWith("SAF_06") ?? false,
      afterTheStart.message ?? "la firma con un formato inexistente se completó",
    ),
  );
  const uploadsBefore = theUploadedResults(server).length;
  const beforeTheStart = await aSignature("rfirma", "MD5withRSA", "CAdES");
  const late = theUploadedResults(server).slice(uploadsBefore);
  emit(
    aConditionEvent(
      A_SAF_BEFORE_THE_START_IS_NOT_UPLOADED,
      late.length === 0,
      late.length === 0
        ? `nada subido; la página acabó con ${beforeTheStart.message ?? beforeTheStart.signature}`
        : `se subió ${late.map((entry) => entry.dat).join(", ")}`,
    ),
  );
  settle({ event: "success" });
}

/** Una firma cuyo StorageService está en el bucle local: la aplicación no debe usarlo. */
async function theLocalStorageScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.loopbackStorage, server.retrieve);
  const signed = await aSignature("rfirma", "SHA256withRSA", "CAdES");
  const contacted = server.requests.filter(
    (entry) => entry.listener === "loopback" && entry.op === "put",
  );
  emit(
    aConditionEvent(
      THE_LOCAL_STORAGE_SERVLET_REFUSED,
      contacted.length === 0,
      contacted.length === 0
        ? `la aplicación no subió nada a ${server.loopbackStorage}; la página acabó con ${signed.message ?? "una firma"}`
        : `la aplicación subió ${contacted.length} veces a ${server.loopbackStorage}`,
    ),
  );
  settle(
    signed.signature
      ? { event: "success", result: signed.signature }
      : { event: "error", ...signed },
  );
}

/** Una firma cuyo StorageService contesta con un 500 a cada subida: la aplicación tiene que intentarla. */
async function theRefusedUploadScript() {
  const server = await anIntermediateServer({ refusingUploads: true });
  AutoScript.setServlets(server.storage, server.retrieve);
  const signed = await aSignature("rfirma", "SHA256withRSA", "CAdES");
  const attempts = server.putsTo(THE_STORAGE_PATH).filter((entry) => entry.dat !== THE_WAIT_MARK);
  emit(
    aMeasuredConditionEvent(
      THE_REFUSED_UPLOAD_ATTEMPTED,
      attempts.length > 0 ? true : null,
      attempts.length > 0
        ? `la aplicación intentó subir el resultado ${attempts.length} veces y el StorageService lo rechazó`
        : `la aplicación no intentó subir nada; la página acabó con ${signed.message ?? "una firma"}`,
    ),
  );
  settle(
    signed.signature
      ? { event: "success", result: signed.signature }
      : { event: "error", ...signed },
  );
}

/** El lote remoto de siempre, con la respuesta por servidor intermedio. */
async function theBatchThroughTheServerScript() {
  const server = await anIntermediateServer();
  AutoScript.setServlets(server.storage, server.retrieve);
  return BATCH_SCRIPTS.batch.run();
}

const throughTheServer = (run, conditions = [], { benchOnly = false } = {}) =>
  aPublishedScript(run, { family: "intermediate", modes: ["relay"], conditions, benchOnly });

export const RELAY_SCRIPTS = {
  relay: throughTheServer(theRelayScript, [THE_REQUEST_RETRIEVED_BY_FILEID]),
  relayrefused: throughTheServer(theRelayRefusedScript, [], { benchOnly: true }),
  relayselectcert: throughTheServer(theSelectionThroughTheServerScript, [
    THE_RESULT_UPLOADED_CIPHERED,
    WAIT_ANNOUNCED_EVERY_TEN_SECONDS,
  ]),
  relayselectcertslowly: throughTheServer(theSelectionThroughTheServerScript, [
    THE_RESULT_UPLOADED_CIPHERED,
    WAIT_ANNOUNCED_EVERY_TEN_SECONDS,
  ]),
  relaytwice: throughTheServer(theTwoSelectionsScript, [EACH_OPERATION_LAUNCHED_APART]),
  relayretrievalfailure: throughTheServer(
    aSpoiledRetrievalScript(
      "err-01:= La sede no entrega la petición",
      THE_RETRIEVAL_FAILURE_NOT_UPLOADED,
    ),
    [THE_RETRIEVAL_FAILURE_NOT_UPLOADED],
  ),
  relayundecipherable: throughTheServer(
    aSpoiledRetrievalScript(AN_UNDECIPHERABLE_REQUEST, THE_UNDECIPHERABLE_REQUEST_NOT_UPLOADED),
    [THE_UNDECIPHERABLE_REQUEST_NOT_UPLOADED],
  ),
  relayrefusedupload: throughTheServer(theRefusedUploadScript, [THE_REFUSED_UPLOAD_ATTEMPTED]),
  relayrefusals: throughTheServer(theRefusalsScript, [
    A_SAF_AFTER_THE_START_TRAVELS_INTACT,
    A_SAF_BEFORE_THE_START_IS_NOT_UPLOADED,
  ]),
  relaylocalstorage: throughTheServer(theLocalStorageScript, [THE_LOCAL_STORAGE_SERVLET_REFUSED]),
  relaybatch: throughTheServer(theBatchThroughTheServerScript, BATCH_SCRIPTS.batch.conditions),
};
