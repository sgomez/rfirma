import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { forgetActivity } from "./App.forgetActivity";
import { SignFlowPrompts, signFlowPromptOpen } from "./App.SignFlowPrompts";
import { formatSignedAt, placingFrom } from "./App.signingOrder";
import { useCertificateSearch } from "./App.useCertificateSearch";
import { useDropNotices } from "./App.useDropNotices";
import { useNativeTitlebar } from "./App.useNativeTitlebar";
import { useOpenShortcut } from "./App.useOpenShortcut";
import { usePageGeometry } from "./App.usePageGeometry";
import { usePlacementControls } from "./App.usePlacementControls";
import { useDestination, usePreferencesState } from "./App.usePreferencesState";
import { usePreviousSignatures } from "./App.usePreviousSignatures";
import { useSignedSummary } from "./App.useSignedSummary";
import { useSignFlow } from "./App.useSignFlow";
import { useSigningFailure } from "./App.useSigningFailure";
import { useStartupNotices } from "./App.useStartupNotices";
import { useVisibleSignature } from "./App.useVisibleSignature";
import { AboutDialog } from "./about/AboutDialog";
import type { ExternalDestinationOpener } from "./desktop/externalDestination";
import { unavailableExternalDestinationOpener } from "./desktop/externalDestination";
import { DocumentTabs } from "./documents/DocumentTabs";
import type { DocumentDrops } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import { RecentsSection } from "./documents/RecentRows";
import type { RecentDocument, RecentsStore } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { classify } from "./errors/classify";
import { PreferencesView } from "./preferences/PreferencesView";
import type { PreferencesStore } from "./preferences/preferences";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import { absentNativeTitlebar, type NativeTitlebar } from "./shell/nativeTitlebar";
import type { CertificateStore } from "./signing/certificate";
import type { DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import type { RubricPicker } from "./signing/rubric";
import { SignedPanel } from "./signing/SignedPanel";
import { SigningPanel } from "./signing/SigningPanel";
import { SigningProgressDialog } from "./signing/SigningProgressDialog";
import type { StampComposer } from "./signing/stampPreview";
import { useSigning } from "./signing/useSigning";
import type { VisibleSignature } from "./signing/visibleSignature";
import { StatusView } from "./status/StatusView";
import { memoryStatus, type StatusPort } from "./status/status";
import { InstallUpdateDialog } from "./updates/InstallUpdateDialog";
import { NewVersionStrip } from "./updates/NewVersionStrip";
import type { VersionCheck } from "./updates/newVersion";
import { DocumentViewer } from "./viewer/DocumentViewer";
import type { PdfDocument } from "./viewer/pdf";
import { firstSealedPage, NO_PAGE_SETS } from "./viewer/signatureBox";
import type { DocumentFailure, PdfSource } from "./viewer/source";

type OpenDialog = "about" | "installUpdate" | null;
type ActiveView = "status" | "preferences" | null;

const NO_RECENTS: readonly RecentDocument[] = [];
const NO_TITLEBAR = absentNativeTitlebar();

interface AppProps {
  recents: RecentsStore;
  picker: DocumentPicker;
  /** Por dónde entra un PDF arrastrado a la ventana. Ver [`DocumentDrops`]. */
  drops: DocumentDrops;
  preferences: PreferencesStore;
  /** De dónde salen los bytes del PDF que se pinta. Ver [`PdfSource`]. */
  pdfs: PdfSource;
  /** Dónde caerá el documento que hay delante. Ver [`DestinationSource`]. */
  destinations: DestinationSource;
  /** Los certificados de los tokens conectados. Ver [`CertificateStore`]. */
  certificates: CertificateStore;
  /** Por dónde entra la rúbrica, ya normalizada. Ver [`RubricPicker`]. */
  rubrics: RubricPicker;
  /** Quien compone el sello que se ve dentro del recuadro. Ver [`StampComposer`]. */
  stamps: StampComposer;
  /** Quien ejecuta las tres etapas de la firma. Ver [`SigningBackend`]. */
  signer: SigningBackend;
  /** Quien lleva al usuario hasta el fichero firmado. Ver [`SignedDocumentOpener`]. */
  opener: SignedDocumentOpener;
  initialSignature: VisibleSignature;
  /** Si hay una versión nueva publicada. Ver [`VersionCheck`]. */
  versions: VersionCheck;
  /** La versión del binario que enseña «Acerca de». */
  version: string;
  /** Dónde va el menú. Por omisión, lo que diga la plataforma. */
  menuAnchor?: MenuAnchor;
  /** Quien abre destinos externos fuera de la aplicación. Ver [`ExternalDestinationOpener`]. */
  externalDestinations?: ExternalDestinationOpener;
  /** Quien lee y reevalúa las señales del panel de estado. Ver [`StatusPort`]. */
  status?: StatusPort;
  /** Recibe, una vez montada, el asa con la que `main.tsx` abre sus vistas desde fuera. */
  onReady?: (handle: AppHandle) => void;
  /** Otra pantalla tapa la ventana, como el asistente del primer arranque. */
  covered?: boolean;
  /** La barra de título GTK de Linux. Ver [`NativeTitlebar`]. */
  titlebar?: NativeTitlebar;
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
  initialSignature,
  versions,
  version,
  menuAnchor,
  externalDestinations = unavailableExternalDestinationOpener(),
  status = memoryStatus(),
  onReady,
  covered = false,
  titlebar = NO_TITLEBAR,
}: AppProps) {
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

  const [pdf, setPdf] = useState<PdfDocument | null>(null);
  // Por qué no se pudo pintar el último documento que se eligió. Vive al lado
  // del PDF y no dentro del visor porque lo produce quien abre, y el visor solo
  // lo enseña.
  const [pdfFailure, setPdfFailure] = useState<DocumentFailure | null>(null);
  // Cuánto ocupa el documento que hay delante. Lo cuenta quien lo abrió, que es
  // el único que ve los bytes: por encima de cierto tamaño la vista previa del
  // sello deja de recalcularse sola (ID-109).
  const [sizeBytes, setSizeBytes] = useState<number | null>(null);
  // Un gesto sobre el recuadro está en curso. Sólo lo mira la vista previa: es
  // lo que congela la vista anterior en vez de pagar un ciclo por fotograma.
  const [gesturing, setGesturing] = useState(false);
  // La página que se está mirando: la sigue eligiendo el visor, y el panel la
  // necesita para elegir la cara del botón de sellar (#194).
  const [viewedPage, setViewedPage] = useState(1);
  // El botón de sellar vive en el panel y actúa en el visor, que es quien
  // tiene el `viewport` para medir la posición estándar del recuadro (#194).
  const [placementRequest, setPlacementRequest] = useState<{
    action: "seal" | "unseal";
  } | null>(null);
  const {
    certificate,
    lookForCertificates,
    installed,
    installCertificate,
    removeCertificate,
    emptyStore,
    chooseCertificate,
  } = useCertificateSearch(certificates);
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
  const [signature, setSignature] = useVisibleSignature(initialSignature, chosen);
  const signing = useSigning(signer);
  const { settings, changeSettings, chooseDestination, rubric, rubricFailure, chooseRubric } =
    usePreferencesState(preferences, rubrics, covered);
  // Mientras los ajustes se leen todavía no se sabe, y lo guardado por omisión es recordar.
  const rememberActivity = settings?.rememberActivity ?? true;
  const documents = useDocuments(recents, picker, rememberActivity);
  const activeId = documents.active?.id ?? null;
  // Con la actividad apagada no se enseñan los recientes que ya hubiera guardados.
  const visibleRecents = rememberActivity ? documents.recents : NO_RECENTS;
  const previousSignatures = usePreviousSignatures(signer, activeId);
  const { destination, singleDestinationId, chooseSingleDestination } = useDestination(
    destinations,
    activeId,
    settings?.destination ?? null,
    signing.state.kind,
  );
  const { i18n } = useTranslation();
  // El instante del recuadro **es estado, no un reloj**: se fija al abrir el
  // documento y no vuelve a correr. Recalcularlo en cada pintada haría que la
  // vista previa enseñara una hora y se estampara otra, que es la diferencia
  // entre enseñar el PDF que se va a firmar y enseñar uno parecido.
  //
  // El **formato** sí se rehace al cambiar de idioma: la hora es la misma, y
  // solo cambia cómo se escribe.
  const [signingInstant, setSigningInstant] = useState(() => new Date());
  const signedAt = useMemo(
    () => formatSignedAt(signingInstant, i18n.language),
    [signingInstant, i18n.language],
  );

  const {
    placing,
    setPlacing,
    pageChoice,
    placement,
    rememberPlacement,
    choosePages,
    changePageChoice,
    placeOnViewedPage,
  } = usePlacementControls(pdf, documents.place, viewedPage);

  const signatureOn = signature.enabled && pdf !== null;
  useEffect(() => {
    if (signatureOn && placing.rect === null) placeOnViewedPage();
  }, [signatureOn, placing.rect, placeOnViewedPage]);

  // Se lee aquí, y no en la vista previa, porque es asíncrono y el ciclo de la
  // firma se decide con la orden ya armada.
  const boxPage = placement === null ? null : (firstSealedPage(placement) ?? 1);
  const geometry = usePageGeometry(pdf, boxPage);

  // Cambiar de pestaña repone el recuadro que guarda: uno ya abierto vuelve a su
  // página y posición, y uno nuevo arranca donde toque, no donde lo dejó otro.
  useEffect(() => {
    const active = documents.active;
    if (!active) {
      setPdf(null);
      setPdfFailure(null);
      setSizeBytes(null);
      setPlacing({ rect: null, sets: NO_PAGE_SETS, choice: "single" });
      return;
    }
    let current = true;
    void pdfs.open(active).then((opened) => {
      if (!current) return;
      setPdf(opened.ok ? opened.pdf : null);
      setPdfFailure(opened.ok ? null : opened.failure);
      setSizeBytes(opened.ok ? opened.sizeBytes : null);
      // Se guarda una sola colocación, la firmada; las otras dos opciones se siembran de ella.
      setPlacing(placingFrom(active.placement, opened.ok ? opened.pdf.pageCount : 0));
      setViewedPage(firstSealedPage(active.placement) ?? 1);
      // Documento nuevo, hora nueva: la del anterior lleva parada desde que se
      // abrió, y el recuadro de este llevaría estampada una hora vieja.
      setSigningInstant(new Date());
    });
    return () => {
      current = false;
    };
  }, [documents.active, pdfs, setPlacing]);

  const { dropNotice } = useDropNotices(drops, documents.accept, documents.enter, activeId);

  const { signedHere, openFailure, openSigned, signAgain } = useSignedSummary(
    signing,
    activeId,
    documents.reopen,
  );
  const { failedHere } = useSigningFailure(signing, activeId);

  // Sin el `catch`, el rechazo quedaría sin dueño; se cuenta en el visor.
  const reportingFailure = (command: () => Promise<void>) => () => {
    command().catch((thrown: unknown) => setPdfFailure(classify(thrown)));
  };
  const openDocument = reportingFailure(documents.open);
  const clearRecents = reportingFailure(documents.clearRecents);

  const signFlow = useSignFlow({
    pdf,
    activeDocument: documents.active,
    placement,
    geometry,
    boxPage,
    signature,
    rubric,
    signedAt,
    language: i18n.resolvedLanguage ?? i18n.language,
    chosen,
    signer,
    stamps,
    sizeBytes,
    gesturing,
    singleDestinationId,
    previousSignatures: previousSignatures.signatures,
    startSigning: signing.start,
  });
  const { stamp, sign } = signFlow;

  const modalOpen =
    dialog !== null || signFlowPromptOpen(signFlow) || signing.state.kind === "running";
  const canOpen = !covered && view === null && !modalOpen;
  useOpenShortcut(openDocument, canOpen);

  const anchor = menuAnchor ?? menuAnchorFor(navigator.userAgent);
  const warningVisible = hasAttention && view !== "status";
  const openHelp = () => void externalDestinations.open("discussions");
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
            <StatusView
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
              installedCertificates={installed}
              onInstallCertificate={installCertificate}
              onRemoveCertificate={removeCertificate}
              onEmptyStore={emptyStore}
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
              signingLocked={signing.state.kind === "running"}
              withOpenButton={anchor !== "titlebar"}
            />
          )
        }
        viewer={
          <DocumentViewer
            pdf={pdf}
            stamped={stamp.pdf}
            stampFrozen={stamp.state.kind === "frozen"}
            onGesture={setGesturing}
            placement={placement}
            canPlace={signature.enabled}
            onPlace={rememberPlacement}
            pageChoice={pageChoice}
            onPageChange={setViewedPage}
            placementRequest={placementRequest}
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
            stamp={stamp.state}
            rubricGap={stamp.rubricGap}
            onComposeStamp={stamp.compose}
            onOpenHelp={() => void externalDestinations.open("discussions")}
          />
        }
        panel={
          signedHere !== null ? (
            // Firmado: la columna derecha cambia de contenido, no de sitio. Es
            // el único acuse de recibo que recibe quien firma, así que se monta
            // en cuanto la postfirma devuelve el documento.
            // Solo mientras siga activo **el documento que se firmó**: el
            // recuento de páginas sale del PDF abierto, y con otro delante sería
            // el nombre de un fichero con las páginas de otro. Sin documento
            // activo tampoco se monta, o quedaría una tercera columna al lado
            // del visor vacío (ID-51).
            <SignedPanel
              document={{
                name: signedHere.document.name,
                pages: pdf?.pageCount ?? null,
                // El tamaño lo trae la postfirma, que lo supo al escribir el
                // fichero: aquí no se recalcula nada (ID-77).
                sizeBytes: signedHere.document.sizeBytes,
              }}
              signedAt={signingInstant}
              signature={signature}
              placement={placement}
              destination={
                destination ?? { folder: settings?.destination ?? "", name: null, writable: true }
              }
              onOpenDocument={() => openSigned(() => opener.openDocument())}
              onOpenFolder={() => openSigned(() => opener.openFolder())}
              onSignAgain={signAgain}
              failure={openFailure}
              onOpenHelp={() => void externalDestinations.open("discussions")}
            />
          ) : pdf && documents.active ? (
            <SigningPanel
              document={{
                id: documents.active.id,
                name: documents.active.name,
                pages: pdf.pageCount,
                sizeBytes,
              }}
              previousSignatures={previousSignatures}
              certificate={certificate}
              onChooseCertificate={chooseCertificate}
              onRetryCertificates={() => void lookForCertificates()}
              onChooseModule={() => void lookForCertificates()}
              signature={signature}
              onChangeSignature={setSignature}
              placement={placement}
              pageSets={placing.sets}
              onChoosePages={choosePages}
              pageChoice={pageChoice}
              onChangePageChoice={changePageChoice}
              viewedPage={viewedPage}
              onSeal={() => setPlacementRequest({ action: "seal" })}
              onUnseal={() => setPlacementRequest({ action: "unseal" })}
              rubric={rubric}
              rubricFailure={rubricFailure}
              onChooseRubric={() => void chooseRubric()}
              destination={
                destination ?? { folder: settings?.destination ?? "", name: null, writable: true }
              }
              onChangeDestination={() => void chooseSingleDestination()}
              onSign={() => void sign()}
              signing={signing.state.kind === "running"}
              onOpenHelp={() => void externalDestinations.open("discussions")}
              failure={failedHere?.failure ?? null}
              onBack={signing.cancel}
              onEmptyStore={() => void emptyStore()}
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
      <SignFlowPrompts flow={signFlow} locale={i18n.resolvedLanguage ?? i18n.language} />
      {signing.state.kind === "running" && <SigningProgressDialog stage={signing.state.stage} />}
    </>
  );
}
