import { useTranslation } from "react-i18next";

/**
 * La ventana vacía del andamiaje (#47), ahora sobre el sistema de diseño y el
 * catálogo de cadenas (#55).
 *
 * NO añadas aquí pantallas ni componentes de producto: los aportan los
 * sub-issues siguientes de #46. Lo que sí es obligatorio a partir de ahora es
 * la forma: `.rf-root` en la raíz de la pantalla —la pone `index.html`—,
 * clases `rf-*` del sistema de diseño, y **ninguna cadena escrita en línea**:
 * todo texto sale del catálogo por su clave.
 */
export function App() {
  const { t } = useTranslation();

  return (
    <main className="rf-section" aria-label={t("app.name")}>
      <h1 className="rf-heading">{t("app.name")}</h1>
    </main>
  );
}
