// El servidor falso de servlet único de la sede: sirve la petición y entrega sus parámetros.

import { createServer } from "node:http";

/** Los parámetros del cuerpo del POST, como `application/x-www-form-urlencoded`. */
export async function theFormParameters(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  return new URLSearchParams(Buffer.concat(chunks).toString("utf8"));
}

/** Los parámetros de la query y los del cuerpo del POST, donde `UrlHttpManagerImpl` los manda. */
export async function theServletParameters(request) {
  const parameters = new URL(request.url, "http://127.0.0.2").searchParams;
  for (const [name, value] of await theFormParameters(request)) {
    parameters.append(name, value);
  }
  return parameters;
}

/** Un servlet HTTP en un puerto libre de `host`, que entrega a `handling` los parámetros de cada petición. */
export function servletServer(handling, { host = "127.0.0.2", reading = theServletParameters } = {}) {
  return new Promise((resolve) => {
    const server = createServer(async (request, response) => {
      await handling(await reading(request), request, response);
    });
    server.listen(0, host, () => resolve(server));
  });
}
