import type { Decorator, Preview } from "@storybook/react-vite";
import "../src/design-system/index.css";
import "../src/app.css";
import { DesignRoot } from "../src/design-system/DesignRoot";
import { LANGUAGES } from "../src/i18n/languages";

const withCatalogAndTheme: Decorator = (Story, { globals }) => (
  <DesignRoot language={globals.language} theme={globals.theme === "dark" ? "dark" : "light"}>
    <Story />
  </DesignRoot>
);

const preview: Preview = {
  decorators: [withCatalogAndTheme],
  parameters: {
    layout: "centered",
    options: {
      storySort: {
        method: "alphabetical",
        order: ["Primitivos", "Dominio", "Flujos", "Pantallas"],
      },
    },
  },
  initialGlobals: { language: "es", theme: "light" },
  globalTypes: {
    language: {
      description: "Idioma",
      toolbar: {
        title: "Idioma",
        icon: "globe",
        dynamicTitle: true,
        items: LANGUAGES.map((value) => ({ value, title: value })),
      },
    },
    theme: {
      description: "Tema",
      toolbar: {
        title: "Tema",
        icon: "paintbrush",
        dynamicTitle: true,
        items: [
          { value: "light", title: "Claro" },
          { value: "dark", title: "Oscuro" },
        ],
      },
    },
  },
};

export default preview;
