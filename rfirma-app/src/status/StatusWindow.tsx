//! La ventana del estado de rFirma: lleva el puerto, mide las señales, cierra con Escape y monta su vista y el velo de la retirada.

import { useCallback, useEffect, useRef, useState } from "react";
import { useActionKeys } from "../design-system/actionKeys";
import type {
  ExternalDestination,
  ExternalDestinationOpener,
} from "../desktop/externalDestination";
import { unavailableExternalDestinationOpener } from "../desktop/externalDestination";
import { classify, type NamedFailure } from "../errors/classify";
import { StatusView } from "./StatusView";
import {
  type Signal,
  type SignalRow,
  type StatusPort,
  withLocalCaCertificateMeasured,
  withVersionMeasured,
} from "./status";
import { WithdrawCertificateDialog } from "./WithdrawCertificateDialog";

export interface StatusWindowProps {
  onClose: () => void;
  statusPort: StatusPort;
  externalDestinations?: ExternalDestinationOpener;
  /**
   * Se llama con las filas de cada remedición propia —al abrirse, tras una
   * acción, con «Volver a comprobar»—, para quien más allá del panel también
   * necesite saberlas (el triángulo del menú).
   */
  onRowsChange?: (rows: SignalRow[]) => void;
  /** Las señales cuyo detalle empieza desplegado. */
  initiallyExpanded?: Signal[];
}

function restoreSignals(snapshot: SignalRow[], signals: Signal[]) {
  return (current: SignalRow[]) =>
    current.map((row) =>
      signals.includes(row.signal)
        ? (snapshot.find((before) => before.signal === row.signal) ?? row)
        : row,
    );
}

export function StatusWindow({
  onClose,
  statusPort,
  externalDestinations = unavailableExternalDestinationOpener(),
  onRowsChange,
  initiallyExpanded,
}: StatusWindowProps) {
  const [rows, setRows] = useState<SignalRow[]>([]);
  const [isRechecking, setIsRechecking] = useState(false);
  const [isWithdrawing, setIsWithdrawing] = useState(false);
  const [failure, setFailure] = useState<NamedFailure | null>(null);
  const rowsRef = useRef(rows);
  rowsRef.current = rows;

  useEffect(() => {
    onRowsChange?.(rows);
  }, [rows, onRowsChange]);

  useEffect(() => {
    let cancelled = false;
    statusPort
      .readStatus()
      .then((initialRows) => {
        if (!cancelled) {
          setRows(initialRows);
        }
        return withLocalCaCertificateMeasured(initialRows, statusPort);
      })
      .then((rowsWithCaMeasured) => {
        if (!cancelled) {
          setRows(rowsWithCaMeasured);
        }
        return withVersionMeasured(rowsWithCaMeasured, statusPort);
      })
      .then((measuredRows) => {
        if (!cancelled) {
          setRows(measuredRows);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [statusPort]);

  useActionKeys({ onSecondary: onClose });

  const handleRecheck = useCallback(() => {
    const snapshot = rowsRef.current;
    setFailure(null);
    setIsRechecking(true);
    setRows((current) =>
      current.map((row) => ({
        ...row,
        verdict: "checking",
        action: null,
        detail: null,
        candidates: null,
      })),
    );
    statusPort
      .recheck()
      .then((updatedRows) => {
        setRows(updatedRows);
        return withVersionMeasured(updatedRows, statusPort);
      })
      .then((measuredRows) => {
        setRows(measuredRows);
        setIsRechecking(false);
      })
      .catch((thrown: unknown) => {
        setRows(snapshot);
        setIsRechecking(false);
        setFailure(classify(thrown));
      });
  }, [statusPort]);

  const handleChooseSiteSignatureHandler = useCallback(
    (handlerId: string) => {
      const snapshot = rowsRef.current;
      setFailure(null);
      setRows((current) =>
        current.map((r) =>
          r.signal === "siteSignature" || r.signal === "localCaCertificate"
            ? {
                ...r,
                verdict: "checking",
                action: null,
                detail: null,
                candidates: null,
                restartFirefoxNotice: false,
              }
            : r,
        ),
      );
      statusPort
        .chooseSiteSignatureHandler(handlerId)
        .then((updatedRows) => {
          setRows((current) =>
            current.map((r) => updatedRows.find((updated) => updated.signal === r.signal) ?? r),
          );
        })
        .catch((thrown: unknown) => {
          setRows(restoreSignals(snapshot, ["siteSignature", "localCaCertificate"]));
          setFailure(classify(thrown));
        });
    },
    [statusPort],
  );

  const handleAction = useCallback(
    (row: SignalRow) => {
      if (!row.action) return;
      if (row.action.kind === "link") {
        void externalDestinations.open(row.action.target as ExternalDestination);
      }
      if (row.action.kind === "choice") {
        handleChooseSiteSignatureHandler(row.action.target);
        return;
      }
      const snapshot = rowsRef.current;
      setFailure(null);
      setRows((current) =>
        current.map((r) =>
          r.signal === row.signal
            ? {
                ...r,
                verdict: "checking",
                action: null,
                detail: null,
                candidates: null,
                restartFirefoxNotice: false,
              }
            : r,
        ),
      );
      if (row.action.kind === "repair") {
        statusPort
          .installLocalCaCertificate()
          .then((updatedRow) => {
            setRows((current) =>
              current.map((r) => (r.signal === updatedRow.signal ? updatedRow : r)),
            );
          })
          .catch((thrown: unknown) => {
            setRows(restoreSignals(snapshot, [row.signal]));
            setFailure(classify(thrown));
          });
        return;
      }
      statusPort
        .recheck()
        .then((updatedRows) => {
          setRows(updatedRows);
        })
        .catch((thrown: unknown) => {
          setRows(restoreSignals(snapshot, [row.signal]));
          setFailure(classify(thrown));
        });
    },
    [externalDestinations, statusPort, handleChooseSiteSignatureHandler],
  );

  // Al cerrar el velo de la retirada, el panel vuelve a medir: la verdad
  // sigue viviendo en la tabla, no en el diálogo.
  const handleWithdrawalDialogClose = useCallback(() => {
    setIsWithdrawing(false);
    handleRecheck();
  }, [handleRecheck]);

  const localCaCertificateRow = rows.find((row) => row.signal === "localCaCertificate");
  const trustedStores =
    localCaCertificateRow?.detail?.kind === "trust" ? localCaCertificateRow.detail.stores : [];

  return (
    <>
      <StatusView
        rows={rows}
        failure={failure}
        isRechecking={isRechecking}
        onClose={onClose}
        onRecheck={handleRecheck}
        onAction={handleAction}
        onChooseSiteSignatureHandler={handleChooseSiteSignatureHandler}
        onWithdraw={() => setIsWithdrawing(true)}
        initiallyExpanded={initiallyExpanded}
      />
      {isWithdrawing && (
        <WithdrawCertificateDialog
          stores={trustedStores}
          onWithdraw={statusPort.withdrawRfirma}
          onClose={handleWithdrawalDialogClose}
        />
      )}
    </>
  );
}
