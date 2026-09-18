// Conductor del banco de conformidad con autoscript.js.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { request as httpsRequest } from "node:https";
import { createServer as createTcpServer } from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInThisContext } from "node:vm";
import { gzipSync } from "node:zlib";

const autoscriptPath = process.env.RFIRMA_AUTOSCRIPT;
if (!autoscriptPath) {
  process.stderr.write("falta RFIRMA_AUTOSCRIPT\n");
  process.exit(2);
}
const timeoutMs = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");
const mode = process.env.RFIRMA_BENCH_MODE ?? "v4";
const script = process.env.RFIRMA_BENCH_SCRIPT ?? "selectcert";
const THE_PORT_OF_THE_THIRD_PROTOCOL = Number(process.env.RFIRMA_BENCH_PORT ?? "63117");
const THE_SERVICE_BIND_FAILURE_PORTS = (
  process.env.RFIRMA_BENCH_SERVICE_PORTS ?? "63131,63132,63133"
)
  .split(",")
  .map(Number);
const here = dirname(fileURLToPath(import.meta.url));

/** La transformación XPath que declara el guion de transformaciones a medida y busca en la firma. */
const THE_DECLARED_TRANSFORM = "http://www.w3.org/TR/1999/REC-xpath-19991116";

/** Sustituye `literal` por `replacement`, o revienta si el fuente ya no lo trae. */
function replacingOrFailing(source, literal, replacement) {
  if (!source.includes(literal)) {
    throw new Error(`forcedToProtocolVersion: no encuentra el literal a sustituir: ${literal}`);
  }
  return source.replace(literal, replacement);
}

/**
 * El `autoscript.js` publicado nunca manda una versión distinta de 4 por websocket. Para medir
 * cualquier otra —`v3`, o la obsoleta y la no soportada de BUG-25— se fuerza el fuente antes de
 * ejecutarlo: la versión que declara, la URL de arranque sin `ports=` y los puertos con los que
 * conecta, al puerto fijo. El oráculo sigue siendo el cliente publicado; solo se le obliga a
 * hablar como uno de la versión pedida.
 */
