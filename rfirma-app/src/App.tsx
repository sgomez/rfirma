import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AboutDialog } from "./about/AboutDialog";
import { DocumentTray } from "./documents/DocumentTray";
import type { DocumentDrops } from "./documents/drops";
import type { DocumentPicker } from "./documents/picker";
import type { RecentsStore } from "./documents/recents";
import { useDocuments } from "./documents/useDocuments";
import { classify, type NamedFailure } from "./errors/classify";
import { PreferencesDialog } from "./preferences/PreferencesDialog";
import type { Preferences, PreferencesStore } from "./preferences/preferences";
import { applyTheme } from "./preferences/theme";
import { MainWindow } from "./shell/MainWindow";
import { type MenuAnchor, menuAnchorFor } from "./shell/menuAnchor";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { Destination, DestinationSource, SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import { PinDialog } from "./signing/PinDialog";
import { base64Of, type Rubric, type RubricFailure, type RubricPicker } from "./signing/rubric";
import { SignedPanel } from "./signing/SignedPanel";
import { type CertificateState, SigningPanel } from "./signing/SigningPanel";
import { SigningProgressDialog } from "./signing/SigningProgressDialog";
import { acknowledgementFor, useSigning } from "./signing/useSigning";
import {
  DEFAULT_VISIBLE_SIGNATURE,
  type Layer2Composer,
  type VisibleSignature,
} from "./signing/visibleSignature";
import { DocumentViewer } from "./viewer/DocumentViewer";
import type { PdfDocument } from "./viewer/pdf";
import type { SignaturePlacement } from "./viewer/signatureBox";
import type { DocumentFailure, PdfSource } from "./viewer/source";

type OpenDialog = "preferences" | "about" | null;

/**
 * Un aviso del arrastre, atado al documento del que habla.
 *
 * `about` es el identificador del documento que tenía que estar delante para
 * que el aviso siga significando algo: el que se acaba de abrir cuando se
 * soltaron varios, o el que ya estaba cuando lo soltado no se pudo abrir.
 */
interface DropNotice {
  about: string | null;
  failure: DocumentFailure;
}

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
  /** Quien compone el texto del recuadro. Ver [`Layer2Composer`]. */
  composer: Layer2Composer;
  /** Quien ejecuta las tres etapas de la firma. Ver [`SigningBackend`]. */
  signer: SigningBackend;
  /**
   * Quien lleva al usuario hasta el fichero firmado. Ver
   * [`SignedDocumentOpener`].
   */
  opener: SignedDocumentOpener;
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
  drops,
  preferences,
  pdfs,
  destinations,
  certificates,
  rubrics,
  composer,
  signer,
  opener,
  menuAnchor,
}: AppProps) {
  const [dialog, setDialog] = useState<OpenDialog>(null);
  const [pdf, setPdf] = useState<PdfDocument | null>(null);
  // Por qué no se pudo pintar el último documento que se eligió. Vive al lado
  // del PDF y no dentro del visor porque lo produce quien abre, y el visor solo
  // lo enseña.
  const [pdfFailure, setPdfFailure] = useState<DocumentFailure | null>(null);
  // Lo que hay que contar del último arrastre, y **de qué documento**: soltar
  // varios PDF deja un aviso que habla del que se abrió, así que se va solo en
  // cuanto hay otro delante. Sin esa atadura habría que borrarlo a mano desde
  // cada camino que cambia de documento, y el que se olvidara dejaría el aviso
  // hablando de un documento que ya no está.
  const [dropNotice, setDropNotice] = useState<DropNotice | null>(null);
  // Dónde va la firma visible. Vive aquí y no en el visor porque el panel de
  // firma —su sub-issue— es quien dirá qué se pinta dentro del recuadro, y los
  // dos tienen que estar mirando el mismo.
  const [placement, setPlacement] = useState<SignaturePlacement | null>(null);
  const [settings, setSettings] = useState<Preferences | null>(null);
  // Dónde caerá el firmado, tal y como lo cuenta el backend. Es estado y no un
  // cálculo del pie porque el nombre lo compone Rust —con el sufijo y el
  // homónimo ya resueltos— y `writable` sale de comprobar la carpeta de verdad
  // (ID-63, ID-67): la ventana lo enseña, no lo deduce.
  const [destination, setDestination] = useState<Destination | null>(null);
  const [certificate, setCertificate] = useState<CertificateState>({ kind: "loading" });
  const [signature, setSignature] = useState<VisibleSignature>(DEFAULT_VISIBLE_SIGNATURE);
  const [rubric, setRubric] = useState<Rubric | null>(null);
  const [rubricFailure, setRubricFailure] = useState<RubricFailure | null>(null);
  // Por qué no se pudo abrir el firmado o su carpeta. Vive aquí y no dentro del
  // resumen porque lo produce quien llama al portal, y el resumen solo lo
  // enseña; sin él, el único camino que el usuario tiene hasta el fichero
  // fallaría sin decir nada (ADR-0011).
  const [openFailure, setOpenFailure] = useState<NamedFailure | null>(null);
  const signing = useSigning(signer);
  // Mientras los ajustes se leen todavía no se sabe, y lo guardado por omisión
  // es recordar; el primer documento no se puede abrir antes de esa lectura.
  const documents = useDocuments(recents, picker, settings?.rememberActivity ?? true);
  const activeId = documents.active?.id ?? null;
  // El documento activo, leíble desde la suscripción al arrastre sin que un
  // cambio de documento la vuelva a montar.
  const activeIdRef = useRef(activeId);
  activeIdRef.current = activeId;
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

  // La rúbrica adoptada en una sesión anterior sigue en el almacén aunque se
  // cierre la aplicación (ID-33): sin esta lectura al arrancar, «Tu rúbrica»
  // aparecía siempre apagada aunque el JPEG estuviera ahí.
  useEffect(() => {
    let current = true;
    rubrics.stored().then((found) => {
      if (current && found !== null) setRubric(found);
    });
    return () => {
      current = false;
    };
  }, [rubrics]);

  // El tema elegido, puesto en el documento. Es lo único de los ajustes que no
  // se pinta dentro de la ventana sino **sobre** ella: los tokens de color
  // cuelgan de `<html>`, así que quien lo aplica tiene que salir del árbol de
  // React. Mientras los ajustes se leen no se toca nada, y manda el sistema.
  useEffect(() => {
    if (settings) applyTheme(settings.theme);
  }, [settings]);

  // El destino se pregunta **por documento**, y otra vez cuando cambia la
  // carpeta elegida: el nombre depende del documento —y de qué homónimos haya
  // ya en la carpeta— y `writable` de si la carpeta sigue estando. Sin
  // documento delante no hay destino que enseñar.
  const chosenFolder = settings?.destination ?? null;
  useEffect(() => {
    // Sin documento delante no hay destino que enseñar, y sin ajustes leídos
    // tampoco: la carpeta que se va a consultar es la que ellos dicen.
    if (activeId === null || chosenFolder === null) {
      setDestination(null);
      return;
    }
    let current = true;
    destinations
      .previewFor(activeId)
      .then((found) => {
        if (current) setDestination(found);
      })
      .catch(() => {
        // Un destino que no se puede consultar no apaga el panel: se queda sin
        // pie hasta la siguiente vuelta, que es menos que perder el documento.
        if (current) setDestination(null);
      });
    return () => {
      current = false;
    };
  }, [destinations, activeId, chosenFolder]);

  // El documento activo, abierto para pintarlo. Cambiar de documento **repone
  // el recuadro de ese documento**, que es lo que guarda su fila de la bandeja
  // (ID-74): uno que ya estuvo abierto vuelve a su página y a su posición, y
  // uno nuevo llega sin ninguna y arranca donde toque, no donde lo dejó el
  // anterior (ID-22).
  useEffect(() => {
    const active = documents.active;
    if (!active) {
      setPdf(null);
      setPdfFailure(null);
      setPlacement(null);
      return;
    }
    let current = true;
    void pdfs.open(active).then((opened) => {
      if (!current) return;
      setPdf(opened.ok ? opened.pdf : null);
      setPdfFailure(opened.ok ? null : opened.failure);
      setPlacement(active.placement);
      // Documento nuevo, hora nueva: la del anterior lleva parada desde que se
      // abrió, y el recuadro de este llevaría estampada una hora vieja.
      setSigningInstant(new Date());
    });
    return () => {
      current = false;
    };
  }, [documents.active, pdfs]);

  // Dónde ha caído el recuadro se pinta y **se apunta en la fila del documento**
  // (ID-74): así el mismo contrato reabierto mañana vuelve a su página y a su
  // posición, y el siguiente documento arranca con el suyo.
  const placeDocument = documents.place;
  const rememberPlacement = useCallback(
    (next: SignaturePlacement) => {
      setPlacement(next);
      void placeDocument(next);
    },
    [placeDocument],
  );

  // El arrastre. Desemboca en el mismo sitio que el diálogo —`accept` es la
  // mitad de `open` que no habla con el portal—, y lo que añade es contar lo
  // que solo pasa al soltar: que no fuera un PDF, que no se dejara leer o que
  // fueran varios.
  //
  // La suscripción se monta una vez y sobrevive a los cambios de documento: si
  // dependiera de `documents`, cada apertura cancelaría el oyente y volvería a
  // suscribirse, y un arrastre que cayera en medio se perdería.
  const acceptDocument = documents.accept;
  useEffect(
    () =>
      drops.subscribe((drop) => {
        if (drop.document === null) {
          // No se ha abierto nada, así que el aviso habla de lo que ya hubiera
          // delante y se va con ello.
          setDropNotice(drop.failure && { about: activeIdRef.current, failure: drop.failure });
          return;
        }
        setDropNotice(
          drop.ignored > 0
            ? {
                about: drop.document.id,
                failure: {
                  situation: "droppedOnlyFirst",
                  detail: `se han soltado ${drop.ignored + 1} ficheros`,
                },
              }
            : null,
        );
        void acceptDocument(drop.document);
      }),
    [drops, acceptDocument],
  );

  // El acuse de recibo, solo si sigue siendo de lo que hay delante. El estado
  // «Firmado» guarda el asa del documento que se firmó; el recuento de páginas
  // que enseña sale del PDF abierto, así que el panel solo puede montarse
  // mientras los dos sean el mismo documento.
  const signedHere = acknowledgementFor(signing.state, documents.active?.id ?? null);
  const signedSomewhere = signing.state.kind === "signed";

  // Y cuando deja de serlo —se elige otro en la bandeja, se olvida el activo,
  // se vacía la lista— el estado se cierra, en vez de quedarse esperando a que
  // el documento firmado vuelva a estar delante.
  const signAnother = signing.signAnother;
  useEffect(() => {
    if (signedSomewhere && signedHere === null) signAnother();
    // Y el fallo de abrir se va con el resumen del que hablaba: es de un
    // documento concreto, como el propio acuse de recibo.
    if (signedHere === null) setOpenFailure(null);
  }, [signedSomewhere, signedHere, signAnother]);

  // Los dos caminos hasta el fichero. El fallo se recoge aquí y se enseña en el
  // resumen: un botón que no hace nada y no dice por qué deja al usuario sin
  // ninguna forma de llegar a lo que acaba de firmar (ID-79).
  const openSigned = (open: () => Promise<void>) => {
    setOpenFailure(null);
    open().catch((thrown: unknown) => setOpenFailure(classify(thrown)));
  };

  // «Volver a firmar»: se cierra el resumen y **se relee el original del
  // disco** (ID-80). Es abrir el documento otra vez, porque entre una firma y
  // la siguiente el usuario ha podido modificarlo fuera o haberse equivocado al
  // configurar la firma. Lo que decida el recuadro recordado —incluido el aviso
  // del ID-22 si ya no cabe— lo resuelve el camino de siempre, no uno nuevo.
  const reopenDocument = documents.reopen;
  const signAgain = () => {
    setOpenFailure(null);
    signing.signAnother();
    reopenDocument();
  };

  // Los certificados se buscan al arrancar y cada vez que alguien pide volver
  // a buscar: una tarjeta insertada tarde es el caso corriente, no la excepción.
  const lookForCertificates = useCallback(async () => {
    setCertificate({ kind: "loading" });
    try {
      const found = await certificates.list();
      setCertificate(chosenFrom(found));
    } catch (thrown) {
      // El rechazo se recoge **aquí** y no se deja escapar (ID-11): sin este
      // `catch` nadie volvía a llamar a `setCertificate` y la ficha se quedaba
      // girando en «Buscando certificados…» para siempre. Se clasifica con el
      // mismo `classify` que el visor y la rúbrica: no hay dos formas de
      // contar un error (ID-29).
      setCertificate({ kind: "failed", failure: classify(thrown) });
    }
  }, [certificates]);

  useEffect(() => {
    void lookForCertificates();
  }, [lookForCertificates]);

  /**
   * Elegir un certificado del desplegable.
   *
   * Solo cambia cuál está puesto: **no** se recuerda aquí. El certificado se
   * recuerda al firmar con él, que es lo que dicen el glosario —«el certificado
   * usado la última vez»— y la historia 7 —«con cuál firmé»—; ninguna dice «el
   * último que miré» (ADR-0010).
   */
  const chooseCertificate = useCallback((chosen: Certificate) => {
    setCertificate((state) =>
      state.kind === "unchosen" || state.kind === "chosen"
        ? { kind: "chosen", certificate: chosen, certificates: state.certificates }
        : state,
    );
  }, []);

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

  /**
   * Abrir un documento por el portal.
   *
   * El `catch` no es decorativo: si la orden que abre el diálogo rechaza, la
   * promesa quedaría sin dueño y el fallo no se contaría en ningún sitio. Se
   * cuenta donde se cuentan los demás del documento, en el visor.
   */
  const openDocument = async () => {
    try {
      await documents.open();
    } catch (thrown) {
      setPdfFailure(classify(thrown));
    }
  };

  /**
   * Un ajuste cambia **en cuanto se toca**, y solo se queda si el disco lo
   * acepta.
   *
   * Si guardar falla —el fichero de configuración no se deja escribir— la
   * pantalla vuelve a lo que había: una ventana que enseña un ajuste que el
   * disco no tiene estaría mintiendo sobre la sesión siguiente. Repuesto el
   * valor, **el rechazo sigue su camino**: quien lo recoge es Preferencias, que
   * es quien sabe en qué sección se pulsó y por tanto dónde va el aviso
   * (ID-70).
   */
  const changeSettings = async (next: Preferences) => {
    const before = settings;
    setSettings(next);
    try {
      await preferences.save(next);
    } catch (thrown) {
      setSettings(before);
      throw thrown;
    }
  };

  /**
   * La carpeta de destino se elige con el **selector de directorio** del
   * sistema, que abre Rust: la ventana no manda ninguna ruta —no la conoce— y
   * lo que recibe de vuelta es el nombre que enseña (ID-65).
   *
   * Cerrar el selector sin elegir deja la carpeta que hubiera. Si guardar
   * falla, el rechazo sigue su camino hasta Preferencias, que es quien sabe
   * dónde va el aviso (ID-70).
   */
  const chooseDestination = async () => {
    const chosen = await preferences.chooseFolder();
    if (chosen !== null && settings !== null) {
      setSettings({ ...settings, destination: chosen });
    }
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
      document: documents.active.id,
      certificate: chosen.id,
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
        status={documents.active?.badge ?? null}
        menuAnchor={menuAnchor ?? menuAnchorFor(navigator.userAgent)}
        onOpenPreferences={() => setDialog("preferences")}
        onOpenAbout={() => setDialog("about")}
        tray={
          <DocumentTray
            recents={documents.recents}
            activeId={documents.active?.id ?? null}
            onOpen={() => void openDocument()}
            onSelect={documents.select}
            onForget={(id) => void documents.forget(id)}
          />
        }
        viewer={
          <DocumentViewer
            pdf={pdf}
            placement={placement}
            onPlace={rememberPlacement}
            onOpen={() => void openDocument()}
            // Los dos avisos caben en el mismo sitio, y manda el del PDF: si el
            // documento que se soltó tampoco se deja pintar, eso es más urgente
            // que contar cuántos ficheros venían con él.
            failure={pdfFailure ?? (dropNotice?.about === activeId ? dropNotice.failure : null)}
          />
        }
        panel={
          signedHere !== null ? (
            // Firmado: la columna derecha cambia de contenido, no de sitio. Es
            // el único acuse de recibo que recibe quien firma, así que se monta
            // en cuanto la postfirma devuelve el documento.
            //
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
              onOpenDocument={() => openSigned(() => opener.openDocument())}
              onOpenFolder={() => openSigned(() => opener.openFolder())}
              onSignAgain={signAgain}
              failure={openFailure}
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
              onChooseCertificate={chooseCertificate}
              onRetryCertificates={() => void lookForCertificates()}
              onChooseModule={() => void lookForCertificates()}
              signature={signature}
              onChangeSignature={setSignature}
              page={placement?.page ?? null}
              rubric={rubric}
              rubricFailure={rubricFailure}
              onChooseRubric={() => void chooseRubric()}
              composer={composer}
              destination={
                destination ?? {
                  folder: settings?.destination ?? "",
                  name: null,
                  writable: true,
                }
              }
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
          onChooseDestination={chooseDestination}
          onChange={changeSettings}
          onForgetActivity={forgetActivity}
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
 * Con uno solo no se pregunta: elegir entre una cosa no es elegir, y eso
 * incluye a uno caducado —el panel lo pone y avisa de por qué no sirve—.
 *
 * Con varios manda **el que se usó la última vez** (#110): quien tiene cuatro
 * certificados los elige una vez, no cada día. Eso no contradice la regla de
 * que la aplicación no elige por su cuenta: no está eligiendo, está devolviendo
 * lo que ya se eligió firmando. Y viene con su estado de ahora, no con el de
 * entonces: si desde la última firma caducó, sale puesto y el panel avisa de
 * por qué ya no sirve, igual que hace con el único certificado de un token.
 *
 * Sin recordado —primera vez, o el recordado ya no está en el token— sigue sin
 * haber preselección: el desplegable dice «Elegir certificado» y el botón de
 * firmar sigue apagado, porque el orden de la lista solo dice en qué orden
 * cargaron los módulos.
 */
function chosenFrom(found: readonly Certificate[]): CertificateState {
  const [first] = found;
  if (first === undefined) return { kind: "empty" };
  if (found.length === 1) return { kind: "chosen", certificate: first, certificates: found };
  const remembered = found.find((one) => one.remembered);
  if (remembered !== undefined) {
    return { kind: "chosen", certificate: remembered, certificates: found };
  }
  return { kind: "unchosen", certificates: found };
}
