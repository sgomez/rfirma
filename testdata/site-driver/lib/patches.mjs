// Los parches que se aplican al fuente del `autoscript.js` publicado antes de ejecutarlo.

/** Sustituye `literal` por `replacement`, o revienta si el fuente ya no lo trae. */
function replacingOrFailing(source, literal, replacement) {
  if (!source.includes(literal)) {
    throw new Error(`el parche no encuentra el literal a sustituir: ${literal}`);
  }
  return source.replace(literal, replacement);
}

/** Fuerza el `autoscript.js` a hablar la versión dada por websocket, sin `ports=` y al puerto fijo. */
export function forcedToProtocolVersion(source, version, port) {
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
    `var ports = [${port}];`,
  );
  return source;
}

/** Fuerza el `autoscript.js` a hablar con el bucle local IPv6. */
export function forcedToIpv6Loopback(source) {
  return replacingOrFailing(source, 'var SERVER_HOST = "127.0.0.1";', 'var SERVER_HOST = "[::1]";');
}

/** Fija los tres puertos candidatos del transporte sin WebSocket, que el original sortea. */
export function forcedToFixedServicePorts(source, ports) {
  return replacingOrFailing(
    source,
    "// Calculamos los puertos\n\t\t\t\t\tvar ports = AfirmaUtils.getRandomPorts(minPort, maxPort);",
    `// Calculamos los puertos\n\t\t\t\t\tvar ports = [${ports.join(", ")}];`,
  );
}

/** Añade `gzip=true` a la URL de la operación, que el `autoscript.js` publicado nunca pone. */
export function withTheDataDeclaredGzipped(source) {
  return replacingOrFailing(
    source,
    "\t\t\t/** Construye una URL que configura la operacion a realizar. */\n\t\t\tfunction buildUrl (paramsObject) {",
    "\t\t\tfunction buildUrl (paramsObject) {\n" +
      '\t\t\t\treturn buildUrlWithoutGzip(paramsObject) + "&gzip=true";\n' +
      "\t\t\t}\n\t\t\tfunction buildUrlWithoutGzip (paramsObject) {",
  );
}

/** Quita `needcert=true` de la URL del lote, que el `autoscript.js` publicado siempre pone. */
export function withoutNeedcertInTheBatch(source) {
  source = replacingOrFailing(source, 'data.needcert = createKeyValuePair ("needcert", true);', "");
  return replacingOrFailing(
    source,
    'data.needcert = generateDataKeyValue ("needcert",  true);',
    "",
  );
}

/** Escribe `jsonBatch` en la URL del lote, en vez del `jsonbatch` en minúsculas del publicado. */
export function withJsonbatchCapitalised(source) {
  source = replacingOrFailing(
    source,
    'createKeyValuePair ("jsonbatch", true);',
    'createKeyValuePair ("jsonBatch", true);',
  );
  return replacingOrFailing(
    source,
    'generateDataKeyValue ("jsonbatch", true);',
    'generateDataKeyValue ("jsonBatch", true);',
  );
}

/** Manda `localBatchProcess=true` también con un lote XML, que el publicado sólo marca en JSON. */
export function withLocalBatchProcessOnAnXmlBatch(source) {
  const theBatchData =
    'data.dat = createKeyValuePair ("dat",  batchB64 == "" ? null : batchB64, true);';
  return replacingOrFailing(
    source,
    theBatchData,
    `${theBatchData}\n\t\t\t\tif (localBatchProcess) { data.localBatchProcess = createKeyValuePair ("localBatchProcess", true); }`,
  );
}

/** Deja que `setStickySignatory(false, true)` suelte la fijación sin mandar `resetsticky`. */
export function withAReleaseWithoutReset(source) {
  return replacingOrFailing(
    source,
    "var setStickySignatory = function(sticky) {",
    "var setStickySignatory = function(sticky, withoutReset) {\n" +
      "\t\t\tif (withoutReset) { stickySignatory = sticky; return; }",
  );
}
