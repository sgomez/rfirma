//! Los dobles y ayudas de las pruebas de Preferencias (`defaults`, `renderView`, `openTab`, `anInstalledCertificate`), que usan también las de `App` y del asistente.

import { screen } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import { renderWithCatalog } from "../testing/render";
import { PreferencesView } from "./PreferencesView";
import { defaults } from "./preferencesFixtures";

const noop = async () => {};

export function renderView(props: Partial<Parameters<typeof PreferencesView>[0]> = {}) {
  return renderWithCatalog(
    <PreferencesView
      preferences={defaults}
      onChooseDestination={noop}
      onChange={noop}
      onForgetActivity={noop}
      installedCertificates={[]}
      onInstallCertificate={async () => true}
      onRemoveCertificate={noop}
      onEmptyStore={noop}
      onClose={noop}
      {...props}
    />,
  );
}

/** Pasa al panel de la pestaña dada, por su nombre en el índice. */
export async function openTab(user: UserEvent, name: string) {
  await user.click(screen.getByRole("tab", { name }));
}

export { anInstalledCertificate, defaults } from "./preferencesFixtures";
