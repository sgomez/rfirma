//! La fábrica de dobles de la ventana principal: el juego completo de puertos, con sobrescrituras por nombre.

import {
  type ExternalDestinationOpener,
  unavailableExternalDestinationOpener,
} from "../desktop/externalDestination";
import { type FakeDocumentDrops, inMemoryDocumentDrops } from "../documents/drops";
import { inMemoryRecents } from "../documents/recents";
import { type CertificateStore, emptyCertificateStore } from "../signing/certificate";
import {
  type DestinationSource,
  inMemoryDestination,
  type SignedDocumentOpener,
  unavailableOpener,
} from "../signing/destination";
import { type SigningBackend, unavailableSigningBackend } from "../signing/flow";
import { emptyRubricPicker, type RubricPicker } from "../signing/rubric";
import type { StatusPort } from "../status/status";
import { inMemoryVersionCheck, type VersionCheck } from "../updates/newVersion";
import { type PdfSource, unavailablePdfSource } from "../viewer/source";

type Recents = ReturnType<typeof inMemoryRecents>;

export interface MainWindowDoubles {
  recents: Recents;
  pdfs: PdfSource;
  certificates: CertificateStore;
  rubrics: RubricPicker;
  signer: SigningBackend;
  drops: FakeDocumentDrops;
  versions: VersionCheck;
  externalDestinations: ExternalDestinationOpener;
  status: StatusPort | undefined;
  destinations: DestinationSource;
  opener: SignedDocumentOpener;
}

export type MainWindowDoubleOverrides = Partial<Omit<MainWindowDoubles, "certificates">> & {
  certificates?: Partial<CertificateStore>;
};

/** Los dobles que da la ventana principal mientras una prueba no diga otros. */
export function aMainWindowDoubles(overrides: MainWindowDoubleOverrides = {}): MainWindowDoubles {
  return {
    recents: overrides.recents ?? inMemoryRecents(),
    pdfs: overrides.pdfs ?? unavailablePdfSource(),
    certificates: { ...emptyCertificateStore(), ...overrides.certificates },
    rubrics: overrides.rubrics ?? emptyRubricPicker(),
    signer: overrides.signer ?? unavailableSigningBackend(),
    drops: overrides.drops ?? inMemoryDocumentDrops(null),
    versions: overrides.versions ?? inMemoryVersionCheck(),
    externalDestinations: overrides.externalDestinations ?? unavailableExternalDestinationOpener(),
    status: overrides.status,
    destinations:
      overrides.destinations ??
      inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true }),
    opener: overrides.opener ?? unavailableOpener(),
  };
}
