//! Mockup del informe de un fallo interno (pánico del backend o fallo al pintar) y del envío a GlitchTip, en las dos ventanas.

import { type ReactNode, type RefObject, useRef, useState } from "react";
import { Admonition } from "../design-system/Admonition";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { Icon } from "../design-system/icons";
import { SedeBody } from "../sede/SedeFrame";
import { SedeOrigin } from "../sede/SedeOrigin";
import "../sede/SedeWindow.css";
import "../sede/SedeEnding.css";
import "./ProblemReportMockup.css";

export type Delivery = "ready" | "sending" | "sent" | "failed";

/** Lo que pasa al pulsar «Enviar»: la historia decide si GlitchTip contesta o no. */
export function useDelivery(initial: Delivery, reachable: boolean) {
  const [delivery, setDelivery] = useState<Delivery>(initial);
  const send = () => {
    setDelivery("sending");
    setTimeout(() => setDelivery(reachable ? "sent" : "failed"), 900);
  };
  return { delivery, send };
}

function DeliveryNotice({ delivery }: { delivery: Delivery }) {
  if (delivery === "sent") {
    return (
      <Admonition tone="info" className="mail__fixed">
        Enviado.
      </Admonition>
    );
  }
  if (delivery === "failed") {
    return (
      <Admonition tone="warning" className="mail__fixed">
        No se ha podido enviar.
      </Admonition>
    );
  }
  return null;
}

export function ReportBody({
  title,
  report,
  delivery,
  children,
}: {
  title: string;
  report: string;
  delivery: Delivery;
  children?: ReactNode;
}) {
  return (
    <>
      <p className="rf-title sede-ending__title">{title}</p>
      {children}
      {/* biome-ignore lint/a11y/noNoninteractiveTabindex: la región con desplazamiento tiene que poder enfocarse para leerla con el teclado. */}
      <pre className="report__text failures__text" tabIndex={0}>
        {report}
      </pre>
      <DeliveryNotice delivery={delivery} />
    </>
  );
}

export function ReportFooter({
  delivery,
  dismissLabel,
  onDismiss,
  onSend,
  primary,
}: {
  delivery: Delivery;
  dismissLabel: string;
  onDismiss: () => void;
  onSend: () => void;
  primary: RefObject<HTMLButtonElement | null>;
}) {
  return (
    <>
      <span className="report__gap" />
      {delivery === "sent" || delivery === "failed" ? (
        <Button ref={primary} variant="primary" onClick={onDismiss}>
          Cerrar
        </Button>
      ) : (
        <>
          <Button variant="secondary" onClick={onDismiss}>
            {dismissLabel}
          </Button>
          <Button
            ref={primary}
            variant="primary"
            disabled={delivery === "sending"}
            onClick={onSend}
          >
            {delivery === "sending" ? "Enviando…" : "Enviar"}
          </Button>
        </>
      )}
    </>
  );
}

interface SendableReportProps {
  report: string;
  initial?: Delivery;
  reachable?: boolean;
}

/** El aviso de un fallo interno, igual en las dos ventanas: qué no se ha podido hacer, de quién es la culpa y qué hacer ahora. */
function FailureNotice({
  title,
  sentence,
  primaryLabel,
  onReport,
}: {
  title: string;
  sentence: string;
  primaryLabel: string;
  onReport: () => void;
}) {
  return (
    <div className="failure-notice" role="alert">
      <div className="failure-notice__head">
        <Icon name="warning" />
        <p className="rf-title failure-notice__title">{title}</p>
      </div>
      <p className="rf-prose failure-notice__sentence">{sentence}</p>
      <div className="failure-notice__actions">
        <Button variant="secondary" onClick={onReport}>
          Informar del problema
        </Button>
        <Button variant="primary" onClick={() => {}}>
          {primaryLabel}
        </Button>
      </div>
    </div>
  );
}