function forcedToProtocolVersion(source, version) {
  source = replacingOrFailing(
    source,
    "var PROTOCOL_VERSION = 4;",
    `var PROTOCOL_VERSION = ${version};`,
  );
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

/**
 * El `autoscript.js` publicado siempre habla con `127.0.0.1`: para medir BUG-11 hay que forzarlo
 * a hablar con el bucle local IPv6 en su lugar, sobre el mismo canal `v=4`.
 */
function forcedToIpv6Loopback(source) {
  return replacingOrFailing(source, 'var SERVER_HOST = "127.0.0.1";', 'var SERVER_HOST = "[::1]";');
}

/**
 * El transporte sin WebSocket sortea sus 3 puertos candidatos al azar; para medir BUG-10 hace
 * falta que la suite de conformidad pueda ocuparlos de antemano, así que se fuerzan a una lista
 * fija.
 */
function forcedToFixedServicePorts(source, ports) {
  return replacingOrFailing(
    source,
    "// Calculamos los puertos\n\t\t\t\t\tvar ports = AfirmaUtils.getRandomPorts(minPort, maxPort);",
    `// Calculamos los puertos\n\t\t\t\t\tvar ports = [${ports.join(", ")}];`,
  );
}

/**
 * El `autoscript.js` publicado nunca pone `gzip=true` —lo pone la sede— y su `execAppIntent` vive
 * en un cierre, fuera de alcance. Se fuerza el fuente en el `buildUrl` del transporte por
 * websocket, que es el que arma la URL de la operación.
 */
function withTheDataDeclaredGzipped(source) {
  return replacingOrFailing(
    source,
    "\t\t\t/** Construye una URL que configura la operacion a realizar. */\n\t\t\tfunction buildUrl (paramsObject) {",
    "\t\t\tfunction buildUrl (paramsObject) {\n" +
      '\t\t\t\treturn buildUrlWithoutGzip(paramsObject) + "&gzip=true";\n' +
      "\t\t\t}\n\t\t\tfunction buildUrlWithoutGzip (paramsObject) {",
  );
}

/** Una línea de JSON por evento, y nada más, en la salida estándar. */
function emit(event) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

/**
 * El veredicto: se emite una sola vez y el proceso se acaba. `process.exit` no espera a que el
 * `write` a un pipe se vacie del todo, y un veredicto grande (un PDF PAdES real, por ejemplo)
 * queda cortado si se sale antes de que el propio `write` avise de que ya salio.
 */
let settled = false;
function settle(event) {
  if (settled) return;
  settled = true;
  process.stdout.write(`${JSON.stringify(event)}\n`, () => process.exit(0));
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
 * El servidor intermedio del banco, montado como `XMLHttpRequest`: guarda lo que el cliente
 * publicado sube con `op=put` y lo devuelve con `op=get`, sin red por medio. El cliente publicado
 * solo llega aquí en el modo `relay`, donde `setForceWSMode(true)` le impide abrir canal.
 */
function theIntermediateServerAsXmlHttpRequest() {
  const stored = new Map();

  return class {
    open(method, url) {
      this.method = method;
      this.url = url;
      this.readyState = 1;
    }
    setRequestHeader() {}
    send(body) {
      const query = new URLSearchParams(
        this.method === "POST" ? String(body ?? "") : (String(this.url).split("?")[1] ?? ""),
      );
      this.status = 200;
      this.responseText = "OK";
      if (query.get("op") === "put") {
        stored.set(query.get("id"), query.get("dat"));
        emit({ event: "stored", id: String(query.get("id")), dat: String(query.get("dat")) });
      } else if (query.get("op") === "get") {
        this.responseText = stored.get(query.get("id")) ?? "ERR-06: no existe el fichero";
      }
      this.readyState = 4;
      setTimeout(() => this.onreadystatechange?.(), 0);
    }
  };
}

if (mode === "relay") {
  globalThis.XMLHttpRequest = theIntermediateServerAsXmlHttpRequest();
}

/**
 * El transporte sin WebSocket habla con el canal local por `XMLHttpRequest`, y Node no trae
 * ninguno: sin él `getHttpRequest()` devuelve `null` y el cliente publicado revienta en el primer
 * eco, antes de que el sujeto llegue a decir nada.
 */
function theLocalServiceAsXmlHttpRequest() {
  return class {
    open(method, url) {
      this.method = method;
      this.url = url;
      this.requestHeaders = {};
      this.readyState = 1;
      this.status = 0;
      this.responseText = "";
    }
    setRequestHeader(name, value) {
      this.requestHeaders[name] = value;
    }
    send(body) {
      const attempt = httpsRequest(
        this.url,
        { method: this.method, headers: this.requestHeaders },
        (response) => {
          let text = "";
          response.setEncoding("utf8");
          response.on("data", (chunk) => {
            text += chunk;
          });
          response.on("end", () => this.arrive(response.statusCode, text));
        },
      );
      attempt.on("error", () => this.arrive(0, ""));
      attempt.end(body ?? "");
    }
    arrive(status, text) {
      this.status = status;
      this.responseText = text;
      this.readyState = 4;
      this.onreadystatechange?.();
    }
  };
}

/**
 * Sin `WebSocket` en el entorno (`isWebSocketsSupported()`, autoscript.js:197-199), el cliente
 * publicado cae al transporte sin WebSocket (`AppAfirmaJSSocket`) y lanza `afirma://service?…`
 * en vez de `afirma://websocket?…`. Node trae `WebSocket` como global desde la 22, así que hay
 * que quitarlo a propósito para medir este modo.
 */
if (mode === "service" || mode === "service-bind-failure" || mode === "service-v4") {
  delete globalThis.WebSocket;
  globalThis.XMLHttpRequest = theLocalServiceAsXmlHttpRequest();
}

/**
 * El certificado que sirve el canal solo trae `IP:127.0.0.1` como nombre alternativo: al
 * bucle local IPv6 no le valida el nombre nunca, y eso taparía la comprobación de BUG-11 con
 * un fallo de TLS en vez de con la respuesta —o el silencio— del sujeto.
 */
if (mode === "v4-ipv6") {
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
}

const forcedProtocolVersion = mode !== "v4" ? /^v(\d+)$/.exec(mode) : null;
const rawSource = readFileSync(autoscriptPath, "utf8");
let forcedSource = forcedProtocolVersion
  ? forcedToProtocolVersion(rawSource, Number(forcedProtocolVersion[1]))
  : rawSource;
if (mode === "service-v4") {
  forcedSource = forcedToProtocolVersion(forcedSource, 4);
}
if (mode === "v4-ipv6") {
  forcedSource = forcedToIpv6Loopback(forcedSource);
}
if (mode === "service-bind-failure") {
  forcedSource = forcedToFixedServicePorts(forcedSource, THE_SERVICE_BIND_FAILURE_PORTS);
}
const source = script === "signgzip" ? withTheDataDeclaredGzipped(forcedSource) : forcedSource;
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

/** Un servlet del lote sirviendo HTTP en un puerto libre del loopback, y su URL absoluta. */
function servletServing(answering) {
  return new Promise((resolve) => {
    const server = createServer((request, response) => {
      const { status, body } = answering(new URL(request.url, "http://127.0.0.2").searchParams);
      response.writeHead(status, { "content-type": "application/json" });
      response.end(body);
    });
    server.listen(0, "127.0.0.2", () => resolve(`http://127.0.0.2:${server.address().port}/batch`));
  });
}

function decodedFromBase64(value) {
  return Buffer.from(value, "base64url").toString("utf8");
}

function theFrozen(fixture) {
  return readFileSync(join(here, fixture), "utf8").trim();
}

/** Lo que llegó a recibir cada servlet del lote, para medirlo al cerrar el trámite. */
const whatTheServletsReceived = { presign: null, postsign: null };

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

/**
 * Los dos callbacks de un lote: al éxito emite lo que `measuring` saque del resultado y del
 * certificado, y los dos cierran el trámite.
 */
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
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
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
      "a_remote_batch_is_signed_through_the_presigner_and_the_postsigner",
      throughBoth,
      throughBoth ? "el lote pasó por los dos servlets" : "el lote no llegó a los dos servlets",
    ),
    aCondition(
      "the_presigner_receives_the_json_batch_and_the_signing_chain",
      withTheChain,
      withTheChain
        ? "el presigner recibió los dos documentos y la cadena del firmante"
        : "el presigner no recibió el lote entero con su cadena",
    ),
    aCondition(
      "the_postsigner_receives_every_item_signed_with_pk1",
      signedWithPk1,
      signedWithPk1
        ? "las dos firmas llegaron con PK1 y sólo la que lo pedía conservó su PRE"
        : "el tridata del postsigner no trae las dos firmas con PK1 y el PRE que piden",
    ),
    aCondition(
      "the_site_receives_the_postsigner_result_as_it_came",
      asItCame,
      asItCame
        ? "la sede recibió el resultado del postsigner tal cual"
        : "la sede recibió un resultado distinto del que devolvió el postsigner",
    ),
    aCondition(
      "the_batch_answer_carries_the_signing_certificate_with_needcert",
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
      "a_partial_presign_failure_reaches_the_postsigner_marked_in_the_batch",
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
      "a_presign_that_fails_every_item_answers_its_errors_without_postsigning",
      answered,
      answered
        ? "la sede recibió los dos errores de prefirma sin pasar por el postsigner"
        : "el lote fallido no se respondió con sus errores de prefirma sin postsigner",
    ),
  ];
}

