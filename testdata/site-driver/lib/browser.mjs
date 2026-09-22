// El navegador mínimo en el que corre el `autoscript.js` publicado bajo Node.

import { request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";

import { emit } from "./events.mjs";

let launches = 0;

/** Cuántas veces ha invocado la página a la aplicación hasta ahora. */
export function theLaunchesSoFar() {
  return launches;
}

/** Monta en `globalThis` lo que el cliente publicado espera de una página. */
export function installTheMinimalBrowser() {
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
      launches += 1;
      emit({ event: "launch", url: String(url) });
    },
  };

  globalThis.window = globalThis;
  globalThis.location = pageLocation;
  globalThis.screen = { width: 1920, height: 1080 };
  globalThis.XMLHttpRequest = undefined;
}

/** La sede publicada de mentira, cuyos servlets atiende el banco en proceso. */
const THE_SITE_ORIGIN = "https://sede.example/";

/**
 * El servidor intermedio como `XMLHttpRequest`: el de la sede de mentira guarda con `op=put` y
 * devuelve con `op=get` en proceso; cualquier otro viaja por HTTP de verdad.
 */
export function theIntermediateServerAsXmlHttpRequest() {
  const stored = new Map();

  return class extends theRealXmlHttpRequest() {
    send(body) {
      if (!String(this.url).startsWith(THE_SITE_ORIGIN)) {
        super.send(body);
        return;
      }
      const query = new URLSearchParams(
        this.method === "POST" ? String(body ?? "") : (String(this.url).split("?")[1] ?? ""),
      );
      let answer = "OK";
      if (query.get("op") === "put") {
        stored.set(query.get("id"), query.get("dat"));
        emit({ event: "stored", id: String(query.get("id")), dat: String(query.get("dat")) });
      } else if (query.get("op") === "get") {
        answer = stored.get(query.get("id")) ?? "ERR-06: no existe el fichero";
      }
      setTimeout(() => this.arrive(200, answer), 0);
    }
  };
}

/** El `XMLHttpRequest` que le falta a Node, por HTTP o HTTPS según la URL. */
function theRealXmlHttpRequest() {
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
      const request = String(this.url).startsWith("https:") ? httpsRequest : httpRequest;
      const attempt = request(
        this.url,
        // El canal del original cierra las líneas con `\n` a secas, que el navegador tolera y Node no.
        { method: this.method, headers: this.requestHeaders, insecureHTTPParser: true },
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

/** El `XMLHttpRequest` con el que el transporte sin WebSocket llega al canal. */
export function theLocalServiceAsXmlHttpRequest() {
  return theRealXmlHttpRequest();
}
