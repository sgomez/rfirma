//! El almacén de certificados con lectores de pega: lista lo que se le da y entrega a quien escucha las noticias del lector que se le anuncien.

import {
  type Certificate,
  type CertificateStore,
  emptyCertificateStore,
  type ReaderNews,
} from "../certificate";

/** El almacén de pega, con el gesto que hace de backend al cambiar un lector o una tarjeta. */
export interface AnnouncingCertificateStore extends CertificateStore {
  announce(news: ReaderNews): void;
}

/** Un almacén que lista `found` y anuncia las noticias del lector a quien las escuche. */
export function announcingCertificateStore(
  found: readonly Certificate[] = [],
): AnnouncingCertificateStore {
  const listeners = new Set<(news: ReaderNews) => void>();
  return {
    ...emptyCertificateStore(),
    list: async () => found,
    followReaders: (onNews) => {
      listeners.add(onNews);
      return () => listeners.delete(onNews);
    },
    announce: (news) => {
      for (const listener of listeners) listener(news);
    },
  };
}
