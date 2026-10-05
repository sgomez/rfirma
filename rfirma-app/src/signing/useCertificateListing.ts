//! El listado de certificados que comparten el desplegable de la firma y Preferencias: buscar, volver a buscar, instalar, quitar y vaciar el almacén; no elige ninguno.

import { useCallback, useEffect, useState } from "react";
import { classify, type NamedFailure } from "../errors/classify";
import { type Certificate, type CertificateStore, installedCertificates } from "./certificate";

/** Lo que hay en los tokens conectados: buscándolo, el fallo al buscar o lo encontrado. */
export type CertificateListing =
  | { kind: "loading" }
  | { kind: "failed"; failure: NamedFailure }
  | { kind: "listed"; certificates: readonly Certificate[] };

/** Los certificados del almacén, buscados al montarse y cada vez que algo los cambia. */
export function useCertificateListing(store: CertificateStore) {
  const [listing, setListing] = useState<CertificateListing>({ kind: "loading" });

  const lookAgain = useCallback(async () => {
    setListing({ kind: "loading" });
    try {
      setListing({ kind: "listed", certificates: await store.list() });
    } catch (thrown) {
      setListing({ kind: "failed", failure: classify(thrown) });
    }
  }, [store]);

  useEffect(() => {
    void lookAgain();
  }, [lookAgain]);

  const installed = installedCertificates(listing.kind === "listed" ? listing.certificates : []);

  const install = useCallback(async () => {
    const added = await store.install();
    if (added) await lookAgain();
    return added;
  }, [store, lookAgain]);

  const remove = useCallback(
    async (id: string) => {
      await store.remove(id);
      await lookAgain();
    },
    [store, lookAgain],
  );

  const emptyStore = useCallback(async () => {
    await store.emptyStore();
    await lookAgain();
  }, [store, lookAgain]);

  return { listing, lookAgain, installed, install, remove, emptyStore };
}