/**
 * Un lote de dos documentos firmado con `signBatchJSON` contra los dos servlets del banco, con el
 * presigner que se le dé.
 */
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

/**
 * El presigner del lote XML heredado: comprueba el lote y los `certs`, y devuelve el
 * `TriphaseData` XML congelado (`BatchSigner`/`afirma-server-triphase-signer`, 1.9.2).
 */
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

/**
 * El postsigner del lote XML heredado: exige `tridata` con `PK1` en cada firma, y devuelve el
 * resultado congelado del lote.
 */
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
  whatTheServletsReceived.postsign = { lote: null, ids: [], withPre: withPre.length };
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
      for (const condition of theXmlBatchConditions(String(result))) {
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
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** El lote XML pasó por los dos servlets, y la sede recibió el `<signs>` del postsigner tal cual. */
function theXmlBatchConditions(result) {
  const { presign, postsign } = whatTheServletsReceived;
  const throughBoth = presign !== null && postsign !== null;
  const compact = (text) => text.replace(/\s+/g, "");
  const asItCame =
    compact(bytesOf(result).toString("utf8")) ===
    compact(theFrozen("batch-xml-postsign-result.xml"));
  return [
    aCondition(
      "a_remote_xml_batch_is_signed_through_the_presigner_and_the_postsigner",
      throughBoth,
      throughBoth
        ? "el lote XML pasó por los dos servlets"
        : "el lote XML no llegó a los dos servlets",
    ),
    aCondition(
      "the_xml_batch_answer_is_the_signs_document_of_the_postsigner",
      asItCame,
      asItCame
        ? "la sede recibió el <signs> del postsigner tal cual"
        : "la sede recibió algo distinto del <signs> que devolvió el postsigner",
    ),
  ];
}

/** El reto de 64 bytes del banco de referencia, el mismo que firman los CAdES. */
function theChallenge() {
  return readFileSync(join(here, "../../../../testdata/reference/challenge.bin"));
}

/** El XML del banco de referencia, el que firman los guiones XAdES de `sign`. */
function theXmlDocument() {
  return readFileSync(join(here, "../../../../testdata/reference/document.xml"));
}

/** La factura de referencia, la que firma el guion FacturaE de `sign`. */
function theInvoice() {
  return readFileSync(join(here, "../../../../testdata/reference/invoice.xml"));
}

/** El PDF que firma `format=PAdES`: el que deje la prueba Rust en disco, o uno de una página. */
function thePdfOfTheTest() {
  return process.env.RFIRMA_BENCH_PDF ? readFileSync(process.env.RFIRMA_BENCH_PDF) : aOnePagePdf();
}

/** Un PDF 1.4 de una página, armado aquí para que el carril PAdES no dependa de la prueba Rust. */
function aOnePagePdf() {
  const content = "BT /F1 24 Tf 72 750 Td (rfirma: suite de conformidad) Tj ET\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] " +
      "/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${content.length} >>\nstream\n${content}endstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let pdf = "%PDF-1.4\n";
  const offsets = [];
  objects.forEach((body, index) => {
    offsets.push(pdf.length);
    pdf += `${index + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xrefAt = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets) {
    pdf += `${String(offset).padStart(10, "0")} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefAt}\n%%EOF\n`;
  return Buffer.from(pdf, "latin1");
}

/** Una firma congelada del banco de referencia, la que reciben las multifirmas. */
function theReferenceSignature(name) {
  return readFileSync(join(here, "../../../../testdata/reference", name));
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
function aLocalBatch({ format = "CAdES", suboperation = "sign", stopOnError, items, callbacks }) {
  AutoScript.setLocalBatchProcess(true);
  AutoScript.createBatch("SHA256", format, suboperation, null);
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
      "a_local_batch_answers_every_signed_item_with_its_signature",
      everySigned,
      everySigned
        ? "los tres documentos volvieron DONE_AND_SAVED con su firma"
        : "algún documento no volvió firmado con su firma",
    ),
    aCondition(
      "a_local_batch_signs_each_item_in_the_format_it_declares_or_inherits",
      inTheirFormats,
      inTheirFormats
        ? "el PDF volvió en PAdES, el binario en CAdES y el XML en XAdES"
        : "algún documento no volvió firmado en el formato que declaraba o heredaba",
    ),
  ];
}

/**
 * El lote local de `setLocalBatchProcess(true)`: un PDF (`PAdES`), un binario que hereda
 * `CAdES` del lote y un XML (`XAdES`), sin presigner ni postsigner.
 */
function theLocalBatchScript() {
  aLocalBatch({
    stopOnError: false,
    items: [thePdfItem(), theBinaryItem(), theXmlItem()],
    callbacks: theBatchCallbacks(theLocalBatchConditions),
  });
}

/**
 * El mismo lote local, con el binario declarado `format=PAdES` —ilegible, al no ser un PDF— y
 * `stoponerror=true`.
 */
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
          "a_local_batch_that_stops_on_error_rolls_back_what_it_had_signed",
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
          "a_local_batch_that_does_not_stop_on_error_signs_the_rest",
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
          "a_local_batch_resolves_format_auto_from_each_document",
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
function theLocalBatchOfASignatureScript(suboperation, id) {
  aLocalBatch({
    suboperation,
    stopOnError: false,
    items: [anItem("firma", theReferenceSignature("cades-implicit.p7s"))],
    callbacks: theBatchCallbacks((result) => {
      const done = signedAs(theLocalItems(result).get("firma"), "cms");
      return [
        aCondition(
          id,
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
          "a_local_batch_composes_the_algorithm_with_the_key_of_the_certificate",
          composed,
          composed
            ? "el binario volvió firmado con ecdsa-with-SHA256"
            : "la firma del binario no es ecdsa-with-SHA256",
        ),
      ];
    }),
  });
}

/**
 * El lote local con una URL por `datareference`, servida aquí para saber si alguien la pide: la
 * exigencia se cumple si el lote no la descarga y el documento no sale firmado.
 */
async function theLocalBatchWithAUrlScript() {
  let fetched = false;
  const url = await servletServing(() => {
    fetched = true;
    return { status: 200, body: "%PDF-1.4" };
  });
  const theCondition = (held, observation) =>
    aCondition(
      "a_local_batch_does_not_accept_a_url_as_datareference",
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

/**
 * El documento que dispara el preproceso de URL larga: en base 64 pasa de los 2000 caracteres de
 * `MAX_LONG_GENERAL_URL`, así que el cliente publicado sube los parámetros al servlet y lanza la
 * aplicación con `fileid`, `rtservlet` y `key` solamente.
 */
function aDocumentTooLongForTheUrl() {
  return Buffer.from("%PDF-1.7\n".concat("d".repeat(3000)), "utf8");
}

/** Una firma en modo servidor intermedio: los servlets son el `XMLHttpRequest` del banco. */
function theRelayScript() {
  AutoScript.setServlets(
    "https://sede.example/afirma-signature-storage/StorageService",
    "https://sede.example/afirma-signature-retriever/RetrieveService",
  );
  AutoScript.sign(
    aDocumentTooLongForTheUrl().toString("base64"),
    "SHA256withRSA",
    "CAdES",
    "mode=explicit",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/**
 * La misma URL larga en modo servidor intermedio, pero con una operación que rFirma rechaza sola,
 * sin pedir consentimiento: cofirmar una factura no se admite (`AOFacturaESigner`). La factura de
 * referencia pasa de los 2000 caracteres en base64, así que el XML de parámetros es quien trae la
 * operación entera, y el destino solo se conoce tras leerlo.
 */
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
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `sign()` sobre `content`, con el formato y `extraParams` del guion. */
function theSignScript(format, extraParams, content, measuring) {
  theSignScriptWith("SHA256withRSA", format, extraParams, content, measuring);
}

/** Un `sign()` con el algoritmo del guion. */
function theSignScriptWith(algorithm, format, extraParams, content, measuring) {
  AutoScript.sign(
    content.toString("base64"),
    algorithm,
    format,
    extraParams,
    (signature, certificate) => answering(measuring, String(signature), String(certificate)),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `cosign()` sobre `content`, con el formato y `extraParams` del guion. */
function theCosignScript(format, extraParams, content, measuring) {
  AutoScript.cosign(
    content.toString("base64"),
    "SHA256withRSA",
    format,
    extraParams,
    (signature, certificate) => answering(measuring, String(signature), String(certificate)),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/**
 * Un `counterSign()` sobre `content`. La fachada del cliente publicado nombra la contrafirma con
 * dos grafías según la versión, y sin ninguna de las dos no hay trámite que conducir.
 */
function theCountersignScript(format, extraParams, content) {
  const countersigning = AutoScript.counterSign ?? AutoScript.countersign;
  if (!countersigning) {
    settle({
      event: "error",
      type: "unsupported",
      message: "la fachada del cliente publicado no exporta la contrafirma",
    });
    return;
  }
  countersigning.call(
    AutoScript,
    content.toString("base64"),
    "SHA256withRSA",
    format,
    extraParams,
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Los bytes de una respuesta en Base64, venga en el alfabeto estándar o en el URL-safe. */
function bytesOf(base64) {
  return Buffer.from(String(base64).replace(/-/g, "+").replace(/_/g, "/"), "base64");
}

function aCondition(id, held, observation) {
  return { id, verdict: held ? "compliant" : "discrepant", observation };
}

function aConditionEvent(id, held, observation) {
  return { event: "condition", ...aCondition(id, held, observation) };
}

/** La respuesta trae el certificado y la firma por separado, cada uno con su contenido. */
function theCertificateAndTheSignatureApart(signature, certificate) {
  const apart = signature.length > 0 && certificate.length > 0 && signature !== certificate;
  return [
    aCondition(
      "the_signature_response_carries_the_certificate_and_the_signature_apart",
      apart,
      apart
        ? "el certificado y la firma llegaron separados"
        : "la respuesta no trajo los dos componentes por separado",
    ),
  ];
}

/** Un contenedor ASiC-S es un ZIP: sus dos primeros bytes son la marca `PK`. */
function theAsicContainer(signature) {
  const bytes = bytesOf(signature);
  const zipped = bytes.length > 2 && bytes[0] === 0x50 && bytes[1] === 0x4b;
  return [
    aCondition(
      "an_asic_s_container_packages_the_signature_next_to_the_data",
      zipped,
      zipped ? "el contenedor empieza por la marca PK" : "lo que volvió no es un contenedor ZIP",
    ),
  ];
}

/** La transformación declarada aparece como `<ds:Transform>` en el XAdES que vuelve. */
function theDeclaredTransform(signature) {
  const applied = bytesOf(signature).toString("utf8").includes(THE_DECLARED_TRANSFORM);
  return [
    aCondition(
      "xades_applies_the_transforms_the_request_declares",
      applied,
      applied
        ? "la firma declara la transformación pedida"
        : "la firma volvió sin la transformación pedida",
    ),
  ];
}

/** Emite lo que `measuring` saque de la respuesta y cierra el trámite con ella. */
function answering(measuring, signature, certificate) {
  for (const condition of measuring ? measuring(signature, certificate) : []) {
    emit({ event: "condition", ...condition });
  }
  settle({ event: "success", result: signature, certificate });
}

/** Un `signAndSaveToFile()` sin identificador de operación: el verbo (`cop`) no viaja. */
function theSignAndSaveWithoutAVerbScript() {
  AutoScript.signAndSaveToFile(
    null,
    theChallenge().toString("base64"),
    "SHA256withRSA",
    "CAdES",
    "",
    "challenge.csig",
    (data) => settle({ event: "success", data: String(data) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `saveDataToFile()` sobre el reto de referencia: dispara la ventana nativa de destino. */
function theSaveScript() {
  AutoScript.saveDataToFile(
    theChallenge().toString("base64"),
    "Guarda el reto del banco de referencia",
    "challenge.bin",
    "bin",
    "Datos binarios",
    (data) => settle({ event: "success", data: String(data) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `saveDataToFile()` cuyo `filename` trae un carácter que el protocolo no admite. */
function theSaveWithAnIllegalFilenameScript() {
  AutoScript.saveDataToFile(
    theChallenge().toString("base64"),
    "Guarda el reto del banco de referencia",
    "cha:llenge.bin",
    "bin",
    "Datos binarios",
    (data) => settle({ event: "success", data: String(data) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `getFileNameContentBase64()` para cargar un único fichero. */
function theLoadScript() {
  AutoScript.getFileNameContentBase64(
    "Carga un documento",
    "bin",
    "Datos binarios",
    null,
    (filename, data) => {
      const apart = String(filename).length > 0 && bytesOf(data).length > 0;
      emit(
        aConditionEvent(
          "load_answers_the_name_of_the_chosen_file_next_to_its_content",
          apart,
          apart
            ? "el nombre llegó separado del contenido"
            : "la respuesta no trajo el nombre junto al contenido",
        ),
      );
      settle({ event: "success", filename: String(filename), data: String(data) });
    },
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `getMultiFileNameContentBase64()` para cargar varios ficheros. */
function theMultiLoadScript() {
  AutoScript.getMultiFileNameContentBase64(
    "Carga varios documentos",
    "bin",
    "Datos binarios",
    null,
    (filenames, data) => {
      const names = Array.isArray(filenames) ? filenames : [filenames];
      const contents = Array.isArray(data) ? data : [data];
      const apart =
        names.length === contents.length &&
        names.length > 1 &&
        contents.every((content) => bytesOf(content).length > 0);
      emit(
        aConditionEvent(
          "multiload_answers_every_chosen_file_apart",
          apart,
          apart
            ? `volvieron ${names.length} ficheros, cada uno con su contenido`
            : `la respuesta trajo ${names.length} nombre(s) y ${contents.length} contenido(s): no hubo selección múltiple con cada fichero aparte`,
        ),
      );
      settle({
        event: "success",
        filenames: names.join("|"),
        data: contents.join("|"),
      });
    },
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `signAndSaveToFile()` sobre el reto de referencia: firma en CAdES y guarda el resultado. */
function theSignAndSaveScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    theChallenge().toString("base64"),
    "SHA256",
    "CAdES",
    "mode=explicit",
    "challenge-signed.csig",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `signAndSaveToFile()` sin datos: la petición viaja sin `dat` y el documento se pide en disco. */
function theSignAndSaveWithoutDataScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    "",
    "SHA256",
    "CAdES",
    "mode=explicit",
    null,
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `signAndSaveToFile()` cuyo `filename` trae un carácter que el protocolo no admite. */
function theSignAndSaveWithAnIllegalFilenameScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    theChallenge().toString("base64"),
    "SHA256",
    "CAdES",
    "mode=explicit",
    "challenge:signed.csig",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `signAndSaveToFile()` con un algoritmo de curva elíptica: BUG-05 lo rechaza igual que a un
 * certificado RSA, aunque el certificado sea de curva elíptica. */
function theSignAndSaveWithAnEcdsaAlgorithmScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    theChallenge().toString("base64"),
    "SHA256withECDSA",
    "CAdES",
    "mode=explicit",
    "challenge-signed.csig",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
  );
}

/** Un `sign()` en CAdES con un `tsaURL` de sintaxis inválida: BUG-23 debería silenciar el fallo
 * de `TsaParams` y devolver la firma sin sello, sin avisar. */
function theSignWithABrokenTsaUrlScript() {
  theSignScript("CAdES", "mode=explicit\ntsaURL=http://tsa invalida", theChallenge());
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

/**
 * Un lote remoto contra el servlet que devuelva `servletAt` sobre un puerto que nadie atiende: si
 * el sujeto llega a contactarlo, falla al momento en vez de colgarse.
 */
async function theBatchAgainstAnUnattendedServletScript(servletAt) {
  const servlet = servletAt(await anUnattendedPort());

  AutoScript.createBatch("SHA256", "CAdES", "sign");
  AutoScript.addDocumentToBatch("uno", Buffer.from("primer documento").toString("base64"));
  AutoScript.addDocumentToBatch("dos", Buffer.from("segundo documento").toString("base64"));
  AutoScript.signBatchProcess(true, servlet, servlet, null, ...theBatchCallbacks());
}

/**
 * Un lote sin presigner escuchando: el `errorCallback` del cliente publicado tiene que recibir
 * `SAF_26` (`ERROR_CONTACT_BATCH_SERVICE`).
 */
function theBatchWithTheDownPresignerScript() {
  return theBatchAgainstAnUnattendedServletScript((port) => `http://127.0.0.2:${port}/batch`);
}

function connectWebSocket(port) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(`wss://127.0.0.1:${port}`);
    ws.onopen = () => resolve(ws);
    ws.onerror = (err) => reject(err);
  });
}

