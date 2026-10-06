//! El componente `App` de la ventana principal: compone los `App.use*`, reparte su estado por el árbol y entrega el `AppHandle` que abre sus vistas desde fuera.

import { useEffect, useMemo, useState } from "react";
import { forgetActivity } from "./App.forgetActivity";
import type { MainWindowPorts } from "./App.ports";
import { useDropNotices } from "./App.useDropNotices";
import { useNativeTitlebar } from "./App.useNativeTitlebar";
import { useOpenShortcut } from "./App.useOpenShortcut";
import { usePreferencesState } from "./App.usePreferencesState";
import { useStartupNotices } from "./App.useStartupNotices";
import { AboutDialog } from "./about/AboutDialog";
import { DocumentTabs } from "./documents/DocumentTabs";
import { isAPdf } from "./documents/document";
import { RecentsSection } from "./documents/RecentRows";
import type { RecentDocument } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { useOpenPdf } from "./documents/useOpenPdf";
import { SigningJourneyDialogs } from "./journey/SigningJourneyDialogs";
import { useSigningJourney } from "./journey/useSigningJourney";
import { usePlacement } from "./placement/usePlacement";
import { PreferencesView } from "./preferences/PreferencesView";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import { SignaturesPanel } from "./signing/SignaturesPanel";
import { SigningPanel } from "./signing/SigningPanel";
import { useCertificateListing } from "./signing/useCertificateListing";
import { useSignatureReading } from "./signing/useSignatureReading";
import type { VisibleSignature } from "./signing/visibleSignature";
import { StatusWindow } from "./status/StatusWindow";
import { InstallUpdateDialog } from "./updates/InstallUpdateDialog";
import { NewVersionStrip } from "./updates/NewVersionStrip";
import { DocumentViewer } from "./viewer/DocumentViewer";
import { standardRectOnPageOf } from "./viewer/signatureBox";

type OpenDialog = "about" | "installUpdate" | null;
type ActiveView = "status" | "preferences" | null;

const NO_RECENTS: readonly RecentDocument[] = [];

interface AppProps {
  /** Los puertos de la ventana. Ver [`MainWindowPorts`]. */
  ports: MainWindowPorts;
  initialSignature: VisibleSignature;
  /** La versión del binario que enseña «Acerca de». */
  version: string;
  /** Dónde va el menú. Por omisión, lo que diga la plataforma. */
  menuAnchor?: MenuAnchor;
  /** Recibe, una vez montada, el asa con la que `main.tsx` abre sus vistas desde fuera. */
  onReady?: (handle: AppHandle) => void;
  /** Otra pantalla tapa la ventana, como el asistente del primer arranque. */
  covered?: boolean;
}

/** El asa que `onReady` entrega: lo único de `App` que se abre desde fuera. */
export interface AppHandle {
  openStatus: () => void;
  openPreferences: () => void;
  openAbout: () => void;
}

/**
 * La composición: quién habla con quién.
 *
 * Todo lo que toca el disco entra por parámetro —los recientes, el portal y los
 * ajustes—, así que la aplicación entera se puede pintar en una prueba sin
 * backend. Quien elige las implementaciones de verdad es `main.tsx`.
 *
 * Los diálogos se montan **sobre** la ventana y no la desmontan: no hay
 * navegación, y los documentos abiertos siguen vivos debajo.
 */
