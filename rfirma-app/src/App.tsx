//! El componente `App` de la ventana principal: compone los `App.use*`, reparte su estado por el árbol y entrega el `AppHandle` que abre sus vistas desde fuera.

import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { forgetActivity } from "./App.forgetActivity";
import type { MainWindowPorts } from "./App.ports";
import { SignFlowPrompts, signFlowPromptOpen } from "./App.SignFlowPrompts";
import { useCertificateSearch } from "./App.useCertificateSearch";
import { useDropNotices } from "./App.useDropNotices";
import { useNativeTitlebar } from "./App.useNativeTitlebar";
import { useOpenShortcut } from "./App.useOpenShortcut";
import { usePageGeometry } from "./App.usePageGeometry";
import { useDestination, usePreferencesState } from "./App.usePreferencesState";
import { usePreviousSignatures } from "./App.usePreviousSignatures";
import { useSignedSummary } from "./App.useSignedSummary";
import { useSignFlow } from "./App.useSignFlow";
import { useSigningFailure } from "./App.useSigningFailure";
import { useStartupNotices } from "./App.useStartupNotices";
import { useViewedSignatures } from "./App.useViewedSignatures";
import { useVisibleSignature } from "./App.useVisibleSignature";
import { AboutDialog } from "./about/AboutDialog";
import { DocumentTabs } from "./documents/DocumentTabs";
import { isAPdf } from "./documents/document";
import { RecentsSection } from "./documents/RecentRows";
import type { RecentDocument } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { classify } from "./errors/classify";
import { firstSealedPage } from "./placement/pageSets";
import { type PlacedDocument, usePlacement } from "./placement/usePlacement";
import { PreferencesView } from "./preferences/PreferencesView";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import { SignedPanel } from "./signing/SignedPanel";
import { SigningPanel } from "./signing/SigningPanel";
import { SigningProgressDialog } from "./signing/SigningProgressDialog";
import { formatSignedAt } from "./signing/signedAt";
import { useSigning } from "./signing/useSigning";
import type { VisibleSignature } from "./signing/visibleSignature";
import { StatusView } from "./status/StatusView";
import { InstallUpdateDialog } from "./updates/InstallUpdateDialog";
import { NewVersionStrip } from "./updates/NewVersionStrip";
import { DocumentViewer } from "./viewer/DocumentViewer";
import type { PdfDocument } from "./viewer/pdf";
import { standardRectOnPageOf } from "./viewer/signatureBox";
import type { DocumentFailure } from "./viewer/source";

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

  const [pdf, setPdf] = useState<PdfDocument | null>(null);
  // Por qué no se pudo pintar el último documento que se eligió. Vive al lado
  // del PDF y no dentro del visor porque lo produce quien abre, y el visor solo
  // lo enseña.
  const [pdfFailure, setPdfFailure] = useState<DocumentFailure | null>(null);
  // Cuánto ocupa el documento que hay delante. Lo cuenta quien lo abrió, que es
  // el único que ve los bytes: por encima de cierto tamaño la vista previa del
  // sello deja de recalcularse sola.
  const [sizeBytes, setSizeBytes] = useState<number | null>(null);
  // Un gesto sobre el recuadro está en curso. Sólo lo mira la vista previa: es
  // lo que congela la vista anterior en vez de pagar un ciclo por fotograma.
  const [gesturing, setGesturing] = useState(false);
  // Un valor nuevo por cada apertura, también del mismo documento: es lo que
  // repone la colocación.
  const [placedDocument, setPlacedDocument] = useState<PlacedDocument | null>(null);
  // El botón de sellar vive en el panel y actúa en el visor, que es quien
  // tiene el `viewport` para medir la posición estándar del recuadro.
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
    usePreferencesState(preferences, rubrics, covered, windowTheme);
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

  const standardRectOn = useMemo(() => (pdf === null ? null : standardRectOnPageOf(pdf)), [pdf]);
  const {
    placing,
    pageMode,
    placement,
    viewedPage,
    viewPage,
    moveBox,
    sealPage,
    unsealPage,
    choosePages,
    changePageMode,
    placeOnViewedPage,
  } = usePlacement({
    document: placedDocument,
    standardRectOn,
    onChange: documents.place,
  });

  const signatureOn = signature.enabled && pdf !== null;
  useEffect(() => {
    if (signatureOn && placing.rect === null) placeOnViewedPage();
  }, [signatureOn, placing.rect, placeOnViewedPage]);

  // Se lee aquí, y no en la vista previa, porque es asíncrono y el ciclo de la
  // firma se decide con la orden ya armada.
  const boxPage = placement === null ? null : (firstSealedPage(placement) ?? 1);
  const geometry = usePageGeometry(pdf, boxPage);

  useEffect(() => {
    const active = documents.active;
    if (!active || !isAPdf(active)) {
      setPdf(null);
      setPdfFailure(null);
      setSizeBytes(null);
      setPlacedDocument(null);
      return;
    }
    let current = true;
    void pdfs.open(active).then((opened) => {
      if (!current) return;
      setPdf(opened.ok ? opened.pdf : null);
      setPdfFailure(opened.ok ? null : opened.failure);
      setSizeBytes(opened.ok ? opened.sizeBytes : null);
      setPlacedDocument({
        placement: active.placement,
        pageCount: opened.ok ? opened.pdf.pageCount : 0,
      });
      // Documento nuevo, hora nueva: la del anterior lleva parada desde que se
      // abrió, y el recuadro de este llevaría estampada una hora vieja.
      setSigningInstant(new Date());
    });
    return () => {
      current = false;
    };
  }, [documents.active, pdfs]);

  const viewedSignatures = useViewedSignatures(signer, activeId);
  const { dropNotice } = useDropNotices(
    drops,
    documents.accept,
    documents.enter,
    activeId,
    viewedSignatures.view,
  );

  const { signedHere, signatures, findings, openFailure, openSigned, signAgain } = useSignedSummary(
    signing,
    activeId,
    documents.reopen,
    signer,
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
    previousSignatures,
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
            onMove={moveBox}
            onTrace={sealPage}
            onSeal={sealPage}
            onUnseal={unsealPage}
            onPageChange={viewPage}
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
            withoutPreview={
              documents.active && !isAPdf(documents.active) ? documents.active.name : null
            }
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
            // del visor vacío.
            <SignedPanel
              documentName={signedHere.document.name}
              signedAt={signingInstant}
              signatures={signatures}
              findings={findings}
              destination={
                destination ?? { folder: settings?.destination ?? "", name: null, writable: true }
              }
              onOpenDocument={() => openSigned(() => opener.openDocument())}
              onOpenFolder={() => openSigned(() => opener.openFolder())}
              onSign={signAgain}
              onChangeDestination={() => void chooseSingleDestination()}
              failure={openFailure}
              onOpenHelp={() => void externalDestinations.open("discussions")}
            />
          ) : viewedSignatures.viewing && documents.active ? (
            <SignedPanel
              documentName={documents.active.name}
              signatures={
                viewedSignatures.reading.kind === "read" ? viewedSignatures.reading.signatures : []
              }
              findings={
                viewedSignatures.reading.kind === "read" ? viewedSignatures.reading.findings : []
              }
              format={
                viewedSignatures.reading.kind === "read" ? viewedSignatures.reading.format : "pades"
              }
              signable={isAPdf(documents.active)}
              reading={viewedSignatures.reading.kind === "reading"}
              readFailure={
                viewedSignatures.reading.kind === "failed" ? viewedSignatures.reading.failure : null
              }
              destination={{
                folder: visibleRecents.find((row) => row.id === activeId)?.folder ?? "",
                name: documents.active.name,
                writable: true,
              }}
              onOpenDocument={() => openSigned(() => opener.openDocument(documents.active?.id))}
              onOpenFolder={() => openSigned(() => opener.openFolder(documents.active?.id))}
              onSign={viewedSignatures.stopViewing}
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
              pageMode={pageMode}
              onChangePageMode={changePageMode}
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