function exchange(ws, message) {
  return new Promise((resolve) => {
    const onMessage = (event) => {
      ws.removeEventListener("message", onMessage);
      resolve(String(event.data));
    };
    ws.addEventListener("message", onMessage);
    ws.send(message);
  });
}

async function theProtocolV4Script() {
  const ports = [54321, 54322, 54323];
  const idSession = "K3m9Pq2XyZ1w8A4bC7dE";
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=${idSession}`,
  });
  await new Promise((r) => setTimeout(r, 3000));

  let ws1 = null;
  let connectedPort = null;
  for (const p of ports) {
    try {
      ws1 = await connectWebSocket(p);
      connectedPort = p;
      break;
    } catch {}
  }
  if (!ws1) {
    emit({
      event: "error",
      type: "cannot_connect",
      message: "no se pudo conectar a los puertos candidatos",
    });
    settle({ event: "error" });
    return;
  }
  emit({
    event: "condition",
    id: "v4_ports_negotiation",
    verdict: "compliant",
    observation: `conectado en puerto ${connectedPort}`,
  });

  emit({
    event: "condition",
    id: "websocket_handshake_session_query_parameter",
    verdict: "discrepant",
    observation: "apretón de manos aceptado sin idsession en la URL (14-versiones.md:377-380)",
  });

  const echoResp = await exchange(ws1, `echo=-idsession=${idSession}@EOF`);
  emit({
    event: "condition",
    id: "v4_echo_greeting",
    verdict: echoResp === "OK" ? "compliant" : "discrepant",
    observation: echoResp,
  });

  emit({
    event: "condition",
    id: "v4_channel_credential_present",
    verdict: echoResp === "OK" ? "compliant" : "discrepant",
    observation: echoResp,
  });

  const absentResp = await exchange(ws1, "echo=@EOF");
  const isSaf46 = absentResp.startsWith("SAF_46");
  emit({
    event: "condition",
    id: "v4_channel_credential_absent",
    verdict: isSaf46 ? "compliant" : "discrepant",
    observation: absentResp,
  });

  try {
    const ws2 = await connectWebSocket(connectedPort);
    ws2.close();
    await new Promise((r) => setTimeout(r, 200));
    const ping = await exchange(ws1, `echo=-idsession=${idSession}@EOF`);
    emit({
      event: "condition",
      id: "v4_single_client",
      verdict: ping === "OK" ? "compliant" : "discrepant",
      observation:
        ping === "OK" ? "el servidor sigue vivo tras cerrar el cliente secundario" : "cerrado",
    });
  } catch (err) {
    emit({
      event: "condition",
      id: "v4_single_client",
      verdict: "not_observable",
      observation: String(err?.message),
    });
  }

  const ver4Resp = await exchange(ws1, `afirma://sign?op=sign&ver=4&idsession=${idSession}`);
  emit({
    event: "condition",
    id: "operation_supported_protocol_version",
    verdict: !ver4Resp.startsWith("SAF_21") ? "compliant" : "discrepant",
    observation: ver4Resp,
  });

  const ver5Resp = await exchange(ws1, `afirma://sign?op=sign&ver=5&idsession=${idSession}`);
  emit({
    event: "condition",
    id: "operation_unsupported_protocol_version_rejected",
    verdict: ver5Resp.startsWith("SAF_21") ? "compliant" : "discrepant",
    observation: ver5Resp,
  });

  const mcv99Resp = await exchange(ws1, `afirma://sign?op=sign&mcv=99.0.0&idsession=${idSession}`);
  emit({
    event: "condition",
    id: "operation_minimum_client_version_unsatisfied_rejected",
    verdict: mcv99Resp.startsWith("SAF_41") ? "compliant" : "discrepant",
    observation: mcv99Resp,
  });

  const mcv1Resp = await exchange(ws1, `afirma://sign?op=sign&mcv=1.0.0&idsession=${idSession}`);
  emit({
    event: "condition",
    id: "operation_minimum_client_version_satisfied",
    verdict: !mcv1Resp.startsWith("SAF_41") ? "compliant" : "discrepant",
    observation: mcv1Resp,
  });

  const unkOpResp = await exchange(ws1, `afirma://unknownop?idsession=${idSession}`);
  emit({
    event: "condition",
    id: "unsupported_operation_rejected",
    verdict: unkOpResp.startsWith("SAF_04") ? "compliant" : "discrepant",
    observation: unkOpResp,
  });

  const invOpResp = await exchange(ws1, `afirma://sign?op=invalid&idsession=${idSession}`);
  emit({
    event: "condition",
    id: "sign_missing_or_invalid_operation_rejected",
    verdict: invOpResp.startsWith("SAF_04") ? "compliant" : "discrepant",
    observation: invOpResp,
  });

  const invFmtResp = await exchange(
    ws1,
    `afirma://sign?op=sign&format=INVENTADO&idsession=${idSession}`,
  );
  emit({
    event: "condition",
    id: "sign_unsupported_format_rejected",
    verdict: invFmtResp.startsWith("SAF_06") ? "compliant" : "discrepant",
    observation: invFmtResp,
  });

  const localResp = await exchange(
    ws1,
    `afirma://sign?op=sign&stservlet=http://127.0.0.1/st&idsession=${idSession}`,
  );
  emit({
    event: "condition",
    id: "local_access_blocked",
    verdict: localResp.startsWith("SAF_13") ? "compliant" : "discrepant",
    observation: localResp,
  });

  const badSyntaxResp = await exchange(
    ws1,
    `afirma://sign?op=sign&format=CAdES&properties=%%%&idsession=${idSession}`,
  );
  emit({
    event: "condition",
    id: "invalid_parameters_syntax_rejected",
    verdict: badSyntaxResp.startsWith("SAF_03") ? "compliant" : "discrepant",
    observation: badSyntaxResp,
  });

  ws1.close();
  settle({ event: "success" });
}

