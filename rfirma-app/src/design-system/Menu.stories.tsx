//! La historia de `Menu`: un menú anclado a su botón, abierto o cerrado, con dos grupos de entradas y una de contenido rico.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useId, useRef, useState } from "react";
import { Button } from "./Button";
import { Menu, MenuItem } from "./Menu";

function MenuDemo({ startsOpen }: { startsOpen: boolean }) {
  const [open, setOpen] = useState(startsOpen);
  const menuId = useId();
  const anchor = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const close = () => setOpen(false);
  return (
    <div ref={anchor} style={{ display: "inline-block", minWidth: 260 }}>
      <Button
        ref={trigger}
        variant="secondary"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => setOpen((was) => !was)}
      >
        Acciones
      </Button>
      <Menu
        open={open}
        onClose={close}
        anchorRef={anchor}
        returnFocusRef={trigger}
        id={menuId}
        aria-label="Acciones"
      >
        <MenuItem onClick={close}>
          <span style={{ display: "flex", flex: 1, flexDirection: "column", gap: 3 }}>
            <span>Solicitud de subvención.pdf</span>
            <small className="rf-text-muted">Documentos</small>
          </span>
          <small className="rf-text-muted">hoy</small>
        </MenuItem>
        <hr className="rf-divider" />
        <MenuItem onClick={close}>Preferencias…</MenuItem>
        <MenuItem onClick={close}>Acerca de</MenuItem>
      </Menu>
    </div>
  );
}

const meta = {
  title: "Sistema de diseño/Menu",
  component: MenuDemo,
  args: { startsOpen: true },
  argTypes: { startsOpen: { control: "boolean" } },
} satisfies Meta<typeof MenuDemo>;

export default meta;

export const Open: StoryObj<typeof meta> = {};
export const Closed: StoryObj<typeof meta> = { args: { startsOpen: false } };
