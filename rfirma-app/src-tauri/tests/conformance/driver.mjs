// Conductor del banco de conformidad con autoscript.js.

import { readFileSync } from "node:fs";
import { createServer } from "node:https";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInThisContext } from "node:vm";

const autoscriptPath = process.env.RFIRMA_AUTOSCRIPT;
if (!autoscriptPath) {
  process.stderr.write("falta RFIRMA_AUTOSCRIPT\n");
  process.exit(2);
}
const timeoutMs = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");
const mode = process.env.RFIRMA_BENCH_MODE ?? "v4";
const script = process.env.RFIRMA_BENCH_SCRIPT ?? "selectcert";
const THE_PORT_OF_THE_THIRD_PROTOCOL = 63117;
const here = dirname(fileURLToPath(import.meta.url));

/** Sustituye `literal` por `replacement`, o revienta si el fuente ya no lo trae. */
function replacingOrFailing(source, literal, replacement) {
  if (!source.includes(literal)) {
    throw new Error(`forcedToTheThirdProtocol: no encuentra el literal a sustituir: ${literal}`);
  }
  return source.replace(literal, replacement);
}

/**
 * El `autoscript.js` publicado nunca manda `v=3` por websocket (siempre habla la 4). Para medir
 * el modo `v3` se fuerza el fuente antes de ejecutarlo: la versión que declara, la URL de
 * arranque sin `ports=` y los puertos con los que conecta, al puerto fijo. El oráculo sigue
 * siendo el cliente publicado; solo se le obliga a hablar como uno de la versión 3.
 */
function forcedToTheThirdProtocol(source) {
  source = replacingOrFailing(source, "var PROTOCOL_VERSION = 4;", "var PROTOCOL_VERSION = 3;");
  source = replacingOrFailing(
    source,
    'var url = "afirma://websocket?ports=" + portsLine\n\t\t\t\t\t+ "&v=" + PROTOCOL_VERSION',
    'var url = "afirma://websocket?v=" + PROTOCOL_VERSION',
  );
  source = replacingOrFailing(
    source,
    "var ports = AfirmaUtils.getRandomPorts(minPort, maxPort);",
    `var ports = [${THE_PORT_OF_THE_THIRD_PROTOCOL}];`,
  );
  return source;
}

/** Una línea de JSON por evento, y nada más, en la salida estándar. */
function emit(event) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

/** El veredicto: se emite una sola vez y el proceso se acaba. */
let settled = false;
function settle(event) {
  if (settled) return;
  settled = true;
  emit(event);
  setImmediate(() => process.exit(0));
}

const toStderr = (...args) => process.stderr.write(`[autoscript] ${args.join(" ")}\n`);
globalThis.console = { ...console, log: toStderr, info: toStderr, debug: toStderr };

const userAgent =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
Object.defineProperty(globalThis, "navigator", {
  configurable: true,
  writable: true,
  value: { userAgent, appVersion: userAgent, language: "es-ES", platform: "Linux x86_64" },
});

const pageLocation = {
  protocol: "https:",
  hostname: "sede.example",
  host: "sede.example",
  port: "",
  href: "https://sede.example/tramite",
  origin: "https://sede.example",
};

/** Un elemento del DOM que acepta todo y no hace nada. */
function anElement(tagName) {
  const element = {
    tagName,
    style: {},
    children: [],
    innerHTML: "",
    outerHTML: "",
    setAttribute() {},
    getAttribute() {
      return null;
    },
    appendChild(child) {
      element.children.push(child);
      return child;
    },
    removeChild(child) {
      return child;
    },
    addEventListener() {},
    removeEventListener() {},
    contains() {
      return false;
    },
    click() {},
    focus() {},
  };
  return element;
}

globalThis.document = {
  readyState: "complete",
  head: anElement("head"),
  body: anElement("body"),
  documentElement: anElement("html"),
  createElement: (tag) => anElement(tag),
  createTextNode: (text) => ({ text }),
  getElementById: () => null,
  getElementsByTagName: () => [],
  querySelector: () => null,
  querySelectorAll: () => [],
  addEventListener() {},
  removeEventListener() {},
  get location() {
    return pageLocation;
  },
  set location(url) {
    emit({ event: "launch", url: String(url) });
  },
};

globalThis.window = globalThis;
globalThis.location = pageLocation;
globalThis.screen = { width: 1920, height: 1080 };
globalThis.XMLHttpRequest = undefined;

/**
 * Sin `WebSocket` en el entorno (`isWebSocketsSupported()`, autoscript.js:197-199), el cliente
 * publicado cae al transporte sin WebSocket (`AppAfirmaJSSocket`) y lanza `afirma://service?…`
 * en vez de `afirma://websocket?…`. Node trae `WebSocket` como global desde la 22, así que hay
 * que quitarlo a propósito para medir este modo.
 */
if (mode === "service") {
  delete globalThis.WebSocket;
}

const rawSource = readFileSync(autoscriptPath, "utf8");
const source = mode === "v3" ? forcedToTheThirdProtocol(rawSource) : rawSource;
runInThisContext(source, { filename: autoscriptPath });

SupportDialog.enableSupportDialog(false);
SupportDialog.enableLoadingDialog(false);
SupportDialog.enableErrorDialog(false);

