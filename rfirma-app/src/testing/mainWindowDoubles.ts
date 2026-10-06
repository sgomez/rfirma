//! La fábrica de dobles de la ventana principal: el juego completo de puertos, con sobrescrituras por nombre.

import type { MainWindowPorts } from "../App.ports";
import { unavailableExternalDestinationOpener } from "../desktop/externalDestination";
import { type FakeDocumentDrops, inMemoryDocumentDrops } from "../documents/drops";
import { inMemoryDocumentPicker } from "../documents/picker";
import { inMemoryRecents } from "../documents/recents";
import { inMemoryPreferences } from "../preferences/preferences";
import { defaults } from "../preferences/testing/fixtures";
import { absentWindowTheme } from "../preferences/theme";
import { absentNativeTitlebar } from "../shell/nativeTitlebar";
import { type CertificateStore, emptyCertificateStore } from "../signing/certificate";
import { inMemoryDestination, unavailableOpener } from "../signing/destination";
import { unavailableSigningBackend } from "../signing/flow";
import { emptyRubricPicker } from "../signing/rubric";
import { unavailableStampComposer } from "../signing/stampPreview";
import { memoryStatus } from "../status/status";
import { inMemoryVersionCheck } from "../updates/newVersion";
import { unavailablePdfSource } from "../viewer/source";

type Recents = ReturnType<typeof inMemoryRecents>;

/** Los puertos de la ventana principal, con los dobles que las pruebas manejan por dentro. */
export interface MainWindowDoubles extends MainWindowPorts {
  recents: Recents;
  drops: FakeDocumentDrops;
}

export type MainWindowDoubleOverrides = Partial<Omit<MainWindowDoubles, "certificates">> & {
  certificates?: Partial<CertificateStore>;
};

/** Los dobles que da la ventana principal mientras una prueba no diga otros. */
export function aMainWindowDoubles(overrides: MainWindowDoubleOverrides = {}): MainWindowDoubles {
  return {
    recents: overrides.recents ?? inMemoryRecents(),
    picker: overrides.picker ?? inMemoryDocumentPicker(),
    drops: overrides.drops ?? inMemoryDocumentDrops(null),
    preferences: overrides.preferences ?? inMemoryPreferences(defaults),
    pdfs: overrides.pdfs ?? unavailablePdfSource(),
    destinations:
      overrides.destinations ??
      inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true }),
    certificates: { ...emptyCertificateStore(), ...overrides.certificates },
    rubrics: overrides.rubrics ?? emptyRubricPicker(),
    stamps: overrides.stamps ?? unavailableStampComposer(),
    signer: overrides.signer ?? unavailableSigningBackend(),
    opener: overrides.opener ?? unavailableOpener(),
    versions: overrides.versions ?? inMemoryVersionCheck(),
    externalDestinations: overrides.externalDestinations ?? unavailableExternalDestinationOpener(),
    status: overrides.status ?? memoryStatus(),
    titlebar: overrides.titlebar ?? absentNativeTitlebar(),
    windowTheme: overrides.windowTheme ?? absentWindowTheme(),
  };
}
