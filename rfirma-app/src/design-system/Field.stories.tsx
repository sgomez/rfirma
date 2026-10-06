//! La historia de `Field`: una etiqueta, un control y su ayuda.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { Field } from "./Field";

const meta = {
  title: "Primitivos/Field",
  component: Field,
  render: (args) => (
    <Field {...args}>
      <label className="rf-label" htmlFor="field-story-input">
        Nombre
      </label>
      <input className="rf-input" id="field-story-input" />
      <span className="rf-hint">Como aparece en el certificado</span>
    </Field>
  ),
} satisfies Meta<typeof Field>;

export default meta;

export const Default: StoryObj<typeof meta> = {};
