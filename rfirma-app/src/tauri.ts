/**
 * Los puertos que hablan con Tauri, reunidos aquí para que la ventana y
 * `main.tsx` sigan importando un solo módulo: cada familia vive en su propio
 * `tauri*.ts` —firma, documento, configuración, sede y estado—, y estos son
 * los únicos ficheros del frontal que saben que debajo hay Tauri.
 *
 * La ventana y sus pruebas siguen hablando con `CertificateStore`,
 * `SigningBackend`, `DocumentPicker`, `PdfSource`, `DocumentDrops` y
 * `RubricPicker`, y quien elige entre estas implementaciones y los dobles de
 * memoria es `main.tsx`.
 *
 * # Los fallos llegan clasificados, no traducidos
 *
 * Las órdenes rechazan con la forma del ID-29 —una situación nuestra y el texto
 * original crudo al lado—, así que aquí no hay ni una tabla de `CKR_*` ni un
 * `catch` que invente un mensaje: lo que no venga con esa forma —una excepción
 * del propio puente de Tauri, una orden que no existe— cae en `unknown` con su
 * texto tal cual, que es exactamente lo que el ADR-0009 pide. Quien lo decide
 * es `errors/classify.ts`, que no es de Tauri sino del ID-29.
 */

export {
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriPdfSource,
  tauriRecents,
} from "./tauriDocuments";
export {
  tauriDestinations,
  tauriExternalDestinationOpener,
  tauriLanguagePreference,
  tauriPreferences,
  tauriSignedDocumentOpener,
  tauriVersionCheck,
} from "./tauriPreferences";
export { tauriSiteErrands } from "./tauriSede";
export {
  tauriCertificateStore,
  tauriRubricPicker,
  tauriSigningBackend,
  tauriStampComposer,
} from "./tauriSigning";
export { tauriStatusPort } from "./tauriStatus";
