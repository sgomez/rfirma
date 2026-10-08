import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SigningPanel.stories";

const composed = composeStories(stories);
const {
  Ready,
  Unchosen,
  Searching,
  NoCertificates,
  SearchFailed,
  SeveralCertificates,
  VisibleSignatureOnePage,
  VisibleSignatureSeveralPages,
  VisibleSignatureEveryPage,
  VisibleSignatureOnAnotherPage,
  VisibleSignatureNotPlaced,
  VisibleSignatureWithoutCertificate,
  CompleteModelWithRubric,
  RubricOnlyModel,
  RubricWithoutImage,
  RubricFailed,
  CustomModel,
  PreviousSignaturesAllValid,
  PreviousSignaturesExpired,
  PreviousSignaturesWithProblems,
  PreviousSignaturesSameCertificate,
  PreviousSignaturesOtherCertificate,
  PreviousSignaturesOnlyAFinding,
  PreviousSignaturesMany,
  ClosedDocument,
  Signing,
  UnwritableDestination,
  LongDestinationName,
  SigningFailed,
  ReaderWithoutCard,
  ReadingTheCard,
  DnieReady,
  CardReady,
  UnreadableCard,
  NoCertificatesWithReader,
} = composed;

const text = (value: string) => screen.getByText(value);
const button = (name: string) => screen.getByRole("button", { name });
const combobox = () => screen.getByRole("combobox", { name: "Certificado" });
const notice = () => document.querySelector(".panel__co-signature");
const block = () => screen.getByRole("region", { name: "Firma visible" });

