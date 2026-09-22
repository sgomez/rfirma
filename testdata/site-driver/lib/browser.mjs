// El navegador mínimo en el que corre el `autoscript.js` publicado bajo Node.

import { request as httpsRequest } from "node:https";

import { emit } from "./events.mjs";

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
      emit({ event: "launch", url: String(url) });
    },
  };

  globalThis.window = globalThis;
  globalThis.location = pageLocation;
  globalThis.screen = { width: 1920, height: 1080 };
  globalThis.XMLHttpRequest = undefined;
}

/** El servidor intermedio como `XMLHttpRequest`: guarda con `op=put` y devuelve con `op=get`. */
export function theIntermediateServerAsXmlHttpRequest() {
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

/** El `XMLHttpRequest` que le falta a Node para que el transporte sin WebSocket llegue al canal. */
export function theLocalServiceAsXmlHttpRequest() {
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
