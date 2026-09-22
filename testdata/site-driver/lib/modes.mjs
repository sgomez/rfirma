// Los modos de la sede: cómo se prepara el entorno y se parchea el `autoscript.js` antes de correr.

import {
  theIntermediateServerAsXmlHttpRequest,
  theLocalServiceAsXmlHttpRequest,
} from "./browser.mjs";
import {
  forcedToFixedServicePorts,
  forcedToIpv6Loopback,
  forcedToProtocolVersion,
} from "./patches.mjs";

/** El puerto fijo al que habla la versión 3 por websocket, el que la prueba le haya dado. */
export function theThirdProtocolPort() {
  return Number(process.env.RFIRMA_BENCH_PORT ?? "63117");
}

function theServiceBindFailurePorts() {
  return (process.env.RFIRMA_BENCH_SERVICE_PORTS ?? "63131,63132,63133").split(",").map(Number);
}

// Sin `WebSocket` el cliente publicado cae a `afirma://service?…`, y Node lo trae de serie.
function withoutWebSocket() {
  delete globalThis.WebSocket;
  globalThis.XMLHttpRequest = theLocalServiceAsXmlHttpRequest();
}

export const MODES = {
  v4: {},
  v3: { patch: (source) => forcedToProtocolVersion(source, 3, theThirdProtocolPort()) },
  "v4-ipv6": {
    // El certificado del canal solo nombra `IP:127.0.0.1`: sin esto fallaría TLS, no el cliente.
    prepare() {
      process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
    },
    patch: forcedToIpv6Loopback,
  },
  service: { prepare: withoutWebSocket },
  "service-bind-failure": {
    prepare: withoutWebSocket,
    patch: (source) => forcedToFixedServicePorts(source, theServiceBindFailurePorts()),
  },
  relay: {
    prepare() {
      globalThis.XMLHttpRequest = theIntermediateServerAsXmlHttpRequest();
    },
    beforeTheApp() {
      AutoScript.setForceWSMode(true);
    },
  },
};
