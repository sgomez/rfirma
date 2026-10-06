//! La historia de `SplitButton`: la acción de abrir con la flecha de los recientes, y la acción sola.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { MenuItem } from "./Menu";
import { SplitButton } from "./SplitButton";

function SplitButtonDemo({ withItems }: { withItems: boolean }) {
  return (
    <div style={{ minHeight: 160 }}>
      <SplitButton
        onAction={() => {}}
        menuLabel="Abiertos recientemente"
        items={
          withItems ? (
            <>
              <MenuItem>Solicitud de subvención.pdf</MenuItem>
              <MenuItem>Memoria técnica.pdf</MenuItem>
            </>
          ) : undefined
        }
      >
        Abrir PDF…
      </SplitButton>
    </div>
  );
}

const meta = {
  title: "Primitivos/SplitButton",
  component: SplitButtonDemo,
  args: { withItems: true },
  argTypes: { withItems: { control: "boolean" } },
} satisfies Meta<typeof SplitButtonDemo>;

export default meta;

export const WithMenu: StoryObj<typeof meta> = {};
export const ActionOnly: StoryObj<typeof meta> = { args: { withItems: false } };
