import { useCallback, useEffect, useState } from "react";
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
import type { Rubric, RubricFailure, RubricPicker } from "./signing/rubric";
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
          pdf && documents.active ? (
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
              onSign={() => void signing.start()}
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
          busy={false}
          onSubmit={(pin) => void signing.submitPin(pin)}
          onCancel={signing.cancel}
        />
      )}
    </>
  );
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
