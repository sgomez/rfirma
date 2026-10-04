//! El estado que se manda a la barra de título nativa y a dónde lleva cada acción que vuelve.

import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { RecentDocument } from "./documents/recents";
import type {
  NativeTitlebar,
  TitlebarAction,
  TitlebarActionName,
  TitlebarRecent,
} from "./shell/nativeTitlebar";

export type TitlebarHandlers = Record<TitlebarActionName, () => void> & {
  recent: (id: string) => void;
};

function recentEntry(row: RecentDocument): TitlebarRecent {
  return {
    path: row.id,
    name: row.name,
    location: row.location,
    signed: row.badge === "Signed",
    found: row.available,
  };
}

export function useNativeTitlebar(
  titlebar: NativeTitlebar,
  openVisible: boolean,
  warningVisible: boolean,
  recents: readonly RecentDocument[],
  handlers: TitlebarHandlers,
) {
  const { t } = useTranslation();

  useEffect(() => {
    titlebar.show({
      openVisible,
      warningVisible,
      labels: {
        open: t("tabs.openPdf"),
        openTooltip: t("tabs.openPdfShortcut"),
        warning: t("header.attention"),
        menu: t("header.menu"),
        status: t("status.title"),
        preferences: t("header.preferences"),
        feedback: t("header.help"),
        about: t("header.about"),
        recents: t("recents.heading"),
        clearRecents: t("recents.clear"),
        notFound: t("recents.missing"),
      },
      recents: recents.map(recentEntry),
    });
  }, [titlebar, openVisible, warningVisible, recents, t]);

  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(
    () =>
      titlebar.onAction((received: TitlebarAction) => {
        if (received.action === "recent") latest.current.recent(received.path);
        else latest.current[received.action]();
      }),
    [titlebar],
  );
}
