// El servidor falso de servlet único de la sede: sirve la petición y entrega sus parámetros.

import { createServer } from "node:http";

import { emit } from "./events.mjs";

async function theBody(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  return new URLSearchParams(Buffer.concat(chunks).toString("utf8"));
}

/** Los parámetros del cuerpo del POST, como `application/x-www-form-urlencoded`; nada de la query. */
export function theFormParameters(_request, form) {
  return form;
}

function theNames(parameters) {
  return [...new Set(parameters.keys())].sort();
}

function withoutParameters(contentType) {
  if (contentType === undefined) return null;
  return contentType.split(";")[0].trim().toLowerCase();
}

function theSchemeAndHost(origin) {
  if (origin === undefined) return null;
  try {
    const url = new URL(origin);
    return `${url.protocol}//${url.hostname}`;
  } catch {
    return origin.trim().toLowerCase();
  }
}

function theScheme(authorization) {
  if (authorization === undefined) return null;
  return authorization.trim().split(/\s+/)[0].toLowerCase();
}

/** La petición que recibió `service`, normalizada: nombres de parámetros sin sus valores. */
export function theRequest(service, request, form) {
  const url = new URL(request.url, "http://127.0.0.2");
  return {
    event: "request",
    service,
    method: request.method,
    path: url.pathname,
    query: theNames(url.searchParams),
    body: theNames(form),
    content_type: withoutParameters(request.headers["content-type"]),
    headers: {
      origin: theSchemeAndHost(request.headers.origin),
      authorization: theScheme(request.headers.authorization),
      accept: request.headers.accept?.trim().toLowerCase() ?? null,
    },
  };
}

/**
 * Un servlet HTTP en un puerto libre de `host`, que entrega a `handling` los parámetros de cada
 * petición; con `service`, cuenta cada petición que recibe como ese servicio remoto. `service`
 * también puede ser una función de la petición, para un servidor que atiende más de un servicio
 * remoto según la ruta.
 */
export function servletServer(
  handling,
  { host = "127.0.0.2", reading = theFormParameters, service = null, telling = emit } = {},
) {
  return new Promise((resolve) => {
    const server = createServer(async (request, response) => {
      const form = await theBody(request);
      const resolved = typeof service === "function" ? service(request) : service;
      if (resolved !== null) telling(theRequest(resolved, request, form));
      await handling(reading(request, form), request, response);
    });
    server.listen(0, host, () => resolve(server));
    server.unref();
  });
}
