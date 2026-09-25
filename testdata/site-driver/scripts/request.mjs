// Los guiones de petición de la sede publicada: lo que el cliente hace con el `dat` que recibe.

import { createServer } from "node:http";

import { theCmsSignature } from "../lib/cms.mjs";
import { aConditionEvent, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import { aPublishedScript, withoutAChoice } from "../lib/script.mjs";

const THE_URL_IN_DAT_DOWNLOADED_AND_SIGNED = "the-url-in-dat-downloaded-and-signed";
const THE_LITERAL_DAT_SIGNED = "the-literal-dat-signed";

const THE_SERVED_DOCUMENT = Buffer.from("documento que sirve la sede por HTTP", "utf8");

/** Ni está en el alfabeto Base64 ni su longitud es múltiplo de cuatro. */
const THE_LITERAL_DAT = "dato*literal!";

/** Un documento servido por HTTP en un puerto libre del loopback, su URL y si alguien lo pidió. */
function aDocumentServed(content) {
  const served = { requested: false, url: null };
  return new Promise((resolve) => {
    const server = createServer((_request, response) => {
      served.requested = true;
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(content);
    });
    server.listen(0, "127.0.0.2", () => {
      served.url = `http://127.0.0.2:${server.address().port}/documento.bin`;
      resolve(served);
    });
  });
}

function theImplicitContentOf(signature) {
  return theCmsSignature(bytesOf(signature))?.content ?? null;
}

/** Un `sign()` en CAdES implícito, que mide con `measuring` la firma recibida. */
function aSignOf(dat, measuring, failing = settlingTheError) {
  AutoScript.sign(
    dat,
    "SHA256withRSA",
    "CAdES",
    withoutAChoice("mode=implicit"),
    (signature, certificate) => {
      emit(measuring(theImplicitContentOf(String(signature))));
      settle({ event: "success", result: String(signature), certificate: String(certificate) });
    },
    failing,
  );
}

/** Si falla después de descargar la URL, lo que falló no es la descarga y no se mide nada. */
async function theSignOfAUrlInDatScript() {
  const served = await aDocumentServed(THE_SERVED_DOCUMENT);
  aSignOf(
    served.url,
    (content) => {
      const signed = content?.equals(THE_SERVED_DOCUMENT) ?? false;
      return aConditionEvent(
        THE_URL_IN_DAT_DOWNLOADED_AND_SIGNED,
        served.requested && signed,
        !served.requested
          ? "nadie pidió la URL que viajaba en dat"
          : signed
            ? "se descargó la URL de dat y la firma contiene lo que sirvió"
            : "se descargó la URL de dat, pero la firma no contiene lo que sirvió",
      );
    },
    (type, message) => {
      if (!served.requested) {
        emit(
          aConditionEvent(
            THE_URL_IN_DAT_DOWNLOADED_AND_SIGNED,
            false,
            `nadie pidió la URL que viajaba en dat y el cliente contestó ${type}: ${message}`,
          ),
        );
      }
      settlingTheError(type, message);
    },
  );
}

function theSignOfALiteralDatScript() {
  const literal = Buffer.from(THE_LITERAL_DAT, "utf8");
  aSignOf(THE_LITERAL_DAT, (content) => {
    const signed = content?.equals(literal) ?? false;
    return aConditionEvent(
      THE_LITERAL_DAT_SIGNED,
      signed,
      signed
        ? `la firma contiene los bytes de «${THE_LITERAL_DAT}» tal cual`
        : `la firma no contiene los bytes de «${THE_LITERAL_DAT}» tal cual`,
    );
  });
}

export const REQUEST_SCRIPTS = {
  signdaturl: aPublishedScript(theSignOfAUrlInDatScript, {
    modes: ["service"],
    conditions: [THE_URL_IN_DAT_DOWNLOADED_AND_SIGNED],
  }),
  signliteraldat: aPublishedScript(theSignOfALiteralDatScript, {
    conditions: [THE_LITERAL_DAT_SIGNED],
  }),
};
