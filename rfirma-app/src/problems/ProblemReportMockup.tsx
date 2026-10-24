//! Mockup del informe de problema: la entrada del menú, el informe con su selector de trámite y el mismo informe en la ventana de sede.

import { type RefObject, useRef, useState } from "react";
import { Admonition } from "../design-system/Admonition";
import { Button } from "../design-system/Button";
import { Combobox, type ComboboxOption } from "../design-system/Combobox";
import { Dialog } from "../design-system/Dialog";
import { Icon } from "../design-system/icons";
import { Menu, MenuItem } from "../design-system/Menu";
import { Row } from "../design-system/Row";
import { SedeBody } from "../sede/SedeFrame";
import { SedeOrigin } from "../sede/SedeOrigin";
import "../sede/SedeWindow.css";
import "../sede/SedeEnding.css";
import "../shell/Header.css";
import "./ProblemReportMockup.css";

type Outcome = "failed" | "cancelled" | "abandoned" | "interrupted";

export interface JournaledErrand {
  id: string;
  when: string;
  site: string;
  operation: string;
  outcome: Outcome;
}

export function MenuWithReportEntry() {
  const anchor = useRef<HTMLDivElement>(null);
  const entries = ["Estado", null, "Preferencias", "Ayuda", "Informar de un problema", "Acerca de"];

  return (
    <div ref={anchor} className="problems-menu">
      <Menu open onClose={() => {}} anchorRef={anchor} className="header__popup" aria-label="Menú">
        {entries.map((entry) =>
          entry === null ? (
            <hr key="divider" className="rf-divider" />
          ) : (
            <MenuItem key={entry} className="header__entry">
              <span className="header__entryLabel">{entry}</span>
              <span className="header__entryIcon" aria-hidden="true" />
            </MenuItem>
          ),
        )}
      </Menu>
    </div>
  );
}

interface ReportPreviewProps {
  errand: JournaledErrand;
  report: string;
  fitsInLink: boolean;
  newerVersion: string | null;
  onBack: () => void;
  onSave: () => void;
  onOpenIssue: () => void;
}

function ReportContent({
  report,
  fitsInLink,
  newerVersion,
}: Pick<ReportPreviewProps, "report" | "fitsInLink" | "newerVersion">) {
  const [description, setDescription] = useState("");

  return (
    <>
      {newerVersion !== null && (
        <Admonition tone="info">Hay una versión más reciente, {newerVersion}.</Admonition>
      )}

      {/* biome-ignore lint/a11y/noNoninteractiveTabindex: la región con desplazamiento tiene que poder enfocarse para leerla con el teclado. */}
      <pre className="report__text" tabIndex={0}>
        {report}
      </pre>

      <div className="rf-field">
        <label className="rf-label" htmlFor="report-description">
          ¿Qué estabas haciendo? (opcional)
        </label>
        <textarea
          id="report-description"
          className="rf-input report__description"
          rows={3}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
        <span className="rf-hint">Será público: no incluyas datos personales.</span>
      </div>

      {!fitsInLink && (
        <Admonition tone="warning">No cabe en el enlace: guárdalo y adjúntalo al issue.</Admonition>
      )}
    </>
  );
}

function ReportActions({
  openButton,
  dismissLabel,
  onBack,
  onSave,
  onOpenIssue,
}: Pick<ReportPreviewProps, "onBack" | "onSave" | "onOpenIssue"> & {
  openButton: RefObject<HTMLButtonElement | null>;
  dismissLabel: string;
}) {
  return (
    <>
      <Button variant="ghost" onClick={onSave}>
        Guardar en un fichero
      </Button>
      <span className="report__gap" />
      <Button variant="secondary" onClick={onBack}>
        {dismissLabel}
      </Button>
      <Button ref={openButton} variant="primary" onClick={onOpenIssue}>
        <Icon name="externalLink" size={14} />
        Abrir en GitHub
      </Button>
    </>
  );
}

/** La misma vista previa dentro de la ventana de sede, que crece como al marcar el área. */
export function SedeReport({ errand, onBack, ...rest }: ReportPreviewProps) {
  const openButton = useRef<HTMLButtonElement>(null);

  return (
    <SedeBody
      flush
      steadyFooter
      primary={openButton}
      onEscape={onBack}
      footer={
        <ReportActions openButton={openButton} dismissLabel="Volver" onBack={onBack} {...rest} />
      }
    >
      <div className="sede-ending">
        <SedeOrigin origin={errand.site} asks={null} />
        <div className="sede-ending__body">
          <p className="rf-title problems__heading">Informe del problema</p>
          <ReportContent {...rest} />
        </div>
      </div>
    </SedeBody>
  );
}

export interface DatedErrand extends JournaledErrand {
  day: string;
  time: string;
}

/** El informe del último trámite fallido, con un selector para cambiar a otro: sin lista previa. */
export function ReportWithPicker({
  errands,
  report,
  fitsInLink = true,
  newerVersion = null,
  defaultOpen = false,
}: {
  errands: DatedErrand[];
  report: string;
  fitsInLink?: boolean;
  newerVersion?: string | null;
  defaultOpen?: boolean;
}) {
  const [chosen, setChosen] = useState(errands[0]);
  const openButton = useRef<HTMLButtonElement>(null);
  const options: ComboboxOption<DatedErrand>[] = errands.map((errand) => ({
    id: errand.id,
    item: errand,
    keywords: [errand.site],
    group: errand.day,
  }));

  if (chosen === undefined) return <NothingToReport />;
  return (
    <Dialog label="Informe del problema" onClose={() => {}} primary={openButton} className="report">
      <p className="rf-title problems__heading">Informe del problema</p>
      <Combobox
        label="Trámite"
        options={options}
        value={chosen.id}
        onChange={setChosen}
        renderOption={(errand) => <ErrandLine errand={errand} withDay={false} />}
        searchPlaceholder="Sede"
        emptyMessage="Ningún trámite coincide"
        countLabel={(shown, total) => `${shown} de ${total}`}
        listMaxHeight={320}
        alwaysGroupHeaders
        defaultOpen={defaultOpen}
      >
        <ErrandLine errand={chosen} withDay />
      </Combobox>
      <ReportContent report={report} fitsInLink={fitsInLink} newerVersion={newerVersion} />
      <Row className="report__footer">
        <ReportActions
          openButton={openButton}
          dismissLabel="Cancelar"
          onBack={() => {}}
          onSave={() => {}}
          onOpenIssue={() => {}}
        />
      </Row>
    </Dialog>
  );
}

function ErrandLine({ errand, withDay }: { errand: DatedErrand; withDay: boolean }) {
  return (
    <span className="problems__line">
      <span className="problems__site">{errand.site}</span>
      <span className="rf-hint">{withDay ? `${errand.day}, ${errand.time}` : errand.time}</span>
    </span>
  );
}

function NothingToReport() {
  const closeButton = useRef<HTMLButtonElement>(null);

  return (
    <Dialog
      label="Informe del problema"
      onClose={() => {}}
      primary={closeButton}
      className="report"
    >
      <p className="rf-title problems__heading">Informe del problema</p>
      <p className="rf-prose">Ningún trámite ha fallado en los últimos 30 días.</p>
      <Row className="report__footer">
        <Button ref={closeButton} variant="primary" onClick={() => {}}>
          Cerrar
        </Button>
      </Row>
    </Dialog>
  );
}
