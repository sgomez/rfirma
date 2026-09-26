import type { Mock } from "vitest";
import { vi } from "vitest";
import type { Certificate } from "../signing/certificate";
import { recordingDocument } from "../viewer/testing/documentViewerFixtures";
import type { Errand } from "./errand";
import type { DescribedDocument, SiteCommands, SiteErrandView } from "./siteErrands";
import { siteErrands } from "./siteErrands";

/** Los dobles y auxiliares que comparten las pruebas de `siteErrands` (TD-78). */

export function certificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "handle-1",
    label: "FNMT",
    holderName: "ADA LOVELACE BYRON",
    stampedSigner: "ADA LOVELACE BYRON",
    givenName: "ADA",
    surname: "LOVELACE BYRON",
    idNumber: "99999999R",
    organizationIdentifier: null,
    issuer: "FNMT-RCM",
    certificateSerialNumber: "1234567890",
    store: "installed",
    status: { kind: "valid", notAfter: 4_102_444_800 },
    remembered: false,
    ...overrides,
  };
}

export const described: DescribedDocument = { title: "Solicitud", pages: 3, sizeBytes: 4096 };

/** El PDF abierto para marcar el área, sin nada que pintar. */
export const opened = recordingDocument().document;

/** Las órdenes, dobladas, y el asa para empujar momentos por el evento. */
function doubled(overrides: Partial<SiteCommands> = {}) {
  const stop: Mock = vi.fn();
  let emit: ((view: SiteErrandView) => void) | null = null;
  const calls: Record<keyof SiteCommands, Mock> = {
    watch: vi.fn(),
    readErrand: vi.fn(),
    identify: vi.fn(),
    confirmSignatures: vi.fn(),
    markArea: vi.fn(),
    decline: vi.fn(),
    beginSigning: vi.fn(),
    signWithPin: vi.fn(),
    finishSigning: vi.fn(),
    saveFile: vi.fn(),
    loadFiles: vi.fn(),
    installCertificate: vi.fn(),
    lookAgain: vi.fn(),
    installLocalCa: vi.fn(),
    closeWindow: vi.fn(),
    dismissWarning: vi.fn(),
    describeDocument: vi.fn(),
    openDocument: vi.fn(),
  };
  const commands: SiteCommands = {
    watch: (onView) => {
      calls.watch();
      emit = onView;
      return stop;
    },
    readErrand: async () => {
      calls.readErrand();
      return null;
    },
    identify: async (id) => {
      calls.identify(id);
      return { ok: true, value: undefined };
    },
    confirmSignatures: async () => {
      calls.confirmSignatures();
      return { ok: true, value: undefined };
    },
    markArea: async (area) => {
      calls.markArea(area);
      return { ok: true, value: true };
    },
    decline: async () => calls.decline(),
    beginSigning: async (id) => {
      calls.beginSigning(id);
      return { ok: true, value: { kind: "typedOnScreen" } };
    },
    signWithPin: async (secret) => {
      calls.signWithPin(secret);
      return { ok: true, value: undefined };
    },
    finishSigning: async () => {
      calls.finishSigning();
      return { ok: true, value: undefined };
    },
    saveFile: async () => {
      calls.saveFile();
      return { ok: true, value: true };
    },
    loadFiles: async () => {
      calls.loadFiles();
      return { ok: true, value: 1 };
    },
    installCertificate: async () => {
      calls.installCertificate();
      return true;
    },
    lookAgain: async () => calls.lookAgain(),
    installLocalCa: async () => calls.installLocalCa(),
    closeWindow: async () => calls.closeWindow(),
    dismissWarning: async () => calls.dismissWarning(),
    describeDocument: async (id) => {
      calls.describeDocument(id);
      return described;
    },
    openDocument: async (id) => {
      calls.openDocument(id);
      return opened;
    },
    ...overrides,
  };

  return { commands, calls, stop, push: (view: SiteErrandView) => emit?.(view) };
}

/** El puerto ya suscrito, con la lista de trámites que ha ido publicando. */
export function watched(overrides: Partial<SiteCommands> = {}) {
  const world = doubled(overrides);
  const port = siteErrands(world.commands);
  const seen: (Errand | null)[] = [];
  const unwatch = port.watch((errand) => seen.push(errand));
  return { ...world, port, seen, unwatch, last: () => seen[seen.length - 1] };
}

export const ASKING_TO_SIGN: SiteErrandView = {
  origin: "sede.ejemplo.gob.es",
  stage: {
    kind: "askingToSign",
    document: "asa-opaca-1",
    signing: "pdf",
    round: { kind: "cosign" },
    certificates: [certificate()],
    unregisteredSignatures: true,
    alreadyChosen: null,
    withoutAsking: false,
  },
};

export const MARKING_THE_AREA: SiteErrandView = {
  origin: "sede.ejemplo.gob.es",
  stage: { kind: "markingTheArea", document: "asa-opaca-1" },
};

export const ASKING_TO_CONFIRM: SiteErrandView = {
  origin: "sede.ejemplo.gob.es",
  stage: { kind: "askingToConfirm", messageCode: "pdfShadowAttackSuspect" },
};
