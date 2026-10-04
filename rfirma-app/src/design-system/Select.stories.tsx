//! La historia de `Select`: el desplegable de la aplicación, cerrado, con el rótulo oculto y desplegado hacia arriba.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { Select } from "./Select";

const options = [
  { value: "system", label: "El del sistema" },
  { value: "light", label: "Claro" },
  { value: "dark", label: "Oscuro" },
] as const;

type Theme = (typeof options)[number]["value"];

function SelectDemo({ hideLabel, opens }: { hideLabel: boolean; opens: "down" | "up" }) {
  const [value, setValue] = useState<Theme>("light");
  return (
    <div style={{ minWidth: 260, paddingTop: opens === "up" ? 160 : 0 }}>
      <Select
        label="Tema"
        hideLabel={hideLabel}
        value={value}
        options={options}
        onChange={setValue}
        opens={opens}
      />
    </div>
  );
}

const meta = {
  title: "Sistema de diseño/Select",
  component: SelectDemo,
  args: { hideLabel: false, opens: "down" },
  argTypes: {
    hideLabel: { control: "boolean" },
    opens: { control: "inline-radio", options: ["down", "up"] },
  },
} satisfies Meta<typeof SelectDemo>;

export default meta;

export const Closed: StoryObj<typeof meta> = {};
export const HiddenLabel: StoryObj<typeof meta> = { args: { hideLabel: true } };
export const OpensUp: StoryObj<typeof meta> = { args: { opens: "up" } };
