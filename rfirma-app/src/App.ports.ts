//! Los puertos de la ventana principal, todos obligatorios: quien monta `App` los cablea todos, y la ausencia de uno es su adaptador ausente.

import type { ExternalDestinationOpener } from "./desktop/externalDestination";
import type { DocumentDrops } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import type { RecentsStore } from "./documents/recents";
import type { PreferencesStore } from "./preferences/preferences";
import type { WindowTheme } from "./preferences/theme";
import type { NativeTitlebar } from "./shell/nativeTitlebar";
import type { CertificateStore } from "./signing/certificate";
import type { DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import type { RubricPicker } from "./signing/rubric";
import type { StampComposer } from "./signing/stampPreview";
import type { StatusPort } from "./status/status";
import type { VersionCheck } from "./updates/newVersion";
import type { PdfSource } from "./viewer/source";

export interface MainWindowPorts {
  recents: RecentsStore;
  picker: DocumentPicker;
  /** Por dónde entra un PDF arrastrado a la ventana. */
  drops: DocumentDrops;
  preferences: PreferencesStore;
  /** De dónde salen los bytes del PDF que se pinta. */
  pdfs: PdfSource;
  /** Dónde caerá el documento que hay delante. */
  destinations: DestinationSource;
  /** Los certificados de los tokens conectados. */
  certificates: CertificateStore;
  /** Por dónde entra la rúbrica, ya normalizada. */
  rubrics: RubricPicker;
  /** Quien compone el sello que se ve dentro del recuadro. */
  stamps: StampComposer;
  /** Quien ejecuta las tres etapas de la firma. */
  signer: SigningBackend;
  /** Quien lleva al usuario hasta el fichero firmado. */
  opener: SignedDocumentOpener;
  /** Si hay una versión nueva publicada. */
  versions: VersionCheck;
  /** Quien abre destinos externos fuera de la aplicación. */
  externalDestinations: ExternalDestinationOpener;
  /** Quien lee y reevalúa las señales del panel de estado. */
  status: StatusPort;
  /** La barra de título GTK de Linux. */
  titlebar: NativeTitlebar;
  /** Quien fija el tema de la ventana nativa. */
  windowTheme: WindowTheme;
}
