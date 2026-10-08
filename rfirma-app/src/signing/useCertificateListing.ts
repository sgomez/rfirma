//! El listado de certificados que comparten el desplegable de la firma y Preferencias: buscar, volver a buscar, instalar, quitar y vaciar el almacén, y la lista que cambian los lectores; no elige ninguno.

import { useCallback, useEffect, useRef, useState } from "react";
import { classify, type NamedFailure } from "../errors/classify";
import {
  type Certificate,
  type CertificateStore,
  installedCertificates,
  NO_READER,
  type ReaderStatus,
} from "./certificate";

/** Lo que hay en los tokens conectados: buscándolo, el fallo al buscar o lo encontrado. */
export type CertificateListing =
  | { kind: "loading" }
  | { kind: "failed"; failure: NamedFailure }
  /** `byReader` cuando la lista la trajo un lector al meter o sacar una tarjeta, y no una búsqueda. */
  | { kind: "listed"; certificates: readonly Certificate[]; byReader?: true };

/** Los certificados del almacén, buscados al montarse y cada vez que algo los cambia. */
export function useCertificateListing(store: CertificateStore) {
  const [listing, setListing] = useState<CertificateListing>({ kind: "loading" });
  const [reader, setReader] = useState<ReaderStatus>(NO_READER);
  const latestListing = useRef(0);

  const lookAgain = useCallback(async () => {
    const mine = ++latestListing.current;
    setListing({ kind: "loading" });
    try {
      const certificates = await store.list();
      if (mine === latestListing.current) setListing({ kind: "listed", certificates });
    } catch (thrown) {
      if (mine === latestListing.current) setListing({ kind: "failed", failure: classify(thrown) });
    }
  }, [store]);

  useEffect(() => {
    void lookAgain();
  }, [lookAgain]);

  useEffect(
    () =>
      store.followReaders((news) => {
        setReader(news.reader);
        if (news.certificates === null) return;
        latestListing.current++;
        setListing({ kind: "listed", certificates: news.certificates, byReader: true });
      }),
    [store],
  );

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

  return { listing, reader, lookAgain, installed, install, remove, emptyStore };
}
