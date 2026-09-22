// Los guiones de ficheros de la sede publicada: guardar, cargar y firmar y guardar.

import { aConditionEvent, bytesOf, emit, settle, settlingTheError } from "../lib/events.mjs";
import { theChallenge } from "../lib/fixtures.mjs";
import { aPublishedScript } from "../lib/script.mjs";

const THE_NAME_NEXT_TO_THE_CONTENT = "the-name-next-to-the-content";
const EVERY_FILE_APART = "every-file-apart";

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
    "",
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
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
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
    (signature, certificate) =>
      settle({ event: "success", result: String(signature), certificate: String(certificate) }),
    settlingTheError,
  );
}

export const FILE_SCRIPTS = {
  save: aPublishedScript(theSaveScript),
  savewithanillegalfilename: aPublishedScript(theSaveWithAnIllegalFilenameScript),
  load: aPublishedScript(theLoadScript, { conditions: [THE_NAME_NEXT_TO_THE_CONTENT] }),
  multiload: aPublishedScript(theMultiLoadScript, { conditions: [EVERY_FILE_APART] }),
  signandsave: aPublishedScript(theSignAndSaveScript),
  signandsavecancelled: aPublishedScript(theSignAndSaveScript),
  signandsavewithoutaverb: aPublishedScript(theSignAndSaveWithoutAVerbScript),
  signandsavewithoutdata: aPublishedScript(theSignAndSaveWithoutDataScript),
  signandsavewithanillegalfilename: aPublishedScript(theSignAndSaveWithAnIllegalFilenameScript),
  signandsavewithsavingparameters: aPublishedScript(theSignAndSaveWithSavingParametersScript),
  signandsavewithecdsa: aPublishedScript(theSignAndSaveWithAnEcdsaAlgorithmScript),
};
