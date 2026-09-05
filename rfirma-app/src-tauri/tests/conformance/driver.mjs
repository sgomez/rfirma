// El conductor del **banco de conformidad**: mete el `autoscript.js` publicado
// —el mismo fichero que sirve una sede de verdad, del tag `v1.9.2` de
// `clienteafirma`— dentro de Node y le deja hablar con el canal de rfirma
// (TD-55, TD-59).
//
// NO es el cliente de canal: ese es el propio, en Rust, y vive en
// `tests/channel_client.rs`. Aquel provoca lo que un cliente conforme no puede
// provocar; este comprueba que el cliente REAL entiende lo que contestamos.
//
// El sujeto de la prueba es `autoscript.js`, así que aquí no se reimplementa
// nada suyo: se le monta alrededor el mínimo navegador que necesita para
// arrancar —`window`, `document`, `navigator` y `WebSocket`— y se le deja
// correr entero. Lo que este fichero SÍ hace es interceptar los dos únicos
// puntos por los que el navegador de verdad hablaría con el escritorio:
//
//   1. `openUrl`, que en Chrome asigna `document.location = "afirma://…"`.
//      Ahí es donde arrancaría rfirma; aquí se publica la URL por la salida
//      estándar para que la prueba de Rust ate el canal en uno de los puertos
//      que el cliente acaba de sortear (ID-215).
//   2. Los dos callbacks de la operación, `successCallback` y `errorCallback`,
//      que son el veredicto: lo que la sede recibiría de verdad.
//
// Contrato con la prueba de Rust: una línea de JSON por evento en la salida
// estándar, y todo lo demás —incluido el `console.log` del propio
// `autoscript.js`— a la de error.
//
//   {"event":"launch","url":"afirma://websocket?ports=…&v=4&jvc=3&idsession=…"}
//   {"event":"error","type":"<clase Java>","message":"<mensaje>"}
//   {"event":"success","data":"<respuesta>"}
//   {"event":"timeout"}
//
// El proceso termina con 0 en cuanto emite uno de los tres últimos: quien
// juzga es la prueba de Rust, no este fichero.

import { readFileSync } from "node:fs";
import { runInThisContext } from "node:vm";

const autoscriptPath = process.env.RFIRMA_AUTOSCRIPT;
if (!autoscriptPath) {
  process.stderr.write("falta RFIRMA_AUTOSCRIPT\n");
  process.exit(2);
}
const timeoutMs = Number(process.env.RFIRMA_BENCH_TIMEOUT_MS ?? "45000");

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
  // Un `setImmediate` para que la escritura salga antes de morir, y `exit`
  // porque `autoscript.js` deja temporizadores de reintento vivos.
  setImmediate(() => process.exit(0));
}

// ---------------------------------------------------------------------------
// El navegador mínimo
// ---------------------------------------------------------------------------

// `autoscript.js` escribe mucho por consola; su ruido va a la salida de error
// para no ensuciar el canal de eventos.
const toStderr = (...args) => process.stderr.write(`[autoscript] ${args.join(" ")}\n`);
globalThis.console = { ...console, log: toStderr, info: toStderr, debug: toStderr };

// Chrome, y a propósito: es la única rama de `openUrl` que no monta un iframe,
// así que la invocación se reduce a una asignación que se puede interceptar.
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
  // Sin `all`: es la marca de Internet Explorer y su polyfill de `setTimeout`
  // es lo primero que hace el fichero.
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
  // AQUÍ SE INTERCEPTA LA INVOCACIÓN: en Chrome, `openUrl` asigna la URL
  // `afirma://` a `document.location`. Eso es lo que dispararía el escritorio.
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

// ---------------------------------------------------------------------------
// El sujeto de la prueba, entero y sin tocar
// ---------------------------------------------------------------------------

const source = readFileSync(autoscriptPath, "utf8");
runInThisContext(source, { filename: autoscriptPath });

// Los diálogos de soporte son DOM y ninguno de los tres aporta nada aquí. Con
// ellos apagados, el propio `autoscript.js` llama directamente al
// `errorCallback` en vez de enseñar un botón de reintento — que es justo el
// camino que la sede vería con `SupportDialog` desactivado.
SupportDialog.enableSupportDialog(false);
SupportDialog.enableLoadingDialog(false);
SupportDialog.enableErrorDialog(false);

const timer = setTimeout(() => settle({ event: "timeout" }), timeoutMs);
timer.unref?.();

// Una excepción que se escape de un manejador de eventos del WebSocket no
// puede quedarse en silencio: sería indistinguible de un cuelgue.
process.on("uncaughtException", (error) => {
  settle({ event: "error", type: "uncaught", message: String(error?.message) });
});

AutoScript.cargarAppAfirma();
AutoScript.selectCertificate(
  "",
  (data) => settle({ event: "success", data: String(data) }),
  (type, message) => settle({ event: "error", type: String(type), message: String(message) }),
);