const timer = setTimeout(() => settle({ event: "timeout" }), timeoutMs);
timer.unref?.();

process.on("uncaughtException", (error) => {
  settle({ event: "error", type: "uncaught", message: String(error?.message) });
});

/** Una selección de certificado del cliente publicado, resuelta cuando conteste el trámite. */
function selecting(step) {
  return new Promise((resolve) => {
    AutoScript.selectCertificate(
      "",
      (data) => {
        emit({ event: "success", step, data: String(data) });
        resolve();
      },
      (type, message) =>
        settle({ event: "error", step, type: String(type), message: String(message) }),
    );
  });
}

/**
 * El canal se cierra detrás de cada respuesta, y el cliente publicado no se entera hasta que
 * procesa el cierre: sin esta espera reutilizaría un socket ya cerrado para la siguiente llamada.
 */
function theChannelClosing() {
  return new Promise((resolve) => setTimeout(resolve, 750));
}

/**
 * Tres selecciones seguidas: dos con el certificado fijado y una tercera tras soltarlo. El cliente
 * publicado no lleva `sticky` en el `extraParams`: lo pone `setStickySignatory`, y `resetsticky`
 * solo sale cuando estaba fijado y se suelta (`autoscript.js`, `setStickySignatory`).
 */
function theStickyScript() {
  AutoScript.setStickySignatory(true);
  return selecting("stuck")
    .then(theChannelClosing)
    .then(() => selecting("stuck-again"))
    .then(theChannelClosing)
    .then(() => {
      AutoScript.setStickySignatory(false);
      return selecting("released");
    })
    .then(() => settle({ event: "done" }));
}

/** El material TLS de los dos servlets: el certificado del servidor local y su clave. */
function theServletMaterial() {
  const certificate = process.env.RFIRMA_BENCH_SERVLET_CERT;
  const key = process.env.RFIRMA_BENCH_SERVLET_KEY;
  if (!certificate || !key) {
    throw new Error("faltan RFIRMA_BENCH_SERVLET_CERT y RFIRMA_BENCH_SERVLET_KEY");
  }
  return { cert: readFileSync(certificate), key: readFileSync(key) };
}

/** Un servlet del lote sirviendo TLS en un puerto libre del loopback, y su URL absoluta. */
function servletServing(answering) {
  return new Promise((resolve) => {
    const server = createServer(theServletMaterial(), (request, response) => {
      const { status, body } = answering(new URL(request.url, "https://127.0.0.1").searchParams);
      response.writeHead(status, { "content-type": "application/json" });
      response.end(body);
    });
    server.listen(0, "127.0.0.1", () =>
      resolve(`https://127.0.0.1:${server.address().port}/batch`),
    );
  });
}

function decodedFromBase64(value) {
  return Buffer.from(value, "base64url").toString("utf8");
}

function theFrozen(fixture) {
  return readFileSync(join(here, fixture), "utf8").trim();
}

/** Lo que ambos servlets exigen del original: el lote en `json` y la cadena en `certs`. */
function missingBatchFields(query) {
  if (!query.get("json")) return "json";
  if (!query.get("certs")) return "certs";
  return null;
}

/**
 * El presigner: comprueba el lote y los `certs`, y devuelve el `TriphaseData` congelado
 * (`BatchSigner`/`afirma-server-triphase-signer`, 1.9.2).
 */
function thePresigner(query) {
  const missing = missingBatchFields(query);
  if (missing) {
    emit({ event: "presign", missing });
    return { status: 400, body: `falta '${missing}'` };
  }

  const lote = JSON.parse(decodedFromBase64(query.get("json")));
  emit({
    event: "presign",
    signs: String(lote.singlesigns.length),
    certs: String(query.get("certs").split(";").length),
    algorithm: String(lote.algorithm),
  });
  return { status: 200, body: theFrozen("batch-presign-response.json") };
}

/**
 * El postsigner: exige `tridata` con `PK1` en cada firma, y devuelve el resultado congelado del
 * lote.
 */
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

  emit({
    event: "postsign",
    signs: String(signs.length),
    pre: String(signs.filter((sign) => !!sign.params.PRE).length),
  });
  return { status: 200, body: theFrozen("batch-postsign-result.json") };
}

/** Un lote de dos documentos firmado con `signBatchJSON` contra los dos servlets del banco. */
async function theBatchScript() {
  const presigner = await servletServing(thePresigner);
  const postsigner = await servletServing(thePostsigner);

  AutoScript.createBatch("SHA256", "CAdES", "sign");
  AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
  AutoScript.addDocumentToBatch("dos", Buffer.from("segundo documento").toString("base64"));
  AutoScript.signBatchProcess(
    true,
    presigner,
    postsigner,
    null,
    (result, certificate) =>
      settle({
        event: "success",
        result: Buffer.from(JSON.stringify(result), "utf8").toString("base64"),
        certificate: String(certificate),
      }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

AutoScript.cargarAppAfirma();

if (script === "batch") {
  theBatchScript();
} else if (script === "sticky") {
  theStickyScript();
} else {
  AutoScript.selectCertificate(
    "",
    (data) => settle({ event: "success", data: String(data) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}
