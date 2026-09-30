/** Lo que la ventana manda a la barra de título nativa y a dónde lleva cada acción que vuelve. */

import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { NativeTitlebar, TitlebarAction } from "./shell/nativeTitlebar";

export type TitlebarHandlers = Record<TitlebarAction, () => void>;

export function useNativeTitlebar(
  titlebar: NativeTitlebar,
  openVisible: boolean,
  warningVisible: boolean,
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
        status: t("header.status"),
        preferences: t("header.preferences"),
        feedback: t("header.help"),
        about: t("header.about"),
      },
    });
  }, [titlebar, openVisible, warningVisible, t]);

  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(() => titlebar.onAction((action) => latest.current[action]()), [titlebar]);
}
