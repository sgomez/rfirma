import type { Decorator, Preview } from "@storybook/react-vite";
import "../src/design-system/index.css";
import "../src/app.css";
import { LANGUAGES } from "../src/i18n/languages";
import { applyTheme } from "../src/preferences/theme";
import { CatalogProvider } from "../src/testing/render";

const withCatalogAndTheme: Decorator = (Story, { globals }) => {
  applyTheme(globals.theme === "dark" ? "dark" : "light", () => {});
  return (
    <CatalogProvider language={globals.language}>
      <div className="rf-root">
        <Story />
      </div>
    </CatalogProvider>
  );
};

const preview: Preview = {
  decorators: [withCatalogAndTheme],
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