describe("the signing panel, by state", () => {
  it("never writes a wildcard in the interface, in any story", () => {
    for (const Story of Object.values(composed)) {
      const { unmount } = renderWithCatalog(<Story />);
      expect(document.body.textContent).not.toMatch(/\$\$/);
      unmount();
    }
  });

  describe("ready to sign", () => {
    it("has one primary button, «Firmar», and it is the last one", () => {
      renderWithCatalog(<Ready />);

      const primaries = [...document.querySelectorAll("button.rf-btn--primary")];
      expect(primaries.map((each) => each.textContent)).toEqual(["Firmar"]);
      expect(screen.getAllByRole("button").at(-1)).toBe(primaries[0]);
      expect(button("Firmar")).toBeEnabled();
      expect(button("Firmar")).not.toHaveAttribute("title");
    });

    it("puts the certificate selector first and has no split button in the footer", () => {
      renderWithCatalog(<PreviousSignaturesAllValid />);

      const scroll = combobox().closest(".panel__scroll");
      expect(scroll?.firstElementChild?.contains(combobox())).toBe(true);
      const footer = button("Firmar").closest("footer") as HTMLElement;
      expect(within(footer).queryByRole("combobox")).not.toBeInTheDocument();
      expect(screen.queryByText(/Firmar como|Elegir certificado/)).not.toBeInTheDocument();
    });

    it("offers the switch off, with no placement, and no co-signature notice", () => {
      renderWithCatalog(<Ready />);

      const toggle = screen.getByRole("switch", { name: "Firma visible" });
      expect(toggle.closest(".switch")).toHaveClass("switch--trailing");
      expect(notice()).toBeNull();
      expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
    });
  });

  describe("the destination", () => {
    it("shows the folder and the file name on their own lines, with the full text in the title", () => {
      renderWithCatalog(<Ready />);

      expect(text("Guardar en")).toBeInTheDocument();
      expect(button("Cambiar")).toBeInTheDocument();
      expect(text("Documentos").closest("[title]")).toHaveAttribute("title", "Documentos");
      expect(text("contrato-firmado.pdf").closest("[title]")).toHaveAttribute(
        "title",
        "contrato-firmado.pdf",
      );
      expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
    });

    it("shortens a long name through the middle and keeps its suffix and extension", () => {
      renderWithCatalog(<LongDestinationName />);

      const shown = screen.getByText(/contrato-de-/);
      expect(shown.textContent).toContain("…");
      expect(shown.textContent?.endsWith("-firmado-2.pdf")).toBe(true);
    });

    it("keeps «Guardar en» and the sign button when it cannot be written to, naming the folder", () => {
      renderWithCatalog(<UnwritableDestination />);

      const message = screen.getByText(
        (_, element) => element?.textContent === "No se puede escribir en Documentos",
        { selector: "span" },
      );
      expect(message).toHaveAttribute("title", "Documentos");
      expect(screen.getByText("Documentos", { selector: "strong" })).toBeInTheDocument();
      expect(text("Guardar en")).toBeInTheDocument();
      expect(button("Cambiar")).toBeInTheDocument();
      expect(button("Firmar")).toBeEnabled();
    });
  });

  describe("the reader line", () => {
    const cases = [
      { Story: ReaderWithoutCard, said: "Lector conectado, sin tarjeta" },
      { Story: ReadingTheCard, said: "Leyendo la tarjeta…" },
      { Story: DnieReady, said: "DNIe listo" },
      { Story: CardReady, said: "Tarjeta lista" },
      { Story: UnreadableCard, said: "rFirma no puede leer la tarjeta del lector" },
    ] as const;

    for (const { Story, said } of cases) {
      it(`says «${said}» right below the selector`, () => {
        renderWithCatalog(<Story />);

        const line = text(said);
        expect(line.closest("[role=status]")).not.toBeNull();
        expect(
          combobox().compareDocumentPosition(line) & Node.DOCUMENT_POSITION_FOLLOWING,
        ).toBeTruthy();
      });
    }

    it("draws no line without a reader", () => {
      renderWithCatalog(<Ready />);

      expect(document.querySelector(".reader-line")).toBeNull();
      for (const { said } of cases) expect(screen.queryByText(said)).toBeNull();
    });

    it("turns the indicator only while the card is read", () => {
      const { unmount } = renderWithCatalog(<ReadingTheCard />);
      expect(document.querySelector(".reader-line__spinner")).not.toBeNull();
      unmount();

      renderWithCatalog(<DnieReady />);
      expect(document.querySelector(".reader-line__spinner")).toBeNull();
    });

    it("keeps the other certificates to choose while the card is read", () => {
      renderWithCatalog(<ReadingTheCard />);

      expect(combobox()).toBeEnabled();
    });

    it("goes under the disabled selector, before the ways out, when there are no certificates", () => {
      renderWithCatalog(<NoCertificatesWithReader />);

      const line = text("Lector conectado, sin tarjeta");
      expect(combobox()).toBeDisabled();
      expect(
        combobox().compareDocumentPosition(line) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
      expect(
        line.compareDocumentPosition(button("Añadir un certificado…")) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    });
  });

  describe("the certificate", () => {
    const cases = [
      { Story: Unchosen, said: "Elige un certificado", firmar: "disabled" },
      { Story: SeveralCertificates, said: "Analytical Engines S.L.", firmar: "enabled" },
      { Story: Searching, said: "Buscando certificados…", firmar: "disabled" },
    ] as const;

    for (const { Story, said, firmar } of cases) {
      it(`says «${said}» and leaves «Firmar» ${firmar}`, () => {
        renderWithCatalog(<Story />);

        expect(combobox()).toHaveTextContent(said);
        if (firmar === "disabled") {
          expect(button("Firmar")).toBeDisabled();
        } else {
          expect(button("Firmar")).toBeEnabled();
        }
        expect(screen.queryByText("Sin certificados")).not.toBeInTheDocument();
      });
    }

    it("keeps the selector in place, disabled, with the two ways out right below it", () => {
      renderWithCatalog(<NoCertificates />);

      expect(combobox()).toBeDisabled();
      expect(combobox()).toHaveTextContent("Sin certificados");
      const select = combobox();
      const add = button("Añadir un certificado…");
      const again = button("Volver a buscar");
      const footer = button("Firmar").closest("footer") as HTMLElement;
      expect(select.compareDocumentPosition(add) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(add.compareDocumentPosition(again) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(footer).not.toContainElement(add);
      expect(footer).not.toContainElement(again);
      expect(add).toHaveClass("rf-btn--primary");
      expect(again).toHaveClass("rf-btn--secondary");
    });

    it("leaves only a disabled «Firmar» in the footer when none turned up", () => {
      renderWithCatalog(<NoCertificates />);

      const footer = button("Firmar").closest("footer") as HTMLElement;
      expect(
        within(footer)
          .getAllByRole("button")
          .map((each) => each.textContent),
      ).toEqual(expect.arrayContaining(["Firmar"]));
      expect(within(footer).queryByRole("button", { name: "Volver a buscar" })).toBeNull();
      expect(button("Firmar")).toBeDisabled();
    });

    it("has the same shape when the search failed, with the error below the ways out", () => {
      renderWithCatalog(<SearchFailed />);

      expect(combobox()).toBeDisabled();
      expect(screen.queryByText("Sin certificados")).not.toBeInTheDocument();
      expect(button("Firmar")).toBeDisabled();
      const again = button("Volver a buscar");
      const error = screen.getByText("CKR_TOKEN_NOT_PRESENT");
      expect(again.compareDocumentPosition(error) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      const footer = button("Firmar").closest("footer") as HTMLElement;
      expect(footer).not.toContainElement(again);
    });

    it("keeps the visible-signature switch off and disabled, with a notice, until one is chosen", () => {
      renderWithCatalog(<VisibleSignatureWithoutCertificate />);

      const toggle = screen.getByRole("switch", { name: "Firma visible" });
      expect(toggle).toHaveAttribute("aria-checked", "false");
      expect(toggle).toBeDisabled();
      expect(text("Elige antes un certificado.")).toBeInTheDocument();
      expect(screen.queryByRole("radio", { name: "Completa" })).not.toBeInTheDocument();
    });

    it("frees the switch, with no notice, once one is chosen", () => {
      renderWithCatalog(<Ready />);

      expect(screen.getByRole("switch", { name: "Firma visible" })).toBeEnabled();
      expect(screen.queryByText("Elige antes un certificado.")).not.toBeInTheDocument();
    });
  });

  describe("the co-signature notice", () => {
    const cases = [
      {
        Story: PreviousSignaturesAllValid,
        line: "Junto a 2 firmas",
        problems: null,
        tone: "valid",
      },
      {
        Story: PreviousSignaturesExpired,
        line: "Junto a 2 firmas",
        problems: "1 caducada",
        tone: "expired",
      },
      {
        Story: PreviousSignaturesWithProblems,
        line: "Junto a 3 firmas",
        problems: "3 problemas",
        tone: "invalid",
      },
      {
        Story: PreviousSignaturesOnlyAFinding,
        line: "Junto a 1 firma",
        problems: "1 problema",
        tone: "invalid",
      },
    ] as const;

    for (const { Story, line, problems, tone } of cases) {
      it(`reads «${line}${problems ? ` · ${problems}` : ""}» in the ${tone} tone`, () => {
        renderWithCatalog(<Story />);

        const textLine = document.querySelector(".panel__co-signature-text");
        expect(textLine).toHaveTextContent(problems === null ? line : `${line} · ${problems}`);
        if (problems !== null) {
          expect(screen.getByText(problems).tagName).toBe("STRONG");
        }
        expect(notice()).toHaveClass(`panel__co-signature--${tone}`);
      });
    }

    it("keeps to one line and one «Ver firmas →» button, whatever the number of signatures", () => {
      renderWithCatalog(<PreviousSignaturesMany />);

      expect(text("Junto a 6 firmas")).toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: "Ver firmas →" })).toHaveLength(1);
      expect(document.querySelector(".panel__previous-signatures-list")).not.toBeInTheDocument();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("says the document admits no more signatures when it is closed", () => {
      renderWithCatalog(<ClosedDocument />);

      expect(text("El documento no admite más firmas.")).toBeInTheDocument();
    });

    it("strips «Ya lo firmaste tú» with the same certificate", () => {
      renderWithCatalog(<PreviousSignaturesSameCertificate />);

      expect(text("Ya lo firmaste tú con este certificado")).toBeInTheDocument();
    });

    it("strips «con otro certificado tuyo» for a renewed one: same NIF and entity, other serial", () => {
      renderWithCatalog(<PreviousSignaturesOtherCertificate />);

      expect(text("Ya lo firmaste tú, con otro certificado tuyo")).toBeInTheDocument();
    });

    it("shows no strip when the earlier signatures are someone else's", () => {
      renderWithCatalog(<PreviousSignaturesAllValid />);

      expect(screen.queryByText(/Ya lo firmaste tú/)).not.toBeInTheDocument();
    });
  });

  describe("the visible signature", () => {
    it("chooses one page, several or all in a segmented group, in that order", () => {
      renderWithCatalog(<VisibleSignatureOnePage />);

      const group = screen.getByRole("radiogroup", { name: "En qué páginas" });
      expect(
        within(group)
          .getAllByRole("radio")
          .map((radio) => radio.parentElement?.textContent),
      ).toEqual(["Una página", "Varias", "Todas"]);
      expect(within(group).getByRole("radio", { name: "Una página" })).toBeChecked();
    });

    it("covers the model cards, «Con rúbrica» and loading one", () => {
      renderWithCatalog(<VisibleSignatureOnePage />);

      const models = screen.getByRole("group", { name: "Modelo" });
      for (const label of ["Completa", "Solo rúbrica", "Personalizada"]) {
        expect(within(models).getByRole("radio", { name: label })).toBeInTheDocument();
      }
      expect(screen.getByRole("switch", { name: "Con rúbrica" })).toBeInTheDocument();
      expect(button("Cargar…")).toBeInTheDocument();
    });

    it("says the page under «one page», with nothing to press while looking at it", () => {
      renderWithCatalog(<VisibleSignatureOnePage />);

      expect(within(block()).getByText("En la página 3")).toBeInTheDocument();
      expect(within(block()).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
    });

    it("offers to move it here when looking at another page under «one page»", () => {
      renderWithCatalog(<VisibleSignatureOnAnotherPage />);

      expect(within(block()).getByText("En la página 3")).toBeInTheDocument();
      expect(within(block()).getByRole("button", { name: "Ponerla aquí" })).toBeInTheDocument();
    });

    it("shows the field with the pages under «several», never saying «sellar»", () => {
      renderWithCatalog(<VisibleSignatureSeveralPages />);

      expect(screen.getByRole("textbox", { name: "Páginas de la firma visible" })).toHaveValue(
        "1,6",
      );
      expect(within(block()).queryByText(/Se sellará|mismo recuadro/)).not.toBeInTheDocument();
      expect(block().textContent).not.toMatch(/sell/i);
    });

    it("shows nothing about pages under «all»", () => {
      renderWithCatalog(<VisibleSignatureEveryPage />);

      expect(within(block()).queryByRole("button", { name: /aquí/ })).not.toBeInTheDocument();
      expect(within(block()).queryByRole("textbox")).not.toBeInTheDocument();
      expect(within(block()).queryByText(/En la página/)).not.toBeInTheDocument();
    });

    it("signs with the switch on even before anything is placed, never asking to place it", () => {
      renderWithCatalog(<VisibleSignatureNotPlaced />);

      expect(button("Firmar")).toBeEnabled();
      expect(screen.queryByText(/Coloca la firma/)).not.toBeInTheDocument();
    });

    it("signs invisibly with the switch off, and shows none of the page modes", () => {
      renderWithCatalog(<Ready />);

      expect(button("Firmar")).toBeEnabled();
      expect(screen.queryByRole("radiogroup")).not.toBeInTheDocument();
    });
  });

  describe("the model and the rubric", () => {
    it("shows the rubric already normalized, over white, in the thumbnail", () => {
      renderWithCatalog(<CompleteModelWithRubric />);

      expect(screen.getByAltText("Tu rúbrica")).toHaveAttribute(
        "src",
        expect.stringContaining("data:image/svg+xml"),
      );
    });

    it("locks «Con rúbrica» on when the rubric-only card is chosen", () => {
      renderWithCatalog(<RubricOnlyModel />);

      const rubricSwitch = screen.getByRole("switch", { name: "Con rúbrica" });
      expect(rubricSwitch).toBeDisabled();
      expect(rubricSwitch).toHaveAttribute("aria-checked", "true");
    });

    it("disables the rubric-only card while «Con rúbrica» is off", () => {
      renderWithCatalog(<VisibleSignatureOnePage />);

      expect(screen.getByRole("radio", { name: "Solo rúbrica" })).toBeDisabled();
    });

    it("shows a dashed hole in place of the thumbnail once switched on without an image", () => {
      renderWithCatalog(<RubricWithoutImage />);

      expect(screen.getAllByTitle("Sin rúbrica cargada").length).toBeGreaterThan(0);
      expect(button("Cargar…")).toBeInTheDocument();
    });

    it("counts the rubric failure with the raw detail apart", () => {
      renderWithCatalog(<RubricFailed />);

      expect(text("No se ha podido usar la imagen")).toBeInTheDocument();
      expect(text("formato no admitido")).toBeInTheDocument();
    });

    it("sketches the phrase in the custom card instead of printing it", () => {
      renderWithCatalog(<CustomModel />);

      const card = screen.getByRole("radio", { name: "Personalizada" }).closest("label");
      const thumbnail = card?.querySelector(".panel__model-thumbnail");
      expect(thumbnail?.textContent).toBe("");
      expect(thumbnail?.querySelector(".panel__model-sketch")).not.toBeNull();
    });
  });

  describe("signing", () => {
    it("says «Firmando…», lets nothing be pressed, and dims the controls", () => {
      renderWithCatalog(<Signing />);

      expect(button("Firmando…")).toBeDisabled();
      expect(combobox()).toBeDisabled();
      expect(
        screen.getByRole("switch", { name: "Firma visible" }).closest(".panel__toggle"),
      ).toHaveClass("panel__toggle--dim");
      expect(screen.getByRole("radiogroup").closest(".panel__controls--dim")).not.toBeNull();
      expect(button("Cambiar")).toHaveClass("panel__controls--dim");
    });

    it("does not dim anything otherwise", () => {
      renderWithCatalog(<VisibleSignatureOnePage />);

      expect(
        screen.getByRole("switch", { name: "Firma visible" }).closest(".panel__toggle"),
      ).not.toHaveClass("panel__toggle--dim");
      expect(screen.getByRole("radiogroup").closest(".panel__controls--dim")).toBeNull();
    });
  });

  describe("a failure to sign", () => {
    it("shows the situation as the cause with the raw CKR apart, and swaps the footer", () => {
      renderWithCatalog(<SigningFailed />);

      expect(text("No se ha podido firmar")).toBeInTheDocument();
      expect(text("Falta la tarjeta o el certificado")).toBeInTheDocument();
      expect(text("CKR_DEVICE_REMOVED durante C_Sign (fase: firma)")).toBeInTheDocument();
      expect(text("El documento sigue como estaba: no se ha guardado nada.")).toBeInTheDocument();
      expect(button("Copiar detalle")).toBeInTheDocument();
      expect(button("Reintentar")).toBeInTheDocument();
      expect(button("Volver")).toBeInTheDocument();
      expect(text("contrato-firmado.pdf")).toBeInTheDocument();
      expect(screen.queryByText("Firma visible")).not.toBeInTheDocument();
    });
  });
});
