//! El velo que confirma, ejecuta y cuenta la retirada del certificado de rFirma (docs/design/retirar-certificado.md).

import { useState } from "react";
import type { StoreDetail, WithdrawalReport } from "./status";
import { type WithdrawalMoment, WithdrawCertificateView } from "./WithdrawCertificateView";

interface WithdrawCertificateDialogProps {
  /** Los almacenes donde está hoy el certificado, tal como los cuenta la fila. */
  stores: StoreDetail[];
  /**
   * Ejecuta la retirada de verdad: el manejador de sedes y la CA de cada
   * almacén. Con el resultado del intento anterior, solo repite lo que
   * falló.
   */
  onWithdraw: (previous: WithdrawalReport | null) => Promise<WithdrawalReport>;
  /** Cierra el velo, con o sin retirada hecha; quien nos monta vuelve a medir. */
  onClose: () => void;
}

/** Conecta la retirada con su vista: lleva el momento y el informe del último intento. */
export function WithdrawCertificateDialog({
  stores,
  onWithdraw,
  onClose,
}: WithdrawCertificateDialogProps) {
  const [moment, setMoment] = useState<WithdrawalMoment>("question");
  const [report, setReport] = useState<WithdrawalReport | null>(null);

  const withdraw = () => {
    setMoment("working");
    void onWithdraw(report).then((result) => {
      setReport(result);
      setMoment("result");
    });
  };

  return (
    <WithdrawCertificateView
      stores={stores}
      moment={moment}
      report={report}
      onWithdraw={withdraw}
      onClose={onClose}
    />
  );
}
