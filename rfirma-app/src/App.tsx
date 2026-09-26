import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSignedAt, type PageGeometry, placingFrom } from "./App.signingOrder";
import { useCertificateSearch } from "./App.useCertificateSearch";
import { useDropNotices } from "./App.useDropNotices";
import { usePlacementControls } from "./App.usePlacementControls";
import { useDestination, usePreferencesState } from "./App.usePreferencesState";
import { useSignedSummary } from "./App.useSignedSummary";
import { useSignFlow } from "./App.useSignFlow";
import { useSigningFailure } from "./App.useSigningFailure";
import { useStartupNotices } from "./App.useStartupNotices";
import { AboutDialog } from "./about/AboutDialog";
import type { ExternalDestinationOpener } from "./desktop/externalDestination";
import { unavailableExternalDestinationOpener } from "./desktop/externalDestination";
import { DocumentTabs } from "./documents/DocumentTabs";
import type { DocumentDrops } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import { RecentsSection } from "./documents/RecentRows";
import type { RecentsStore } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { classify } from "./errors/classify";
import { PreferencesView } from "./preferences/PreferencesView";
import type { PreferencesStore } from "./preferences/preferences";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import { NotificationStrip } from "./shell/NotificationStrip";
import type { CertificateStore } from "./signing/certificate";
import type { DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import type { RubricPicker } from "./signing/rubric";
import { SignedPanel } from "./signing/SignedPanel";
import { SigningPanel } from "./signing/SigningPanel";
import { SigningProgressDialog } from "./signing/SigningProgressDialog";
import type { StampComposer } from "./signing/stampPreview";
import { UnregisteredSignaturesDialog } from "./signing/UnregisteredSignaturesDialog";
import { UnsealedPagesDialog } from "./signing/UnsealedPagesDialog";
import { useSigning } from "./signing/useSigning";
import type { VisibleSignature } from "./signing/visibleSignature";
import { StatusView } from "./status/StatusView";
import { memoryStatus, type StatusPort } from "./status/status";
import type { VersionCheck } from "./updates/newVersion";
import { DocumentViewer } from "./viewer/DocumentViewer";
import type { PdfDocument } from "./viewer/pdf";
import { firstSealedPage, NO_PAGE_SETS } from "./viewer/signatureBox";
import type { DocumentFailure, PdfSource } from "./viewer/source";

type OpenDialog = "about" | null;
type ActiveView = "status" | "preferences" | null;

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
  /** Dónde va el menú. Por omisión, lo que diga la plataforma. */
  menuAnchor?: MenuAnchor;
  /** Quien abre destinos externos fuera de la aplicación. Ver [`ExternalDestinationOpener`]. */
  externalDestinations?: ExternalDestinationOpener;
  /** Quien lee y reevalúa las señales del panel de estado. Ver [`StatusPort`]. */
  status?: StatusPort;
  /**
   * Se llama una vez montada, con un asa hacia sus propias vistas. Solo lo usa
   * `main.tsx`, para que el menú del asistente del primer arranque
   * (`setup/SetupWizard.tsx`) pueda abrir Estado, Preferencias y Acerca de en
   * esta misma instancia en vez de duplicarlas.
   */
  onReady?: (handle: AppHandle) => void;
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
  menuAnchor,
  externalDestinations = unavailableExternalDestinationOpener(),
  status = memoryStatus(),
  onReady,
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
  const [rememberedSignature, setSignature] = useState<VisibleSignature>(initialSignature);
  const {
    certificate,
    lookForCertificates,
    installed,
    installCertificate,
    removeCertificate,
    chooseCertificate,
  } = useCertificateSearch(certificates);
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
  const signature = useMemo(
    () => (chosen === null ? { ...rememberedSignature, enabled: false } : rememberedSignature),
    [chosen, rememberedSignature],
  );
  const signing = useSigning(signer);
  const { settings, changeSettings, chooseDestination, rubric, rubricFailure, chooseRubric } =
    usePreferencesState(preferences, rubrics);
  // Mientras los ajustes se leen todavía no se sabe, y lo guardado por omisión es recordar.
  const documents = useDocuments(recents, picker, settings?.rememberActivity ?? true);
  const activeId = documents.active?.id ?? null;
  const { destination, singleDestinationId, chooseSingleDestination } = useDestination(
    destinations,
    activeId,
    settings?.destination ?? null,
    signing.state.kind,
  );
  const { t, i18n } = useTranslation();
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
  const [geometry, setGeometry] = useState<PageGeometry | null>(null);
  useEffect(() => {
    if (pdf === null || boxPage === null) {
      setGeometry(null);
      return;
    }
    let current = true;
    void pdf.getPage(boxPage).then((page) => {
      if (current) setGeometry({ page: boxPage, view: page.view, rotate: page.rotate });
    });
    return () => {
      current = false;
    };
  }, [pdf, boxPage]);

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

  const {
    stamp,
    sign,
    sealLossPrompt,
    setSealLossPrompt,
    signAnyway,
    unregisteredPrompt,
    setUnregisteredPrompt,
    signWithUnregisteredSignatures,
  } = useSignFlow({
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
    startSigning: signing.start,
  });

  // Olvidar la actividad es una sola promesa al usuario del ordenador
  // compartido: se van los recientes y el certificado a la vez (ID-34).
  const forgetActivity = async () => {
    // Los recientes de la ventana se vacían **aunque el borrado del disco
    // falle**: lo que promete el rótulo es que dejen de estar, y quedarse a
    // medias sería enseñarlos como si nada hubiera pasado.
    // El centinela envuelve el valor en vez de serlo: un rechazo con `null`
    // —el tipo capturado es `unknown`— volvería a ser el `catch {}` vacío que
    // esta función existe para quitar de en medio.
    let failure: { thrown: unknown } | null = null;
    try {
      await preferences.forgetActivity();
    } catch (thrown) {
      failure = { thrown };
    }
    try {
      await documents.forgetAll();
    } catch (thrown) {
      // El primero que falló es el que se cuenta: si el disco ya había dicho
      // que no, ese rechazo es el que explica lo que ha pasado, y perderlo
      // aquí dejaría el fallo de verdad sin llegar a *Privacidad*.
      failure ??= { thrown };
    }
    // El fallo se cuenta **después** de vaciar la ventana, y lo cuenta
    // Preferencias en su sección de Privacidad (ID-70): lo que no puede pasar
    // es que el borrado del disco falle y nadie lo diga.
    if (failure !== null) throw failure.thrown;
  };

  return (
    <>
      <MainWindow
        menuAnchor={menuAnchor ?? menuAnchorFor(navigator.userAgent)}
        hasAttention={hasAttention}
        onOpenStatus={() => setView("status")}
        onOpenPreferences={() => setView("preferences")}
        onOpenHelp={() => void externalDestinations.open("discussions")}
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
              onForgetActivity={forgetActivity}
              installedCertificates={installed}
              onInstallCertificate={installCertificate}
              onRemoveCertificate={removeCertificate}
              onClose={() => setView(null)}
            />
          ) : null
        }
        notification={
          // El único inquilino de la franja (ID-354). La acción no descarga
          // nada: lleva a *Acerca de*, que es donde están las órdenes de alta
          // del repositorio (ID-181), y así el `opener:deny-open-url` del
          // ID-85 sigue sin hacer falta.
          newVersion !== null && !versionDismissed && (settings?.notifyNewVersion ?? true) ? (
            <NotificationStrip
              message={t("notifications.newVersion.message", { version: newVersion.version })}
              action={{
                label: t("notifications.newVersion.action"),
                onSelect: () => setDialog("about"),
              }}
              dismissLabel={t("actions.dismiss")}
              onDismiss={() => setVersionDismissed(true)}
            />
          ) : null
        }
        tabs={
          <DocumentTabs
            tabs={documents.tabs}
            activeId={activeId}
            recents={documents.recents}
            onActivate={documents.activate}
            onClose={documents.close}
            onOpen={openDocument}
            onSelectRecent={documents.select}
            onClearRecents={clearRecents}
            signingLocked={signing.state.kind === "running"}
          />
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
                  recents={documents.recents}
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
                name: documents.active.name,
                pages: pdf.pageCount,
                sizeBytes,
                signatures: null,
              }}
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
            />
          ) : null
        }
      />
      {dialog === "about" && (
        <AboutDialog
          version={__APP_VERSION__}
          newVersion={newVersion}
          onClose={() => setDialog(null)}
        />
      )}
      {unregisteredPrompt !== null && (
        <UnregisteredSignaturesDialog
          onConfirm={() => void signWithUnregisteredSignatures()}
          onCancel={() => setUnregisteredPrompt(null)}
        />
      )}
      {sealLossPrompt !== null && (
        <UnsealedPagesDialog
          fallen={sealLossPrompt.fallen}
          chosen={sealLossPrompt.chosen}
          onConfirm={() => void signAnyway()}
          onCancel={() => setSealLossPrompt(null)}
        />
      )}
      {signing.state.kind === "running" && <SigningProgressDialog stage={signing.state.stage} />}
    </>
  );
}
