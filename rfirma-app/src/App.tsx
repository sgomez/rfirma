import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AboutDialog } from "./about/AboutDialog";
import { DocumentTray } from "./documents/DocumentTray";
import type { DocumentPicker } from "./documents/picker";
import type { RecentsStore } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { PreferencesDialog } from "./preferences/PreferencesDialog";
import type { Preferences, PreferencesStore } from "./preferences/preferences";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { SigningBackend } from "./signing/flow";
import { PinDialog } from "./signing/PinDialog";
import { base64Of, type Rubric, type RubricFailure, type RubricPicker } from "./signing/rubric";
import { SignedPanel } from "./signing/SignedPanel";
import { type CertificateState, SigningPanel } from "./signing/SigningPanel";
import { SigningProgressDialog } from "./signing/SigningProgressDialog";
import { useSigning } from "./signing/useSigning";
import {
  DEFAULT_VISIBLE_SIGNATURE,
  type Layer2Composer,
  type VisibleSignature,
} from "./signing/visibleSignature";
import { DocumentViewer } from "./viewer/DocumentViewer";
import type { PdfDocument } from "./viewer/pdf";
import type { SignaturePlacement } from "./viewer/signatureBox";
import type { PdfSource } from "./viewer/source";

type OpenDialog = "preferences" | "about" | null;

interface AppProps {
  recents: RecentsStore;
  picker: DocumentPicker;
  preferences: PreferencesStore;
  /** De dónde salen los bytes del PDF que se pinta. Ver [`PdfSource`]. */
  pdfs: PdfSource;
  /**
   * Las carpetas de destino, por su nombre. Bajo el arenero hay exactamente
   * una, la de documentos del usuario (ADR-0011).
   */
  destinations: readonly string[];
  /** Los certificados de los tokens conectados. Ver [`CertificateStore`]. */
  certificates: CertificateStore;
  /** Por dónde entra la rúbrica, ya normalizada. Ver [`RubricPicker`]. */
  rubrics: RubricPicker;
  /** Quien compone el texto del recuadro. Ver [`Layer2Composer`]. */
  composer: Layer2Composer;
  /** Quien ejecuta las tres etapas de la firma. Ver [`SigningBackend`]. */
  signer: SigningBackend;
  /** Dónde va el menú de dos entradas. Por omisión, lo que diga la plataforma. */
  menuAnchor?: MenuAnchor;
}

/**
 * La composición: quién habla con quién.
 *
 * Todo lo que toca el disco entra por parámetro —los recientes, el portal y los
 * ajustes—, así que la aplicación entera se puede pintar en una prueba sin
 * backend. Quien elige las implementaciones de verdad es `main.tsx`.
 *
 * Los diálogos se montan **sobre** la ventana y no la desmontan: no hay
 * navegación, y el estado de la bandeja sigue vivo debajo.
 */
