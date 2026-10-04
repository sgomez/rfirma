//! Las historias de `CertificateCard`, una por variante de certificado.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { CertificateCard } from "./CertificateCard";
import type { Certificate } from "./certificate";

const IN_2020 = 1_579_046_400;
const IN_2030 = 1_893_456_000;
const IN_2099 = 4_070_908_800;

function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef",
    label: "FNMT-GEMELO",
    holderName: "LOVELACE BYRON ADA",
    stampedSigner: "LOVELACE BYRON ADA - NIF 0000****T",
    givenName: "Ada",
    surname: "Lovelace Byron",
    idNumber: "IDCES-00000000T",
    organizationIdentifier: null,
    entityName: null,
    issuer: "FNMT-RCM",
    certificateSerialNumber: "1234567890",
    stores: ["card"],
    status: { kind: "valid", notAfter: IN_2030 },
    remembered: false,
    ...overrides,
  };
}

const meta = {
  title: "Firma/CertificateCard",
  component: CertificateCard,
  args: { certificate: aCertificate() },
} satisfies Meta<typeof CertificateCard>;

export default meta;

export const Personal: StoryObj<typeof meta> = {};

export const OnBehalfOfAnEntity: StoryObj<typeof meta> = {
  args: {
    certificate: aCertificate({
      entityName: "Analytical Engines S.L.",
      organizationIdentifier: "VATES-B00000000",
    }),
  },
};

export const Expired: StoryObj<typeof meta> = {
  args: { certificate: aCertificate({ status: { kind: "expired", notAfter: IN_2020 } }) },
};

export const CannotSign: StoryObj<typeof meta> = {
  args: {
    certificate: aCertificate({ status: { kind: "notYetValid", notBefore: IN_2099 } }),
  },
};

export const SeveralStores: StoryObj<typeof meta> = {
  args: { certificate: aCertificate({ stores: ["firefox", "nssdb", "installed"] }) },
};
