// Los guiones de ficheros de la sede publicada: guardar, cargar y firmar y guardar.

import { theCmsSignature, theShapeOf } from "../lib/cms.mjs";
import { aConditionEvent, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import { theChallenge, theReferenceSignature } from "../lib/fixtures.mjs";
import { aPublishedScript, withoutAChoice } from "../lib/script.mjs";
import { THE_SIGNATURE_VERIFIES, theSignatureVerifies } from "../lib/verification.mjs";

const THE_NAME_NEXT_TO_THE_CONTENT = "the-name-next-to-the-content";
const EVERY_FILE_APART = "every-file-apart";
const THE_FILENAME_IN_A_THIRD_COMPONENT = "the-filename-in-a-third-component";
const THE_PICKED_SIGNATURE_COSIGNED = "the-picked-signature-cosigned";
/** La firma del banco de referencia que el arnés de la suite deja en disco como `firma.csig`. */
const THE_SIGNATURE_TO_PICK = "cades-implicit.p7s";
/** La mide el arnés de la suite, que lee el fichero guardado en el perfil aislado. */
const THE_DECODED_BYTES_ON_DISK = "the-decoded-bytes-on-disk";
/** La mide el arnés de la suite, que relee la firma guardada donde la propone la petición. */
const THE_RETURNED_SIGNATURE_ON_DISK = "the-returned-signature-on-disk";

/** Emite si la firma del reto verifica y cierra el trámite con ella. */
function settlingTheVerifiedSignature(signature, certificate) {
  for (const condition of theSignatureVerifies("cms", theChallenge)(signature, certificate)) {
    emit({ event: "condition", ...condition });
  }
  settle({ event: "success", result: String(signature), certificate: String(certificate) });
}

const THE_SAVING_EXTENSION = "csig";
const THE_SAVING_DESCRIPTION = "Firma de la sede";
const THE_SAVING_DIRECTORY = "/tmp";

/** Un `signAndSaveToFile()` sin identificador de operación: el verbo (`cop`) no viaja. */
function theSignAndSaveWithoutAVerbScript() {
  AutoScript.signAndSaveToFile(
    null,
    theChallenge().toString("base64"),
    "SHA256withRSA",
    "CAdES",
    withoutAChoice(),
    "challenge.csig",
    (data) => settle({ event: "success", data: String(data) }),
    settlingTheError,
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
    settlingTheError,
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
    settlingTheError,
  );
}

/** Un `saveDataToFile()` sin datos: la petición viaja sin `dat`. */
function theSaveWithoutDataScript() {
  AutoScript.saveDataToFile(
    null,
    "Guarda nada",
    "challenge.bin",
    "bin",
    "Datos binarios",
    (data) => settle({ event: "success", data: String(data) }),
    settlingTheError,
  );
}

/** Un `saveDataToFile()` cuyas extensiones traen un `;`, que `exts` no admite. */
function theSaveWithIllegalExtensionsScript() {
  AutoScript.saveDataToFile(
    theChallenge().toString("base64"),
    "Guarda el reto del banco de referencia",
    "challenge.bin",
    "b;in",
    "Datos binarios",
    (data) => settle({ event: "success", data: String(data) }),
    settlingTheError,
  );
}

/** Una carga, simple o múltiple, sin medir nada: sólo importa cómo termina al cancelarla. */
function theLoadToCancelScript(multiple) {
  const load = multiple
    ? AutoScript.getMultiFileNameContentBase64
    : AutoScript.getFileNameContentBase64;
  load(
    multiple ? "Carga varios documentos" : "Carga un documento",
    "bin",
    "Datos binarios",
    null,
    (filenames, data) =>
      settle({ event: "success", filenames: String(filenames), data: String(data) }),
    settlingTheError,
  );
}

/** Un `sign()` sin datos: la petición viaja sin `dat` y el documento se pide en disco. */
function theSignWithoutDataScript() {
  AutoScript.sign(
    "",
    "SHA256withRSA",
    "CAdES",
    "mode=implicit",
    (signature, certificate, extraInfo) => {
      emit(theFilenameInAThirdComponent(extraInfo));
      settle({ event: "success", result: String(signature), certificate: String(certificate) });
    },
    settlingTheError,
  );
}

function theFilenameInAThirdComponent(extraInfo) {
  let filename = null;
  try {
    filename = extraInfo ? JSON.parse(String(extraInfo)).filename : null;
  } catch {
    filename = null;
  }
  const carried = typeof filename === "string" && filename.length > 0;
  return aConditionEvent(
    THE_FILENAME_IN_A_THIRD_COMPONENT,
    carried,
    carried
      ? `la respuesta trajo un tercer componente con el nombre «${filename}»`
      : `la respuesta no trajo el nombre del fichero elegido en un tercer componente (${extraInfo === null ? "no hubo" : `llegó ${String(extraInfo).slice(0, 40)}`})`,
  );
}

/** Una cofirma sin datos: la petición viaja sin `dat` y la firma que se cofirma se pide en disco. */
function theCosignWithoutDataScript() {
  AutoScript.coSign(
    "",
    null,
    "SHA256withRSA",
    "CAdES",
    withoutAChoice(),
    (signature, certificate) => {
      emit(thePickedSignatureCosigned(signature));
      settle({ event: "success", result: String(signature), certificate: String(certificate) });
    },
    settlingTheError,
  );
}

function thePickedSignatureCosigned(signature) {
  const cms = theCmsSignature(bytesOf(signature));
  const [picked] = theCmsSignature(theReferenceSignature(THE_SIGNATURE_TO_PICK)).signers;
  const kept = cms?.signers.some((signer) => signer.signature.equals(picked.signature)) ?? false;
  return aConditionEvent(
    THE_PICKED_SIGNATURE_COSIGNED,
    kept && theShapeOf(cms) === "[][]",
    !cms
      ? "lo que volvió no es un CMS SignedData"
      : `firmantes con la forma ${theShapeOf(cms)}; ${kept ? "uno es" : "ninguno es"} el de la firma preparada en disco`,
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
          THE_NAME_NEXT_TO_THE_CONTENT,
          apart,
          apart
            ? "el nombre llegó separado del contenido"
            : "la respuesta no trajo el nombre junto al contenido",
        ),
      );
      settle({ event: "success", filename: String(filename), data: String(data) });
    },
    settlingTheError,
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
          EVERY_FILE_APART,
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
    settlingTheError,
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
    settlingTheVerifiedSignature,
    settlingTheError,
  );
}

