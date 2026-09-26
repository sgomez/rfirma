import type { Certificate } from "./signing/certificate";
import { isUsable } from "./signing/certificate";
import type { SigningOrder } from "./signing/flow";
import { base64Of, type Rubric } from "./signing/rubric";
import type { CertificateState } from "./signing/SigningPanel";
import type { VisibleSignature } from "./signing/visibleSignature";
import {
  NO_PAGE_SETS,
  type PageChoice,
  type PageSets,
  type Placement,
  sealedPages,
  type UserSpaceRect,
} from "./viewer/signatureBox";

/**
 * La colocación **entera**, tal y como la guarda la ventana (#188).
 *
 * Un rectángulo, tres conjuntos —uno por opción— y cuál de ellas manda. Lo que
 * cruza a firmar es el `Placement` que sale de las tres, no esto: aquí vive el
 * estado de la interfaz, y ahí fuera solo se puede firmar en un sitio.
 */
export interface Placing {
  rect: UserSpaceRect | null;
  sets: PageSets;
  choice: PageChoice;
}

/**
 * La colocación guardada en la fila, repartida en las tres opciones (ID-74).
 *
 * La opción activa es **la que explica el conjunto sin inventar nada**: una
 * página sola es `Solo 1 página`, la palabra `"all"` es `Todas las páginas` y
 * cualquier otra cosa es `Estas páginas`. Las demás arrancan vacías a propósito
 * —no se rellenan «por si acaso»— para que la primera vez que se elijan se
 * siembren de esta, que es lo que pide la ficha.
 */
export function placingFrom(placement: Placement | null, pageCount: number): Placing {
  if (placement === null) return { rect: null, sets: NO_PAGE_SETS, choice: "single" };
  const { rect, pages } = placement;
  if (pages === "all") return { rect, sets: NO_PAGE_SETS, choice: "all" };
  const only = sealedPages(pages, pageCount);
  if (only.length === 1 && only[0] !== undefined) {
    return { rect, sets: { single: only[0], these: null }, choice: "single" };
  }
  return { rect, sets: { single: null, these: pages }, choice: "these" };
}

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
 * razón de que exista: el ID-107 promete que lo que se ve dentro del recuadro
 * coincide con el PDF firmado, y dos constructores separados podrían dejar de
 * cumplirlo sin que ninguna prueba lo notara.
 */
export function signingOrderFor({
  documentId,
  certificate,
  placement,
  geometry,
  pageCount,
  signature,
  rubric,
  signedAt,
  language,
}: {
  documentId: string;
  certificate: Certificate;
  placement: Placement;
  geometry: PageGeometry;
  pageCount: number;
  signature: VisibleSignature;
  rubric: Rubric | null;
  signedAt: string;
  language: string;
}): SigningOrder {
  return {
    document: documentId,
    certificate: certificate.id,
    placement: {
      page: geometry.page,
      pages: placement.pages,
      pageCount,
      mediaBox: geometry.view,
      rotation: geometry.rotate,
      rect: [placement.rect.x0, placement.rect.y0, placement.rect.x1, placement.rect.y1],
    },
    fields: signature.fields,
    reason: signature.reason,
    signedAt,
    // La rúbrica solo viaja si además está marcada: tener una imagen guardada
    // no es quererla dentro del recuadro.
    rubric: signature.rubric && rubric !== null ? base64Of(rubric) : null,
    language,
    // Nadie ha consentido nada todavía: el permiso se pone al aceptar el aviso
    // de las firmas sin registrar, y en ningún otro sitio (ID-301).
    allowUnregisteredSignatures: false,
  };
}

/**
 * La fecha del recuadro, en el idioma de la ventana.
 *
 * El **formato** es lo único de la firma visible que decide el frontal, y es a
 * propósito: quien sabe el huso y las convenciones de fecha del sistema es el
 * navegador, no Rust, y meter una biblioteca de husos en el backend para
 * repetir lo que `Intl` ya sabe sería duplicar el problema. Las **etiquetas**
 * del recuadro siguen siendo de `signing::layer2_text` (ID-19): aquí no se
 * escribe «Fecha», solo lo que va detrás.
 */
export function formatSignedAt(instant: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "medium",
  }).format(instant);
}

/**
 * Con qué certificado se firma, a partir de los que hay.
 *
 * Con uno solo no se pregunta: elegir entre una cosa no es elegir. Pero eso no
 * vale para uno caducado —preseleccionar un certificado inservible sería
 * elegir por la persona usuaria con qué identidad firma, y eso no lo hace la
 * aplicación por su cuenta (#197)—, así que con uno solo inservible el
 * desplegable arranca sin elección, igual que si no hubiera ninguno.
 *
 * Con varios manda **el que se usó la última vez** (#110): quien tiene cuatro
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
  // Ni siquiera con uno solo se elige solo (#973, revierte el #197): elegir
  // con qué identidad se firma un documento con validez jurídica no lo hace la
  // aplicación por su cuenta, y «hay uno solo» no es una excepción a esa regla.
  const remembered = found.find((one) => one.remembered);
  if (remembered !== undefined && isUsable(remembered.status)) {
    return { kind: "chosen", certificate: remembered, certificates: found };
  }
  return { kind: "unchosen", certificates: found };
}
