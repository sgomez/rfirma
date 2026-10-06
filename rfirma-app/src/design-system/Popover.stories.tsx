//! La historia de `Popover`: un panel anclado a su botón, abierto o cerrado, con su cierre por Escape y por pulsar fuera.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useRef, useState } from "react";
import { Button } from "./Button";
import { Card } from "./Card";
import { Popover } from "./Popover";

function PopoverDemo({ startsOpen, inPortal }: { startsOpen: boolean; inPortal: boolean }) {
  const [open, setOpen] = useState(startsOpen);
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  return (
    <div ref={anchor} style={{ display: "inline-block", minWidth: 220 }}>
      <Button ref={trigger} variant="secondary" onClick={() => setOpen((was) => !was)}>
        Abrir panel
      </Button>
      <Popover
        open={open}
        onClose={() => setOpen(false)}
        anchorRef={anchor}
        returnFocusRef={trigger}
        tabIndex={-1}
        portal={inPortal ? { maxHeight: 240 } : undefined}
      >
        <Card elevated className="rf-body" style={{ marginTop: 4, padding: "8px 10px" }}>
          Contenido del panel
        </Card>
      </Popover>
    </div>
  );
}

const meta = {
  title: "Primitivos/Popover",
  component: PopoverDemo,
  args: { startsOpen: true, inPortal: false },
  argTypes: { startsOpen: { control: "boolean" }, inPortal: { control: "boolean" } },
} satisfies Meta<typeof PopoverDemo>;

export default meta;

export const Open: StoryObj<typeof meta> = {};
export const Closed: StoryObj<typeof meta> = { args: { startsOpen: false } };
export const InPortal: StoryObj<typeof meta> = { args: { inPortal: true } };
