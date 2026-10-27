//! Mockup de «Informar de un problema» desde el menú: la entrada, el diálogo con el selector de trámites y fallos, y el caso vacío.

import { useRef, useState } from "react";
import { Admonition } from "../design-system/Admonition";
import { Button } from "../design-system/Button";
import { Combobox, type ComboboxOption } from "../design-system/Combobox";
import { Dialog } from "../design-system/Dialog";
import { Menu, MenuItem } from "../design-system/Menu";
import { Row } from "../design-system/Row";
import { ReportBody, ReportFooter, useDelivery } from "./ProblemReportFailures";
import "../shell/Header.css";
import "../sede/SedeEnding.css";
import "./ProblemReportMockup.css";

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

/** Un trámite de sede o un fallo de la ventana principal, con el informe que le corresponde. */
export interface Reportable {
  id: string;
  day: string;
  time: string;
  place: string;
  report: string;
}

/** El informe del último fallo, con un selector para cambiar a otro. */
export function ReportWithPicker({
  entries,
  newerVersion = null,
  defaultOpen = false,
}: {
  entries: Reportable[];
  newerVersion?: string | null;
  defaultOpen?: boolean;
}) {
  const [chosen, setChosen] = useState(entries[0]);
  const { delivery, send } = useDelivery("ready", true);
  const primary = useRef<HTMLButtonElement>(null);
  const options: ComboboxOption<Reportable>[] = entries.map((entry) => ({
    id: entry.id,
    item: entry,
    keywords: [entry.place],
    group: entry.day,
  }));

  if (chosen === undefined) return <NothingToReport />;
  return (
    <Dialog label="Informe del problema" onClose={() => {}} primary={primary} className="report">
      <div className="failures__dialog-body">
        <ReportBody title="Informe del problema" report={chosen.report} delivery={delivery}>
          <Combobox
            label="Problema"
            options={options}
            value={chosen.id}
            onChange={setChosen}
            renderOption={(entry) => <EntryLine entry={entry} withDay={false} />}
            searchPlaceholder="Sede o ventana"
            emptyMessage="Nada coincide"
            countLabel={(shown, total) => `${shown} de ${total}`}
            listMaxHeight={320}
            alwaysGroupHeaders
            defaultOpen={defaultOpen}
          >
            <EntryLine entry={chosen} withDay />
          </Combobox>
          {newerVersion !== null && (
            <Admonition tone="info" className="mail__fixed">
              Hay una versión más reciente, {newerVersion}.
            </Admonition>
          )}
        </ReportBody>
      </div>
      <div className="report__footer failures__footer">
        <ReportFooter
          delivery={delivery}
          dismissLabel="Cancelar"
          onDismiss={() => {}}
          onSend={send}
          primary={primary}
        />
      </div>
    </Dialog>
  );
}

function EntryLine({ entry, withDay }: { entry: Reportable; withDay: boolean }) {
  return (
    <span className="problems__line">
      <span className="problems__site">{entry.place}</span>
      <span className="rf-hint">{withDay ? `${entry.day}, ${entry.time}` : entry.time}</span>
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
      <p className="rf-title sede-ending__title">Informe del problema</p>
      <p className="rf-prose">Nada ha fallado en los últimos 30 días.</p>
      <Row className="report__footer">
        <Button ref={closeButton} variant="primary" onClick={() => {}}>
          Cerrar
        </Button>
      </Row>
    </Dialog>
  );
}
