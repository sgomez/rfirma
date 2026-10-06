//! La historia de `Tabs`: una tira de pestañas de las que la persona activa una, con otra bloqueada.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { Tab, Tabs } from "./Tabs";

function TabsDemo({ withDisabled }: { withDisabled: boolean }) {
  const [active, setActive] = useState("solicitud");
  const names = ["solicitud", "memoria", "anexo"];
  return (
    <Tabs aria-label="Documentos abiertos">
      {names.map((name) => (
        <Tab
          key={name}
          selected={name === active}
          disabled={withDisabled && name === "anexo"}
          onClick={() => setActive(name)}
        >
          {name}.pdf
        </Tab>
      ))}
    </Tabs>
  );
}

const meta = {
  title: "Primitivos/Tabs",
  component: TabsDemo,
  args: { withDisabled: false },
  argTypes: { withDisabled: { control: "boolean" } },
} satisfies Meta<typeof TabsDemo>;

export default meta;

export const Default: StoryObj<typeof meta> = {};
export const WithDisabledTab: StoryObj<typeof meta> = { args: { withDisabled: true } };