/** 5 · Ventana principal: un pánico del backend al leer el documento; su informe se abre en un diálogo. */
export function MainWindowPanic({
  report,
  initial = "ready",
  reachable = true,
}: SendableReportProps) {
  const [reporting, setReporting] = useState(initial !== "ready");
  const { delivery, send } = useDelivery(initial, reachable);
  const primary = useRef<HTMLButtonElement>(null);

  return (
    <div className="failures__centre">
      <FailureNotice
        title="No se ha podido abrir el documento"
        sentence="Es un fallo de rFirma. Prueba a abrirlo de nuevo."
        primaryLabel="Cerrar"
        onReport={() => setReporting(true)}
      />
      {reporting && (
        <Dialog
          label="Informe del problema"
          onClose={() => setReporting(false)}
          primary={primary}
          className="report"
        >
          <div className="failures__dialog-body">
            <ReportBody title="Informe del problema" report={report} delivery={delivery} />
          </div>
          <div className="report__footer failures__footer">
            <ReportFooter
              delivery={delivery}
              dismissLabel="Cancelar"
              onDismiss={() => setReporting(false)}
              onSend={send}
              primary={primary}
            />
          </div>
        </Dialog>
      )}
    </div>
  );
}

/** 6 · Fallo al pintar: la ventana ya no existe, así que el informe se abre en la misma pantalla. */
export function RenderFailure({
  report,
  initial = "ready",
  reachable = true,
}: SendableReportProps) {
  const [reporting, setReporting] = useState(initial !== "ready");
  const { delivery, send } = useDelivery(initial, reachable);
  const primary = useRef<HTMLButtonElement>(null);

  if (!reporting) {
    return (
      <div className="failures__centre">
        <FailureNotice
          title="La ventana ha dejado de funcionar"
          sentence="Es un fallo de rFirma. Recárgala para seguir."
          primaryLabel="Recargar la ventana"
          onReport={() => setReporting(true)}
        />
      </div>
    );
  }
  return (
    <div className="failures__screen">
      <div className="failures__screen-body">
        <ReportBody title="Informe del problema" report={report} delivery={delivery} />
      </div>
      <div className="failures__footer">
        <ReportFooter
          delivery={delivery}
          dismissLabel="Volver"
          onDismiss={() => setReporting(false)}
          onSend={send}
          primary={primary}
        />
      </div>
    </div>
  );
}

/** El desenlace de sede que no acaba en firma, con el mismo título, frase y botones que el aviso. */
export function SedeFailure({
  site,
  title,
  sentence,
  onReport,
}: {
  site: string;
  title: string;
  sentence: string;
  onReport: () => void;
}) {
  const close = useRef<HTMLButtonElement>(null);
  return (
    <SedeBody
      steadyFooter
      flush
      primary={close}
      footer={
        <>
          <span className="report__gap" />
          <Button variant="secondary" onClick={onReport}>
            Informar del problema
          </Button>
          <Button ref={close} variant="primary" onClick={() => {}}>
            Cerrar
          </Button>
        </>
      }
    >
      <div className="sede-ending">
        <SedeOrigin origin={site} asks={null} />
        <div className="sede-ending__body">
          <div className="failure-notice__head">
            <Icon name="warning" />
            <p className="rf-title sede-ending__title">{title}</p>
          </div>
          <p className="rf-prose failure-notice__sentence">{sentence}</p>
        </div>
      </div>
    </SedeBody>
  );
}

/** 7 · Ventana de sede: el informe del trámite, ahora con «Enviar»; sirve igual para un rechazo que para un pánico. */
export function SedeReportSend({
  site,
  report,
  initial = "ready",
  reachable = true,
  onBack,
}: SendableReportProps & { site: string; onBack: () => void }) {
  const { delivery, send } = useDelivery(initial, reachable);
  const primary = useRef<HTMLButtonElement>(null);

  return (
    <SedeBody
      flush
      steadyFooter
      primary={primary}
      onEscape={onBack}
      footer={
        <ReportFooter
          delivery={delivery}
          dismissLabel="Volver"
          onDismiss={onBack}
          onSend={send}
          primary={primary}
        />
      }
    >
      <div className="sede-ending">
        <SedeOrigin origin={site} asks={null} />
        <div className="sede-ending__body">
          <ReportBody title="Informe del problema" report={report} delivery={delivery} />
        </div>
      </div>
    </SedeBody>
  );
}

export function WindowFrame({
  width,
  height,
  children,
}: {
  width: number;
  height: number;
  children: ReactNode;
}) {
  return (
    <div className="failures__frame" style={{ width, height }}>
      {children}
    </div>
  );
}
