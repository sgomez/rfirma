//! La geometría de la página y la orden de firma armada en un solo sitio, y el certificado que se elige de los encontrados. Sin React.

import type { Placement } from "../placement/pageSets";
import type { Certificate } from "../signing/certificate";
import { isUsable } from "../signing/certificate";
import type { SigningOrder } from "../signing/flow";
import { base64Of, type Rubric } from "../signing/rubric";
import type { CertificateState } from "../signing/SigningPanel";
import type { VisibleSignature } from "../signing/visibleSignature";

/**
 * La página que mide la orden: la primera del conjunto, con su caja y su giro.
 *
 * El backend **no lee PDFs** —la conversión del recuadro a puntos PAdES es
 * suya, pero los datos de la página los tiene `pdf.js`—, así que esto es lo que
 * la ventana tiene que llevarle.
 */
export interface PageGeometry {
  page: number;
  view: readonly [number, number, number, number];
  rotate: number;
}

/**
 * La orden de firma, armada en **un solo sitio**.
 *
 * La usan la firma de verdad y el ciclo en seco de la vista previa, y esa es la
 * razón de que exista: lo que se ve dentro del recuadro tiene que
 * coincidir con el PDF firmado, y dos constructores separados podrían dejar de
 * cumplirlo sin que ninguna prueba lo notara.
 */
export function signingOrderFor({
  documentId,
  certificate,
  box,
  pageCount,
  signature,
  rubric,
  signedAt,
  language,
}: {
  documentId: string;
  certificate: Certificate;
  box: { placement: Placement; geometry: PageGeometry } | null;
  pageCount: number;
  signature: VisibleSignature;
  rubric: Rubric | null;
  signedAt: string;
  language: string;
}): SigningOrder {
  return {
    document: documentId,
    certificate: certificate.id,
    placement: box && {
      page: box.geometry.page,
      pages: box.placement.pages,
      pageCount,
      mediaBox: box.geometry.view,
      rotation: box.geometry.rotate,
      rect: [
        box.placement.rect.x0,
        box.placement.rect.y0,
        box.placement.rect.x1,
        box.placement.rect.y1,
      ],
    },
    content: signature.content,
    withRubric: signature.withRubric,
    signedAt,
    // La rúbrica solo viaja si además está marcada: tener una imagen guardada
    // no es quererla dentro del recuadro.
    rubric: box !== null && signature.withRubric && rubric !== null ? base64Of(rubric) : null,
    language,
    // Nadie ha consentido nada todavía: el permiso se pone al aceptar el aviso
    // de las firmas sin registrar, y en ningún otro sitio.
    allowUnregisteredSignatures: false,
  };
}

/**
 * Con qué certificado se firma, a partir de los que hay.
 *
 * Nunca se elige por su cuenta: elegir con qué identidad se firma un
 * documento con validez jurídica es cosa de la persona usuaria, y «hay uno
 * solo» no es una excepción a esa regla —preseleccionarlo sería elegir por
 * ella—.
 *
 * Manda **el que se usó la última vez**: quien tiene cuatro
 * certificados los elige una vez, no cada día. Eso no contradice la regla de
 * que la aplicación no elige por su cuenta: no está eligiendo, está devolviendo
 * lo que ya se eligió firmando. Pero viene con su estado de ahora, no con el
 * de entonces, y la misma regla del párrafo anterior le alcanza igual: si
 * desde la última firma caducó, no sale puesto —«nunca se preselecciona un
 * certificado no utilizable» no tiene excepción para el recordado—, y el
 * desplegable arranca sin elección como si no hubiera recordado ninguno.
 *
 * Sin recordado —primera vez, o el recordado ya no está en el token, o ya no
 * sirve— sigue sin haber preselección: el desplegable dice «Elegir
 * certificado» y el botón de firmar sigue apagado, porque el orden de la
 * lista solo dice en qué orden cargaron los módulos.
 */
export function chosenFrom(found: readonly Certificate[]): CertificateState {
  const [first] = found;
  if (first === undefined) return { kind: "empty" };
  const remembered = found.find((one) => one.remembered);
  if (remembered !== undefined && isUsable(remembered.status)) {
    return { kind: "chosen", certificate: remembered, certificates: found };
  }
  return { kind: "unchosen", certificates: found };
}
