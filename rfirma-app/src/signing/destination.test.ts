import { describe, expect, it } from "vitest";
import { FOLDER_BUDGET, NAME_BUDGET, shortenDestination } from "./destination";

/**
 * El recorte se prueba **como función pura**, aparte del pie que lo pinta
 * (TD-13): son reglas sobre cadenas, y comprobarlas por la pantalla obligaría a
 * medir píxeles para saber si una `…` está donde toca.
 */
describe("shortenDestination", () => {
  const shorten = (folder: string, name: string) => shortenDestination({ folder, name });

  it("leaves a name that already fits untouched", () => {
    expect(shorten("Documentos", "contrato-firmado.pdf")).toEqual({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
    });
  });

  it("cuts a long name through the middle and keeps its extension", () => {
    const { name } = shorten("Documentos", `${"a".repeat(60)}.pdf`);

    expect(name).toHaveLength(NAME_BUDGET);
    expect(name).toContain("…");
    expect(name.endsWith(".pdf")).toBe(true);
    expect(name.startsWith("aaaa")).toBe(true);
  });

  it("keeps the -firmado suffix when it cuts the name", () => {
    const { name } = shorten(
      "Documentos",
      `contrato-de-arrendamiento-${"largo-".repeat(6)}firmado.pdf`,
    );

    expect(name).toHaveLength(NAME_BUDGET);
    expect(name.endsWith("-firmado.pdf")).toBe(true);
    expect(name.startsWith("contrato-de-")).toBe(true);
  });

  it("keeps the tie-breaking number of the -firmado suffix", () => {
    // El número es la respuesta a «¿voy a machacar el anterior?»: perderlo en
    // el recorte es perder justo lo que se estaba mirando.
    const { name } = shorten("Documentos", `contrato-${"muy-".repeat(12)}firmado-2.pdf`);

    expect(name).toHaveLength(NAME_BUDGET);
    expect(name.endsWith("-firmado-2.pdf")).toBe(true);
  });

  it("cuts a long folder by its tail and never through the middle", () => {
    const { folder } = shorten("Documentos-del-ayuntamiento-de-la-ciudad", "contrato.pdf");

    expect(folder).toHaveLength(FOLDER_BUDGET);
    expect(folder.endsWith("…")).toBe(true);
    expect(folder.startsWith("Documentos-del-")).toBe(true);
    expect(folder.slice(0, -1)).not.toContain("…");
  });

  it("keeps the tail when there is no room left for the trunk", () => {
    const { name } = shortenDestination(
      { folder: "Documentos", name: "contrato-firmado-2.pdf" },
      { name: 4 },
    );

    expect(name).toBe("…-firmado-2.pdf");
  });
});
