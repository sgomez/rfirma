/** El puerto de Tauri del estado de la instalación. */

import { invoke } from "@tauri-apps/api/core";
import type { SignalRow, StatusPort, WithdrawalReport } from "./status/status";

export function tauriStatusPort(): StatusPort {
  return {
    readStatus: () => invoke<SignalRow[]>("read_status", { recheck: false }),
    recheck: () => invoke<SignalRow[]>("read_status", { recheck: true }),
    measureLocalCaCertificate: () => invoke<SignalRow>("measure_local_ca_certificate"),
    installLocalCaCertificate: () => invoke<SignalRow>("install_local_ca_certificate"),
    chooseSiteSignatureHandler: (handlerId) =>
      invoke<SignalRow[]>("choose_site_signature_handler", { handler: handlerId }),
    withdrawRfirma: (previous) => invoke<WithdrawalReport>("withdraw_rfirma", { previous }),
  };
}