export function App({
  recents,
  picker,
  preferences,
  pdfs,
  destinations,
  certificates,
  rubrics,
  composer,
  signer,
  menuAnchor,
}: AppProps) {
  const [dialog, setDialog] = useState<OpenDialog>(null);
  const [pdf, setPdf] = useState<PdfDocument | null>(null);
  // Dónde va la firma visible. Vive aquí y no en el visor porque el panel de
  // firma —su sub-issue— es quien dirá qué se pinta dentro del recuadro, y los
  // dos tienen que estar mirando el mismo.
  const [placement, setPlacement] = useState<SignaturePlacement | null>(null);
  const [settings, setSettings] = useState<Preferences | null>(null);
  const [certificate, setCertificate] = useState<CertificateState>({ kind: "loading" });
  const [signature, setSignature] = useState<VisibleSignature>(DEFAULT_VISIBLE_SIGNATURE);
  const [rubric, setRubric] = useState<Rubric | null>(null);
  const [rubricFailure, setRubricFailure] = useState<RubricFailure | null>(null);
  const signing = useSigning(signer);
  // Mientras los ajustes se leen todavía no se sabe, y lo guardado por omisión
  // es recordar; el primer documento no se puede abrir antes de esa lectura.
  const documents = useDocuments(recents, picker, settings?.rememberActivity ?? true);
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

  useEffect(() => {
    let current = true;
    preferences.read().then((read) => {
      if (current) setSettings(read);
    });
    return () => {
      current = false;
    };
  }, [preferences]);

  // El documento activo, abierto para pintarlo. Cambiar de documento tira el
  // recuadro: la posición es de este documento y de esta página.
  useEffect(() => {
    const active = documents.active;
    if (!active) {
      setPdf(null);
      setPlacement(null);
      return;
    }
    let current = true;
    void pdfs.open(active).then((opened) => {
      if (!current) return;
      setPdf(opened);
      setPlacement(null);
      // Documento nuevo, hora nueva: la del anterior lleva parada desde que se
      // abrió, y el recuadro de este llevaría estampada una hora vieja.
      setSigningInstant(new Date());
    });
    return () => {
      current = false;
    };
  }, [documents.active, pdfs]);

  // Los certificados se buscan al arrancar y cada vez que alguien pide volver
  // a buscar: una tarjeta insertada tarde es el caso corriente, no la excepción.
  const lookForCertificates = useCallback(async () => {
    setCertificate({ kind: "loading" });
    const found = await certificates.list();
    setCertificate(chosenFrom(found));
  }, [certificates]);

  useEffect(() => {
    void lookForCertificates();
  }, [lookForCertificates]);

  // La rúbrica se comprueba y se normaliza **al elegirla**, con el panel
  // abierto, y nunca al firmar (ADR-0012): el fallo se cuenta aquí.
  const chooseRubric = async () => {
    const choice = await rubrics.choose();
    if (choice === null) return;
    if ("failure" in choice) {
      setRubricFailure(choice.failure);
      return;
    }
    setRubricFailure(null);
    setRubric(choice.rubric);
  };

  const changeSettings = async (next: Preferences) => {
    setSettings(next);
    await preferences.save(next);
  };

  /**
   * La firma: se arma la orden con lo que hay decidido y se manda entera.
   *
   * La `MediaBox` y la `/Rotate` salen de la página abierta porque el backend
   * **no lee PDFs**: la conversión del recuadro a puntos PAdES es suya
   * (`signing::placement`, con la guardia del ID-22), pero los datos de la
   * página los tiene `pdf.js`.
   */
  const sign = async () => {
    const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
    if (pdf === null || documents.active === null || placement === null || chosen === null) {
      // El botón ya está apagado sin certificado en vigor; aquí solo se
      // estrecha el tipo, y callar es mejor que fabricar una orden a medias.
      return;
    }
    const page = await pdf.getPage(placement.page);
    await signing.start(chosen, {
      document: documents.active.path,
      certificate: chosen.label,
      placement: {
        page: placement.page,
        mediaBox: page.view,
        rotation: page.rotate,
        rect: [placement.rect.x0, placement.rect.y0, placement.rect.x1, placement.rect.y1],
      },
      fields: signature.fields,
      reason: signature.reason,
      signedAt,
      // La rúbrica solo viaja si además está marcada: tener una imagen
      // guardada no es quererla dentro del recuadro.
      rubric: signature.rubric && rubric !== null ? base64Of(rubric) : null,
      language: i18n.resolvedLanguage ?? i18n.language,
    });
  };

  // Olvidar la actividad es una sola promesa al usuario del ordenador
  // compartido: se van los recientes y el certificado a la vez (ID-34).
  const forgetActivity = async () => {
    await preferences.forgetActivity();
    await documents.forgetAll();
  };

  return (
    <>
      <MainWindow
        status={documents.active?.badge ?? null}
        menuAnchor={menuAnchor ?? menuAnchorFor(navigator.userAgent)}
        onOpenPreferences={() => setDialog("preferences")}
        onOpenAbout={() => setDialog("about")}
        tray={
          <DocumentTray
            recents={documents.recents}
            activePath={documents.active?.path ?? null}
            onOpen={() => void documents.open()}
            onSelect={documents.select}
            onForget={(path) => void documents.forget(path)}
          />
        }
        viewer={
          <DocumentViewer
            pdf={pdf}
            placement={placement}
            onPlace={setPlacement}
            onOpen={() => void documents.open()}
          />
        }
        panel={
          signing.state.kind === "signed" ? (
            // Firmado: la columna derecha cambia de contenido, no de sitio. Es
            // el único acuse de recibo que recibe quien firma, así que se monta
            // en cuanto la postfirma devuelve el documento.
            <SignedPanel
              document={{ name: signing.state.document.name, pages: pdf?.pageCount ?? null }}
              onSignAnother={signing.signAnother}
            />
          ) : pdf && documents.active ? (
            <SigningPanel
              document={{
                name: documents.active.name,
                pages: pdf.pageCount,
                sizeBytes: null,
                signatures: null,
              }}
              certificate={certificate}
              onChooseCertificate={() => void lookForCertificates()}
              onRetryCertificates={() => void lookForCertificates()}
              onChooseModule={() => void lookForCertificates()}
              signature={signature}
              onChangeSignature={setSignature}
              page={placement?.page ?? null}
              rubric={rubric}
              rubricFailure={rubricFailure}
              onChooseRubric={() => void chooseRubric()}
              composer={composer}
              destination={{
                folder: settings?.destination ?? destinations[0] ?? "",
                writable: true,
              }}
              onChangeDestination={() => setDialog("preferences")}
              signedAt={signedAt}
              onSign={() => void sign()}
              signing={signing.state.kind === "running" || signing.state.kind === "pin"}
              failure={
                signing.state.kind === "failed"
                  ? {
                      situation: signing.state.failure.situation,
                      detail: signing.state.failure.detail,
                    }
                  : null
              }
            />
          ) : null
        }
      />
      {dialog === "preferences" && settings !== null && (
        <PreferencesDialog
          preferences={settings}
          destinations={destinations}
          onChange={(next) => void changeSettings(next)}
          onForgetActivity={() => void forgetActivity()}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog === "about" && (
        <AboutDialog version={__APP_VERSION__} onClose={() => setDialog(null)} />
      )}
      {signing.state.kind === "running" && <SigningProgressDialog stage={signing.state.stage} />}
      {signing.state.kind === "pin" && certificate.kind === "chosen" && (
        <PinDialog
          certificate={certificate.certificate}
          failure={signing.state.failure}
          onSubmit={(pin) => void signing.submitPin(pin)}
          onCancel={signing.cancel}
        />
      )}
    </>
  );
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
function formatSignedAt(instant: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "medium",
  }).format(instant);
}

/**
 * Con qué certificado se firma, a partir de los que hay.
 *
 * Con uno solo no se pregunta: elegir entre una cosa no es elegir. Con varios,
 * el panel enseña «Elegir certificado» y el diálogo que los lista es de su
 * propio sub-issue; hasta entonces se queda en `unchosen`, que es la verdad.
 */
function chosenFrom(found: readonly Certificate[]): CertificateState {
  const [first] = found;
  if (first === undefined) return { kind: "empty" };
  if (found.length === 1) return { kind: "chosen", certificate: first };
  return { kind: "unchosen" };
}
