//! Las historias del desenlace de sede: cada final con su icono y su título, y cada rechazo con su frase y su detalle.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import {
  NAMED_BY_THE_DESK,
  REFUSAL_ACTION_OF,
  type RefusalSituation,
  type SiteOutcome,
} from "./errand";
import { SedeEnding } from "./SedeEnding";
import { sedeViewActions } from "./testing/fixtures/sedeView";
import { signedDocument } from "./testing/fixtures/sedeWindow";

const ENDINGS = {
  signed: { kind: "signed", document: signedDocument },
  batchSigned: { kind: "batchSigned", signs: 3 },
  cancelled: { kind: "cancelled", document: signedDocument },
  saved: { kind: "saved" },
  loaded: { kind: "loaded", fileCount: 2 },
} satisfies Record<string, SiteOutcome>;

const DETAILS: Partial<Record<RefusalSituation, string>> = {
  missingFormat: "SAF_01: falta el parámetro format",
  sha1: "SAF_03: el algoritmo 'SHA1withRSA' es SHA-1: rFirma firma con SHA-2",
  invoiceMultisignature: "SAF_05: multifirma de factura",
  unsupportedCountersignature: "SAF_06: contrafirma no admitida",
  portsTaken: "SAF_09: puertos ocupados",
  saveCancelled: "SAF_12: guardado cancelado",
  certificateNotFound: "SAF_13: certificado no encontrado",
  unknown: "error inesperado",
  incorrectPin: "CKR_PIN_INCORRECT",
};

const SITUATION_LABELS: Record<RefusalSituation, string> = {
  appendedSignaturePage: "Página de firma añadida",
  unsupportedFilter: "Filtro de certificados no admitido",
  unsupportedProtocolVersion: "Versión del protocolo no admitida",
  missingFormat: "Falta el formato",
  unsupportedKeyStore: "Almacén de claves no admitido",
  errandInFlight: "Otro trámite en curso",
  portsTaken: "Puertos ocupados",
  sha1: "Pide SHA-1",
  invoiceMultisignature: "Multifirma de factura",
  unsupportedCountersignature: "Contrafirma no admitida",
  saveCancelled: "Guardado cancelado",
  loadCancelled: "Carga cancelada",
  cannotSaveData: "No se pueden guardar los datos",
  cannotLoadData: "No se pueden cargar los datos",
  batchPresignerUnreachable: "Lote: prefirma inalcanzable",
  batchPostsignerUnreachable: "Lote: postfirma inalcanzable",
  batchInvalidPresignResponse: "Lote: respuesta de prefirma no válida",
  batchInvalidPostsignResponse: "Lote: respuesta de postfirma no válida",
  batchSigningFailed: "Lote: la firma falló",
  triphaseServerUrlMissing: "Trifásica: falta la URL del servidor",
  triphaseServerException: "Trifásica: excepción del servidor",
  triphaseServerUnreachable: "Trifásica: servidor inalcanzable",
  triphaseServerUnexpectedAnswer: "Trifásica: respuesta inesperada",
  certificateNotFound: "Certificado no encontrado",
  folderMissing: "La carpeta no existe",
  unwritable: "No se puede escribir",
  invalidSignature: "Firma no válida",
  confirmationNeeded: "Pide confirmación",
  localBatchSign: "Lote local",
  unrecognisedType: "Tipo no reconocido",
  siteErrandNotLive: "El trámite ya no está activo",
  pdfHasUnregisteredSignatures: "PDF con firmas sin registrar",
  secretOnTheReaderKeypad: "PIN en el teclado del lector",
  userCancelled: "Cancelado por la persona",
  promptFailed: "Falló el diálogo",
  unknown: "Desconocido",
  incorrectPin: "Escritorio: PIN incorrecto",
  pinLocked: "Escritorio: PIN bloqueado",
  tokenAbsent: "Escritorio: sin tarjeta",
  expiredSession: "Escritorio: sesión caducada",
  moduleNotFound: "Escritorio: módulo no encontrado",
  pkcs12Unreadable: "Escritorio: PKCS#12 ilegible",
  incorrectPkcs12Password: "Escritorio: contraseña de PKCS#12 incorrecta",
  pkcs12NoPrivateKey: "Escritorio: PKCS#12 sin clave privada",
  keyKindUnsupported: "Escritorio: tipo de clave no admitido",
  mechanismNotOffered: "Escritorio: mecanismo no ofrecido",
  notAPdf: "Escritorio: no es un PDF",
  documentEncrypted: "Escritorio: documento cifrado",
  documentCertified: "Escritorio: documento certificado",
  documentUnreadable: "Escritorio: documento ilegible",
  boxOutOfPage: "Escritorio: recuadro fuera de la página",
  pageOutOfDocument: "Escritorio: página fuera del documento",
  sealMismatch: "Escritorio: el sello no coincide",
  bridgeFailed: "Escritorio: falló el puente",
  notAFolder: "Escritorio: no es una carpeta",
  folderUnreadable: "Escritorio: carpeta ilegible",
  folderUnwritable: "Escritorio: carpeta sin escritura",
  noFreeName: "Escritorio: sin nombre libre",
  removalNotSupported: "Escritorio: no se puede quitar",
  noKeyring: "Escritorio: sin llavero",
  keyringPinMissing: "Escritorio: falta el PIN del llavero",
};

