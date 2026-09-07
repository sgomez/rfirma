// Conductor del banco de conformidad con autoscript.js.

import { readFileSync } from "node:fs";
import { runInThisContext } from "node:vm";

const autoscriptPath = process.env.RFIRMA_AUTOSCRIPT;
if (!autoscriptPath) {
  process.stderr.write("falta RFIRMA_AUTOSCRIPT\n");
  process.exit(2);
}
const timeoutMs = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");
const mode = process.env.RFIRMA_BENCH_MODE ?? "v4";
const THE_PORT_OF_THE_THIRD_PROTOCOL = 63117;

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
  source = replacingOrFailing(
    source,
    "var PROTOCOL_VERSION = 4;",
    "var PROTOCOL_VERSION = 3;",
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

AutoScript.cargarAppAfirma();
AutoScript.selectCertificate(
  "",
  (data) => settle({ event: "success", data: String(data) }),
  (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
);