export function App({
  ports,
  initialSignature,
  version,
  menuAnchor,
  onReady,
  covered = false,
}: AppProps) {
  const {
    recents,
    picker,
    drops,
    preferences,
    pdfs,
    destinations,
    certificates,
    rubrics,
    stamps,
    signer,
    opener,
    versions,
    externalDestinations,
    status,
    titlebar,
    windowTheme,
  } = ports;
  const [dialog, setDialog] = useState<OpenDialog>(null);
  const [view, setView] = useState<ActiveView>(null);

  useEffect(() => {
    onReady?.({
      openStatus: () => setView("status"),
      openPreferences: () => setView("preferences"),
      openAbout: () => setDialog("about"),
    });
  }, [onReady]);

  const { newVersion, versionDismissed, setVersionDismissed, setStatusRows, hasAttention } =
    useStartupNotices(status, versions);

  const certificateListing = useCertificateListing(certificates);
  const { settings, changeSettings, chooseDestination, rubric, rubricFailure, chooseRubric } =
    usePreferencesState(preferences, rubrics, covered, windowTheme);
  const rubricSection = useMemo(
    () => ({ value: rubric, failure: rubricFailure, choose: () => void chooseRubric() }),
    [rubric, rubricFailure, chooseRubric],
  );
  // Mientras los ajustes se leen todavía no se sabe, y lo guardado por omisión es recordar.
  const rememberActivity = settings?.rememberActivity ?? true;
  const documents = useDocuments(recents, picker, rememberActivity);
  const activeId = documents.active?.id ?? null;
  const {
    pdf,
    failure: pdfFailure,
    sizeBytes,
    opening,
    open: openDocument,
    clearRecents,
  } = useOpenPdf(documents, pdfs);
  // Con la actividad apagada no se enseñan los recientes que ya hubiera guardados.
  const visibleRecents = rememberActivity ? documents.recents : NO_RECENTS;

  const standardRectOn = useMemo(() => (pdf === null ? null : standardRectOnPageOf(pdf)), [pdf]);
  const placementState = usePlacement({
    document: opening,
    standardRectOn,
    onChange: documents.place,
  });
  const { placement, viewPage, moveBox, sealPage } = placementState;

  const journey = useSigningJourney({
    ports: { signer, stamps, destinations, opener },
    document: { active: documents.active, pdf, sizeBytes, opening },
    placement: placementState,
    certificates: certificateListing,
    rubric,
    settingsFolder: settings?.destination ?? null,
    initialSignature,
    reopenDocument: documents.reopen,
  });
  const { stamp, acknowledgement } = journey;

  const signatureReading = useSignatureReading(
    signer,
    opener,
    documents.active,
    visibleRecents.find((row) => row.id === activeId),
  );
  const { dropNotice } = useDropNotices(
    drops,
    documents.accept,
    documents.enter,
    activeId,
    signatureReading.view,
  );

  const modalOpen = dialog !== null || journey.signals.dialogOpen;
  const canOpen = !covered && view === null && !modalOpen;
  useOpenShortcut(openDocument, canOpen);

  const anchor = menuAnchor ?? menuAnchorFor(navigator.userAgent);
  const warningVisible = hasAttention && view !== "status";
  const openHelp = () => void externalDestinations.open("discussions");
  const signaturesShown = acknowledgement ?? signatureReading.reading;
  const acknowledgementMoment = acknowledgement && {
    kind: "acknowledgement" as const,
    signedAt: acknowledgement.signedAt,
    signatures: acknowledgement.signatures,
    findings: acknowledgement.findings,
    onChangeDestination: () => void acknowledgement.changeDestination(),
  };
  const readingMoment = signatureReading.reading && {
    kind: "reading" as const,
    state: signatureReading.reading,
    signable: signatureReading.reading.signable,
  };
  const signaturesMoment = acknowledgementMoment ?? readingMoment;
  const unlessModal = (action: () => void) => () => {
    if (!modalOpen) action();
  };
  useNativeTitlebar(
    titlebar,
    !covered && view === null,
    warningVisible && !covered,
    visibleRecents,
    {
      open: () => {
        if (canOpen) openDocument();
      },
      status: unlessModal(() => setView("status")),
      preferences: unlessModal(() => setView("preferences")),
      feedback: unlessModal(openHelp),
      about: unlessModal(() => setDialog("about")),
      clearRecents: unlessModal(clearRecents),
      recent: (id) => {
        const row = visibleRecents.find((one) => one.id === id);
        if (!modalOpen && row?.available) documents.select(row);
      },
    },
  );

  const forgetAll = () =>
    forgetActivity(
      () => preferences.forgetActivity(),
      () => documents.forgetAll(),
    );

  const notifyNewVersion = settings?.notifyNewVersion ?? true;

  return (
    <>
      <MainWindow
        menuAnchor={anchor}
        hasAttention={warningVisible}
        onOpenStatus={() => setView("status")}
        onOpenPreferences={() => setView("preferences")}
        onOpenHelp={openHelp}
        onOpenAbout={() => setDialog("about")}
        view={
          view === "status" ? (
            <StatusWindow
              statusPort={status}
              externalDestinations={externalDestinations}
              onClose={() => setView(null)}
              onRowsChange={setStatusRows}
            />
          ) : view === "preferences" && settings !== null ? (
            <PreferencesView
              preferences={settings}
              onChooseDestination={chooseDestination}
              onChange={changeSettings}
              onForgetActivity={forgetAll}
              installedCertificates={certificateListing.installed}
              onInstallCertificate={certificateListing.install}
              onRemoveCertificate={certificateListing.remove}
              onEmptyStore={certificateListing.emptyStore}
              onClose={() => setView(null)}
            />
          ) : null
        }
        notification={
          <NewVersionStrip
            newVersion={versionDismissed || !notifyNewVersion ? null : newVersion}
            onOpen={setDialog}
            onDismiss={() => setVersionDismissed(true)}
          />
        }
        tabs={
          anchor === "titlebar" && documents.tabs.length === 0 ? null : (
            <DocumentTabs
              tabs={documents.tabs}
              activeId={activeId}
              recents={visibleRecents}
              onActivate={documents.activate}
              onClose={documents.close}
              onOpen={openDocument}
              onSelectRecent={documents.select}
              onClearRecents={clearRecents}
              signingLocked={journey.signals.signing}
              withOpenButton={anchor !== "titlebar"}
            />
          )
        }
        viewer={
          <DocumentViewer
            pdf={pdf}
            stamped={stamp.pdf}
            stampFrozen={stamp.frozen}
            onGesture={stamp.onGesture}
            placement={placement}
            canPlace={journey.signature.value.enabled}
            onMove={moveBox}
            onTrace={sealPage}
            onPageChange={viewPage}
            onOpen={openDocument}
            emptyExtra={
              documents.tabs.length === 0 ? (
                <RecentsSection
                  recents={visibleRecents}
                  onSelect={documents.select}
                  onClear={clearRecents}
                />
              ) : null
            }
            // Los dos avisos caben en el mismo sitio, y manda el del PDF: si el
            // documento que se soltó tampoco se deja pintar, eso es más urgente
            // que contar cuántos ficheros venían con él.
            failure={pdfFailure ?? (dropNotice?.about === activeId ? dropNotice.failure : null)}
            withoutPreview={
              documents.active && !isAPdf(documents.active) ? documents.active.name : null
            }
            stamp={stamp.state}
            rubricGap={stamp.rubricGap}
            onComposeStamp={stamp.compose}
            onOpenHelp={openHelp}
          />
        }
        panel={
          signaturesShown !== null && signaturesMoment !== null ? (
            // Firmado: la columna derecha cambia de contenido, no de sitio. Es
            // el único acuse de recibo que recibe quien firma, así que se monta
            // en cuanto la postfirma devuelve el documento.
            // Solo mientras siga activo **el documento que se firmó**: el
            // recuento de páginas sale del PDF abierto, y con otro delante sería
            // el nombre de un fichero con las páginas de otro. Sin documento
            // activo tampoco se monta, o quedaría una tercera columna al lado
            // del visor vacío.
            <SignaturesPanel
              documentName={signaturesShown.documentName}
              destination={signaturesShown.destination}
              moment={signaturesMoment}
              onOpenDocument={signaturesShown.openDocument}
              onOpenFolder={signaturesShown.openFolder}
              onSign={signaturesShown.signAgain}
              failure={signaturesShown.openFailure}
              onOpenHelp={openHelp}
            />
          ) : pdf && documents.active ? (
            <SigningPanel
              document={{
                id: documents.active.id,
                name: documents.active.name,
                sizeBytes,
              }}
              previousSignatures={journey.previousSignatures}
              certificate={journey.certificate}
              signature={journey.signature}
              placementState={placementState}
              rubric={rubricSection}
              destination={journey.destination}
              signing={journey.signing}
              onOpenHelp={openHelp}
              failure={journey.failure}
              onEmptyStore={() => void certificateListing.emptyStore()}
            />
          ) : null
        }
      />
      {dialog === "about" && (
        <AboutDialog
          version={version}
          newVersion={newVersion}
          versions={versions}
          offerUpdate={notifyNewVersion}
          onOpenSourceCode={() => void externalDestinations.open("sourceCode")}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog === "installUpdate" && newVersion !== null && (
        <InstallUpdateDialog
          newVersion={newVersion}
          versions={versions}
          onClose={() => setDialog(null)}
        />
      )}
      <SigningJourneyDialogs journey={journey} />
    </>
  );
}
