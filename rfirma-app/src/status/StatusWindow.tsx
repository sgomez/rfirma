//! La ventana del estado de rFirma: lleva el puerto, mide las señales, atiende el Escape y monta su vista y el velo de la retirada.

import { useCallback, useEffect, useState } from "react";
import type {
  ExternalDestination,
  ExternalDestinationOpener,
} from "../desktop/externalDestination";
import { unavailableExternalDestinationOpener } from "../desktop/externalDestination";
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

  // El velo de la retirada atiende su propio Escape (WithdrawCertificateDialog);
  // mientras está delante, uno que le llegue aquí no debe cerrar además el panel.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && !isWithdrawing) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, isWithdrawing]);

  const handleRecheck = useCallback(() => {
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
      });
  }, [statusPort]);

  const handleChooseSiteSignatureHandler = useCallback(
    (handlerId: string) => {
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
      statusPort.chooseSiteSignatureHandler(handlerId).then((updatedRows) => {
        setRows((current) =>
          current.map((r) => updatedRows.find((updated) => updated.signal === r.signal) ?? r),
        );
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
        statusPort.installLocalCaCertificate().then((updatedRow) => {
          setRows((current) =>
            current.map((r) => (r.signal === updatedRow.signal ? updatedRow : r)),
          );
        });
        return;
      }
      statusPort.recheck().then((updatedRows) => {
        setRows(updatedRows);
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
