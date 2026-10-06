//! La historia de `Dialog`: un diálogo con dos salidas, otro con la primaria en el foco y otro sin ninguna salida.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useRef } from "react";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import type { PrimaryButton } from "./actionKeys";
import { Button } from "./Button";
import { Dialog, type DialogProps } from "./Dialog";
import { Row } from "./Row";

const meta = {
  title: "Primitivos/Dialog",
  component: Dialog,
  parameters: {
    layout: "centered",
    designSync: { cardMode: "single", primaryStory: "Closable", viewport: "1340x780" },
  },
  decorators: [inDialogWindow],
  argTypes: { role: { control: "select", options: ["dialog", "alertdialog"] } },
  args: { label: "¿Firmar de todos modos?" },
} satisfies Meta<typeof Dialog>;

export default meta;

function ConfirmBody({ primary, onCancel }: { primary?: PrimaryButton; onCancel?: () => void }) {
  return (
    <>
      <p className="rf-title">¿Firmar de todos modos?</p>
      <p className="rf-prose">El documento tiene firmas que no son válidas.</p>
      <Row style={{ justifyContent: "flex-end" }}>
        <Button variant="ghost" onClick={onCancel}>
          Cancelar
        </Button>
        <Button variant="primary" ref={primary}>
          Firmar
        </Button>
      </Row>
    </>
  );
}

export const Closable: StoryObj<typeof meta> = {
  args: { onClose: () => {}, children: <ConfirmBody /> },
};

function ConfirmWithPrimary(args: DialogProps) {
  const primary = useRef<HTMLButtonElement>(null);
  return (
    <Dialog {...args} primary={primary}>
      <ConfirmBody primary={primary} onCancel={args.onClose} />
    </Dialog>
  );
}

export const WithPrimary: StoryObj<typeof meta> = {
  args: { onClose: () => {} },
  render: (args) => <ConfirmWithPrimary {...args} />,
};

export const WithoutExit: StoryObj<typeof meta> = {
  args: {
    label: "Firmando",
    children: <p className="rf-title">Firmando</p>,
  },
};
