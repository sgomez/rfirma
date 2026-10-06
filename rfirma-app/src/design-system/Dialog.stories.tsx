//! La historia de `Dialog`: un diálogo con dos salidas y el foco en su primaria, y otro sin ninguna.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { useDefaultButton } from "./actionKeys";
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

function ClosableDialog(args: DialogProps) {
  const sign = useDefaultButton();
  return (
    <Dialog {...args} primary={sign}>
      <p className="rf-title">¿Firmar de todos modos?</p>
      <p className="rf-prose">El documento tiene firmas que no son válidas.</p>
      <Row style={{ justifyContent: "flex-end" }}>
        <Button variant="ghost" onClick={args.onClose}>
          Cancelar
        </Button>
        <Button variant="primary" ref={sign}>
          Firmar
        </Button>
      </Row>
    </Dialog>
  );
}

export const Closable: StoryObj<typeof meta> = {
  args: { onClose: () => {} },
  render: (args) => <ClosableDialog {...args} />,
};

export const WithoutExit: StoryObj<typeof meta> = {
  args: {
    label: "Firmando",
    children: <p className="rf-title">Firmando</p>,
  },
};