/** Un `signAndSaveToFile()` que declara las extensiones, la descripción y el directorio del guardado. */
function theSignAndSaveWithSavingParametersScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    theChallenge().toString("base64"),
    "SHA256",
    "CAdES",
    [
      "mode=explicit",
      `filenameSaveExts=${THE_SAVING_EXTENSION}`,
      `filenameSaveDescription=${THE_SAVING_DESCRIPTION}`,
      `filenameSaveCurrentDir=${THE_SAVING_DIRECTORY}`,
    ].join("\n"),
    "challenge-signed.csig",
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
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
    settlingTheError,
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
    settlingTheError,
  );
}

/** Un `signAndSaveToFile()` con un algoritmo de curva elíptica (BUG-05). */
function theSignAndSaveWithAnEcdsaAlgorithmScript() {
  AutoScript.signAndSaveToFile(
    "sign",
    theChallenge().toString("base64"),
    "SHA256withECDSA",
    "CAdES",
    "mode=explicit",
    "challenge-signed.csig",
    settlingTheVerifiedSignature,
    settlingTheError,
  );
}

export const FILE_SCRIPTS = {
  save: aPublishedScript(theSaveScript),
  savewithanillegalfilename: aPublishedScript(theSaveWithAnIllegalFilenameScript),
  load: aPublishedScript(theLoadScript, { conditions: [THE_NAME_NEXT_TO_THE_CONTENT] }),
  multiload: aPublishedScript(theMultiLoadScript, { conditions: [EVERY_FILE_APART] }),
  signandsave: aPublishedScript(theSignAndSaveScript, {
    conditions: [THE_SIGNATURE_VERIFIES, THE_RETURNED_SIGNATURE_ON_DISK],
  }),
  signandsavecancelled: aPublishedScript(theSignAndSaveScript, {
    conditions: [THE_SIGNATURE_VERIFIES],
  }),
  signandsavewithoutaverb: aPublishedScript(theSignAndSaveWithoutAVerbScript),
  signandsavewithoutdata: aPublishedScript(theSignAndSaveWithoutDataScript),
  signandsavewithanillegalfilename: aPublishedScript(theSignAndSaveWithAnIllegalFilenameScript),
  signandsavewithsavingparameters: aPublishedScript(theSignAndSaveWithSavingParametersScript, {
    conditions: [THE_RETURNED_SIGNATURE_ON_DISK],
  }),
  signandsavewithecdsa: aPublishedScript(theSignAndSaveWithAnEcdsaAlgorithmScript, {
    conditions: [THE_SIGNATURE_VERIFIES],
  }),
  savecancelled: aPublishedScript(theSaveScript),
  savewithoutdata: aPublishedScript(theSaveWithoutDataScript),
  savewithillegalextensions: aPublishedScript(theSaveWithIllegalExtensionsScript),
  savereadback: aPublishedScript(theSaveScript, { conditions: [THE_DECODED_BYTES_ON_DISK] }),
  loadcancelled: aPublishedScript(() => theLoadToCancelScript(false)),
  multiloadcancelled: aPublishedScript(() => theLoadToCancelScript(true)),
  signwithoutdata: aPublishedScript(theSignWithoutDataScript, {
    conditions: [THE_FILENAME_IN_A_THIRD_COMPONENT],
  }),
  cosignwithoutdata: aPublishedScript(theCosignWithoutDataScript, {
    conditions: [THE_PICKED_SIGNATURE_COSIGNED],
  }),
};