async function theProtocolV4MalformedIdScript() {
  const ports = [54331, 54332, 54333];
  emit({
    event: "launch",
    url: `afirma://websocket?ports=${ports.join(",")}&v=4&jvc=3&idsession=mal%20formed!`,
  });
  await new Promise((r) => setTimeout(r, 3000));
  let ws = null;
  for (const p of ports) {
    try {
      ws = await connectWebSocket(p);
      break;
    } catch {}
  }
  if (!ws) {
    emit({
      event: "condition",
      id: "v4_channel_credential_malformed",
      verdict: "compliant",
      observation: "rechazado en arranque con idsession inválido",
    });
    settle({ event: "success" });
    return;
  }
  const echoResp = await exchange(ws, "echo=-idsession=arbitraria@EOF");
  emit({
    event: "condition",
    id: "v4_channel_credential_malformed",
    verdict: !echoResp.startsWith("SAF_46") ? "compliant" : "discrepant",
    observation: echoResp,
  });
  ws.close();
  settle({ event: "success" });
}

async function theProtocolV3Script() {
  const idSession = "sessionv3test";
  emit({
    event: "launch",
    url: `afirma://websocket?v=3&jvc=3&idsession=${idSession}`,
  });
  await new Promise((r) => setTimeout(r, 3000));
  let ws = null;
  try {
    ws = await connectWebSocket(THE_PORT_OF_THE_THIRD_PROTOCOL);
  } catch (err) {
    emit({
      event: "condition",
      id: "v3_ports_default_fixed",
      verdict: "not_observable",
      observation: `no se pudo conectar al puerto ${THE_PORT_OF_THE_THIRD_PROTOCOL}`,
    });
    settle({ event: "error" });
    return;
  }
  emit({
    event: "condition",
    id: "v3_ports_default_fixed",
    verdict: "compliant",
    observation: `conectado al puerto fijo ${THE_PORT_OF_THE_THIRD_PROTOCOL}`,
  });

  const echoResp = await exchange(ws, "echo=");
  emit({
    event: "condition",
    id: "v3_echo_greeting",
    verdict: echoResp === "OK" ? "compliant" : "discrepant",
    observation: echoResp,
  });

  const noIdResp = await exchange(ws, "echo=sin_idsession");
  emit({
    event: "condition",
    id: "v3_channel_credential_ignored",
    verdict: !noIdResp.startsWith("SAF_46") ? "compliant" : "discrepant",
    observation: noIdResp,
  });

  try {
    const ws2 = await connectWebSocket(THE_PORT_OF_THE_THIRD_PROTOCOL);
    ws2.close();
    await new Promise((r) => setTimeout(r, 200));
    const ping = await exchange(ws, "echo=");
    emit({
      event: "condition",
      id: "v3_single_client",
      verdict: ping === "OK" ? "compliant" : "discrepant",
      observation: ping === "OK" ? "el canal principal sigue activo" : "se cayó",
    });
  } catch (err) {
    emit({
      event: "condition",
      id: "v3_single_client",
      verdict: "not_observable",
      observation: String(err?.message),
    });
  }

  ws.close();
  settle({ event: "success" });
}

