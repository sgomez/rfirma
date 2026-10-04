//! Las historias de la ventana principal: sin documentos, con la franja de versión nueva, con el aviso y con la barra de título nativa.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyRecents } from "../../.storybook/fixtures/documents";
import { DocumentTabs } from "../documents/DocumentTabs";
import { RecentsSection } from "../documents/RecentRows";
import { NewVersionStrip } from "../updates/NewVersionStrip";
import { DocumentViewer } from "../viewer/DocumentViewer";
import { MainWindow } from "./MainWindow";

const noTabs = (
  <DocumentTabs
    tabs={[]}
    activeId={null}
    recents={storyRecents}
    onActivate={fn()}
    onClose={fn()}
    onOpen={fn()}
    onSelectRecent={fn()}
    onClearRecents={fn()}
  />
);

const emptyViewer = (
  <DocumentViewer
    pdf={null}
    placement={null}
    onOpen={fn()}
    emptyExtra={<RecentsSection recents={storyRecents} onSelect={fn()} onClear={fn()} />}
  />
);

const meta = {
  title: "Ventana principal/1 · Ventana",
  component: MainWindow,
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div
        style={{
          width: 1100,
          height: 640,
          position: "relative",
          transform: "translateZ(0)",
          overflow: "hidden",
          border: "1px solid var(--rf-border-subtle)",
          borderRadius: "var(--rf-radius-lg)",
          boxShadow: "var(--rf-shadow-elevated)",
        }}
      >
        <Story />
      </div>
    ),
  ],
  args: {
    menuAnchor: "header",
    onOpenPreferences: fn(),
    onOpenAbout: fn(),
    onOpenStatus: fn(),
    onOpenHelp: fn(),
    tabs: noTabs,
    viewer: emptyViewer,
    panel: null,
  },
} satisfies Meta<typeof MainWindow>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Empty: Story = {};

export const WithAttention: Story = { args: { hasAttention: true } };

export const NewVersionInstallable: Story = {
  args: {
    notification: (
      <NewVersionStrip
        newVersion={{ version: "0.4.1", installable: true }}
        onOpen={fn()}
        onDismiss={fn()}
      />
    ),
  },
};

export const NewVersionNotInstallable: Story = {
  args: {
    notification: (
      <NewVersionStrip
        newVersion={{ version: "0.4.1", installable: false }}
        onOpen={fn()}
        onDismiss={fn()}
      />
    ),
  },
};

export const NativeTitlebar: Story = { args: { menuAnchor: "titlebar" } };
