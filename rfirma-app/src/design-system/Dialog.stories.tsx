//! La historia de `Dialog`: un diálogo con dos salidas y otro sin ninguna.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { Button } from "./Button";
import { Dialog } from "./Dialog";
import { Row } from "./Row";

const meta = {
  title: "Sistema de diseño/Dialog",
  component: Dialog,
  parameters: { layout: "centered" },
  decorators: [inDialogWindow],
  argTypes: { role: { control: "select", options: ["dialog", "alertdialog"] } },
  args: { label: "¿Firmar de todos modos?" },
} satisfies Meta<typeof Dialog>;

export default meta;

export const Closable: StoryObj<typeof meta> = {
  args: {
    onClose: () => {},
    children: (
      <>
        <p className="rf-title">¿Firmar de todos modos?</p>
        <p className="rf-prose">El documento tiene firmas que no son válidas.</p>
        <Row>
          <Button variant="ghost">Cancelar</Button>
          <Button variant="primary">Firmar</Button>
        </Row>
      </>
    ),
  },
};

export const WithoutExit: StoryObj<typeof meta> = {
  args: {
    label: "Firmando",
    children: <p className="rf-title">Firmando</p>,
  },
};