const SITUATIONS = [...Object.keys(REFUSAL_ACTION_OF), ...NAMED_BY_THE_DESK].filter(
  (situation) => situation !== "unknown",
) as RefusalSituation[];

interface EndingDesignProps {
  ending: "outcome" | "refused";
  kind: keyof typeof ENDINGS;
  situation: RefusalSituation;
  withOrigin: boolean;
  withReport?: boolean;
}

function EndingDesign({
  ending,
  kind,
  situation,
  withOrigin,
  withReport = false,
}: EndingDesignProps) {
  const outcome: SiteOutcome =
    ending === "outcome"
      ? ENDINGS[kind]
      : { kind: "refused", situation, detail: DETAILS[situation] ?? situation };

  return (
    <SedeEnding
      key={`${ending}-${kind}-${situation}`}
      origin={withOrigin ? "sede.villaejemplo.es" : null}
      outcome={outcome}
      onClose={sedeViewActions.onClose}
      onOpenHelp={sedeViewActions.onOpenHelp}
      onReport={withReport ? sedeViewActions.onClose : undefined}
    />
  );
}

const hidden = { table: { disable: true } };

const withOrigin = {
  description: "Si se sabe qué sede lo pidió; sin origen, la cabecera la nombra sin identificar",
};

const meta = {
  title: "Pantallas/Ventana de sede/Desenlace",
  ...sedeMomentMeta,
  component: EndingDesign,
  args: { ending: "outcome", kind: "signed", situation: "missingFormat", withOrigin: true },
  argTypes: {
    ending: hidden,
    kind: hidden,
    situation: hidden,
    withOrigin,
  },
  parameters: {
    ...sedeMomentMeta.parameters,
    docs: {
      description: {
        component: [
          "El desenlace de sede con el esqueleto del consentimiento compacto. Arriba, la cabecera con solo el nombre de la sede, sin «Te pide la firma»; debajo, alineado a la izquierda con ese nombre, el icono a la izquierda del título, en la misma fila, sin nada de lo que se firmó; abajo, «Cerrar» donde estaba «Firmar», con los segundos que faltan para irse sola: «Cerrar (15)» baja hasta cerrar.",
          "En el rechazo, bajo el título y alineados a la izquierda, una sola frase con la causa o con lo que puede hacer la persona, y el detalle técnico copiable con su nota para la sede. El desconocido añade «¿Te parece un fallo de rFirma?» con la ayuda, y como el que pide cerrar otro programa, no se cierra solo: el botón dice «Cerrar» a secas.",
          "No repitas qué se firmó ni cuántos documentos: eso ya se vio al consentir. No pongas la cuenta atrás en una frase aparte: la lleva el botón.",
        ].join("\n\n"),
      },
    },
  },
} satisfies Meta<typeof EndingDesign>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Outcome: Story = {
  argTypes: {
    kind: {
      description: "Cómo acabó el trámite",
      control: "select",
      options: Object.keys(ENDINGS),
      labels: {
        signed: "Firmado",
        batchSigned: "Lote firmado",
        cancelled: "Cancelado",
        saved: "Guardado",
        loaded: "Cargado",
      },
      table: { disable: false },
    },
  },
};

export const Refused: Story = {
  args: { ending: "refused" },
  argTypes: {
    situation: {
      description:
        "Por qué rFirma rechazó la petición; las del escritorio se cuentan con el título de su mensaje",
      control: "select",
      options: SITUATIONS,
      labels: SITUATION_LABELS,
      table: { disable: false },
    },
  },
};

export const RefusedUnknown: Story = { args: { ending: "refused", situation: "unknown" } };

export const RefusedWithReport: Story = {
  name: "Mockup · Rechazado, con informe",
  args: { ending: "refused", situation: "batchPostsignerUnreachable", withReport: true },
};
