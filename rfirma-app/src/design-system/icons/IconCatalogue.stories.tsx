//! El catálogo de iconos: cada papel dibujado por cada familia candidata, lado a lado, con su origen y su licencia.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { ICON_FAMILIES } from "./families";
import { ICON_NAMES } from "./names";

const CELL = { padding: "6px 16px", textAlign: "left" } as const;

function IconCatalogue() {
  return (
    <table className="rf-prose" style={{ borderCollapse: "collapse" }}>
      <thead>
        <tr>
          <th scope="col" style={CELL}>
            Papel
          </th>
          {ICON_FAMILIES.map((family) => (
            <th key={family.name} scope="col" style={CELL}>
              {family.name}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {ICON_NAMES.map((name) => (
          <tr key={name}>
            <th scope="row" style={CELL}>
              <code>{name}</code>
            </th>
            {ICON_FAMILIES.map((family) => {
              const { Glyph, origin } = family.glyphs[name];
              return (
                <td key={family.name} style={CELL}>
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                    <Glyph size={24} />
                    {`${origin.family} · ${origin.license}`}
                  </span>
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const meta = {
  title: "Pantallas/Catálogo de iconos",
  component: IconCatalogue,
} satisfies Meta<typeof IconCatalogue>;

export default meta;

export const Catalogue: StoryObj<typeof meta> = {};