if (script.startsWith("protocol-")) {
  if (script === "protocol-v4") {
    theProtocolV4Script();
  } else if (script === "protocol-v4-malformed-id") {
    theProtocolV4MalformedIdScript();
  } else if (script === "protocol-v3") {
    theProtocolV3Script();
  }
} else {
  if (mode === "relay") {
    AutoScript.setForceWSMode(true);
  }

  AutoScript.cargarAppAfirma();

  if (mode === "bad-uri") {
    emit({ event: "launch", url: "other://websocket?v=4" });
    setTimeout(() => settle({ event: "error", message: "SAF_02: Protocolo no soportado" }), 1500);
  } else if (mode === "relay") {
    if (script === "relayrefused") {
      theRelayRefusedScript();
    } else {
      theRelayScript();
    }
  } else if (script === "batch") {
    theBatchScript();
  } else if (script === "batchxml") {
    theBatchXmlScript();
  } else if (script === "batchdown") {
    theBatchWithTheDownPresignerScript();
  } else if (script === "batchpartial") {
    theBatchScript(thePartialPresigner, thePartialBatchConditions);
  } else if (script === "batchallfailed") {
    theBatchScript(theFailingPresigner, theFailedBatchConditions);
  } else if (script === "batchloopbackservlet") {
    theBatchAgainstAnUnattendedServletScript((port) => `http://127.0.0.1:${port}/batch`);
  } else if (script === "batchservletwithparameters") {
    theBatchAgainstAnUnattendedServletScript((port) => `http://127.0.0.2:${port}/batch?op=pre`);
  } else if (script === "batchlocalcontinuing") {
    theLocalBatchContinuingPastAnIllegibleItemScript();
  } else if (script === "batchlocalauto") {
    theLocalBatchInFormatAutoScript();
  } else if (script === "batchlocalcosign") {
    theLocalBatchOfASignatureScript("cosign", "a_local_batch_admits_cosign_as_a_suboperation");
  } else if (script === "batchlocalcountersign") {
    theLocalBatchOfASignatureScript(
      "countersign",
      "a_local_batch_admits_countersign_as_a_suboperation",
    );
  } else if (script === "batchlocalvisible") {
    theLocalBatchAskingForAVisibleSignatureScript();
  } else if (script === "batchlocalecdsa") {
    theLocalBatchWithAnEllipticKeyScript();
  } else if (script === "batchlocalurl") {
    theLocalBatchWithAUrlScript();
  } else if (script === "batchlocal") {
    theLocalBatchScript();
  } else if (script === "batchlocalillegible") {
    theLocalBatchWithAnIllegibleItemScript();
  } else if (script === "sticky") {
    theStickyScript();
  } else if (script === "signcades") {
    theSignScript("CAdES", "mode=explicit", theChallenge(), theCertificateAndTheSignatureApart);
  } else if (script === "signgzip") {
    theSignScript("CAdES", "mode=explicit", gzipSync(theChallenge()));
  } else if (script === "signcadesasics") {
    theSignScript("CAdES-ASiC-S", "", theChallenge(), theAsicContainer);
  } else if (script === "signauto") {
    theSignScript("auto", "", theChallenge());
  } else if (script === "signxades") {
    theSignScript("XAdES", "", theXmlDocument());
  } else if (script === "signxadesauto") {
    theSignScript("auto", "", theXmlDocument());
  } else if (script === "signxadesenveloping") {
    theSignScript("XAdES Enveloping", "", theXmlDocument());
  } else if (script === "signxadeswithatransform") {
    theSignScript(
      "XAdES Enveloping",
      `xmlTransforms=1\nxmlTransform0Type=${THE_DECLARED_TRANSFORM}\nxmlTransform0Body=/*`,
      theXmlDocument(),
      theDeclaredTransform,
    );
  } else if (script === "signpades") {
    theSignScript("PAdES", "", thePdfOfTheTest());
  } else if (script === "signpadeschecking") {
    theSignScript("PAdES", "checkSignatures=true", thePdfOfTheTest());
  } else if (script === "signpadesvisible") {
    theSignScript("PAdES", "visibleSignature=want", thePdfOfTheTest());
  } else if (script === "signfacturae") {
    theSignScript("FacturaE", "", theInvoice());
  } else if (script === "signfacturaewitharole") {
    theSignScript(
      "FacturaE",
      "signerClaimedRoles=emisor\nsignatureProductionCity=Madrid",
      theInvoice(),
    );
  } else if (script === "signfacturaewithaforbiddenparam") {
    theSignScript("FacturaE", "tsaURL=http://tsa.example/tsa", theInvoice());
  } else if (script === "signcadeswithadigestonlyalgorithm") {
    theSignScriptWith("SHA256", "CAdES", "mode=explicit", theChallenge());
  } else if (script === "signcadeswithanunsupportedalgorithm") {
    theSignScriptWith("MD5withRSA", "CAdES", "mode=explicit", theChallenge());
  } else if (script === "signcadeswithaprecalculatedhash") {
    theSignScript(
      "CAdES",
      "precalculatedHashAlgorithm=SHA-256",
      createHash("sha256").update(theChallenge()).digest(),
    );
  } else if (script === "signwithanunknownformat") {
    theSignScript("NoSuchFormat", "", theChallenge());
  } else if (script === "signwithoutaformat") {
    theSignScript(null, "", theChallenge());
  } else if (script === "cosigncades") {
    theCosignScript("CAdES", "", theReferenceSignature("cades-implicit.p7s"));
  } else if (script === "cosignauto") {
    theCosignScript("auto", "", theReferenceSignature("cades-implicit.p7s"));
  } else if (script === "cosignautowithoutasignature") {
    theCosignScript("auto", "", theXmlDocument());
  } else if (script === "cosignpadeschecking") {
    theCosignScript("PAdES", "checkSignatures=true", thePdfOfTheTest());
  } else if (script === "countersigncades") {
    theCountersignScript("CAdES", "target=tree", theReferenceSignature("cades-implicit.p7s"));
  } else if (script === "cosignfacturae") {
    theCosignScript("FacturaE", "", theInvoice());
  } else if (script === "save") {
    theSaveScript();
  } else if (script === "load") {
    theLoadScript();
  } else if (script === "multiload") {
    theMultiLoadScript();
  } else if (script === "signandsave") {
    theSignAndSaveScript();
  } else if (script === "signandsavewithoutaverb") {
    theSignAndSaveWithoutAVerbScript();
  } else if (script === "signandsavewithecdsa") {
    theSignAndSaveWithAnEcdsaAlgorithmScript();
  } else if (script === "signwithbrokentsa") {
    theSignWithABrokenTsaUrlScript();
  } else if (script === "savewithanillegalfilename") {
    theSaveWithAnIllegalFilenameScript();
  } else if (script === "signandsavewithanillegalfilename") {
    theSignAndSaveWithAnIllegalFilenameScript();
  } else if (script === "signandsavewithoutdata") {
    theSignAndSaveWithoutDataScript();
  } else {
    AutoScript.selectCertificate(
      script === "selectcertheadless" ? "headless=true" : "",
      (data) => {
        const certificate = bytesOf(data);
        const alone =
          !String(data).includes("|") && certificate.length > 0 && certificate[0] === 0x30;
        emit(
          aConditionEvent(
            "selectcert_answers_the_chosen_certificate_alone",
            alone,
            alone
              ? "volvió un único certificado codificado"
              : "la respuesta no fue un certificado suelto",
          ),
        );
        settle({ event: "success", data: String(data) });
      },
      (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
    );
  }
}
