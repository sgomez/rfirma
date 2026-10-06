//! Las historias de la sede sin certificado utilizable: ninguno instalado o todos excluidos, desde una sede o una orden de terminal.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { sedeMomentMeta } from "../../.storybook/decorators/sedeWindow";
import type { NamedFailure } from "../errors/classify";
import type { NoCertificateReason } from "./errand";
import { SedeNoCertificate } from "./SedeNoCertificate";
import { momentStory, sedeErrand, sedeViewActions } from "./testing/fixtures/sedeView";

const meta = {
  title: "Flujos/Sede/Sin certificado",
  ...sedeMomentMeta,
  parameters: { ...sedeMomentMeta.parameters, designSync: { cardMode: "column" } },
  component: SedeNoCertificate,
  args: {
    origin: "sede.ejemplo.gob.es",
    terminal: false,
    failure: null,
    onInstall: sedeViewActions.onInstallCertificate,
    onLookAgain: sedeViewActions.onLookAgain,
    onLeave: sedeViewActions.onCancel,
  },
} satisfies Meta<typeof SedeNoCertificate>;

export default meta;

type Story = StoryObj<typeof meta>;

const terminalOrder = { documentPath: "/home/ada/contratos/convenio.pdf" };

const missing = (
  reason: NoCertificateReason,
  owned: number,
  extra: { failure?: NamedFailure; terminal?: boolean } = {},
): Story => {
  const { terminal = false, failure } = extra;
  const errand = sedeErrand(
    { kind: "noCertificate", reason, owned },
    terminal ? { origin: null, terminalOrder } : {},
  );
  return momentStory(
    { origin: errand.origin, reason, owned, terminal, ...(failure ? { failure } : {}) },
    errand,
  );
};

export const NoneInstalled = missing("none", 0);

export const ExcludedBySite = missing("excluded", 2);

export const ExcludedOnlyCertificate = missing("excluded", 1);

export const ExcludedManyCertificates = missing("excluded", 3);

export const InstallFailed = missing("none", 0, {
  failure: {
    situation: "pkcs12Unreadable",
    detail: "no se puede leer el fichero",
    attemptsLeft: null,
  },
});

export const TerminalNoneInstalled = missing("none", 0, { terminal: true });

export const TerminalExcludedByFilter = missing("excluded", 2, { terminal: true });
