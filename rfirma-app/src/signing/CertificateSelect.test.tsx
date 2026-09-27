import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { CertificateSelect } from "./CertificateSelect";
import type { Certificate } from "./certificate";

/**
 * **Grada A**: el selector son datos y una devolución de llamada; no habla con
 * el token.
 */
function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef0123456789abcdef",
    label: "Firma",
    holderName: "Ada Lovelace Byron",
    stampedSigner: "Ada Lovelace Byron",
    givenName: "Ada",
    surname: "Lovelace Byron",
    idNumber: "99999999R",
    organizationIdentifier: null,
    entityName: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1234567890",
    store: "card",
    stores: ["card"],
    status: { kind: "valid", notAfter: Date.UTC(2028, 2, 15, 12) / 1000 },
    remembered: false,
    ...overrides,
  };
}

const personal = aCertificate({ id: "personal", stores: ["firefox", "chrome"], store: "firefox" });
const representative = aCertificate({
  id: "representative",
  entityName: "Reformas Martín SL",
  organizationIdentifier: "B12345678",
  issuer: "AC Representación",
  stores: ["installed"],
  store: "installed",
});
const expired = aCertificate({
  id: "expired",
  holderName: "Beatriz Núñez",
  status: { kind: "expired", notAfter: Date.UTC(2025, 2, 3, 12) / 1000 },
});
const revoked = aCertificate({
  id: "revoked",
  holderName: "Carlos Peña",
  status: { kind: "revoked", reason: "keyCompromise" },
});

const everyKind: readonly Certificate[] = [expired, representative, revoked, personal];

function renderSelect(props: Partial<Parameters<typeof CertificateSelect>[0]> = {}) {
  const onChoose = vi.fn();
  const { container } = renderWithCatalog(
    <CertificateSelect certificates={everyKind} chosen={null} onChoose={onChoose} {...props} />,
  );
  return { onChoose, container };
}

const box = () => screen.getByRole("combobox", { name: "Certificado" });
const rows = () => screen.getAllByRole("option");

function row(index: number): HTMLElement {
  const found = rows()[index];
  if (found === undefined) throw new Error(`la lista no tiene fila ${index}`);
  return found;
}

describe("CertificateSelect", () => {
  describe("closed", () => {
    it("is labelled «Certificado» and says «Elige un certificado» while nothing is chosen", () => {
      renderSelect();

      expect(screen.getByText("Certificado")).toBeVisible();
      expect(box()).toHaveTextContent("Elige un certificado");
      expect(box()).toHaveAttribute("aria-expanded", "false");
    });

    it("shows a personal certificate as the holder and «A título personal · id number»", () => {
      renderSelect({ chosen: personal });

      expect(box()).toHaveTextContent("Ada Lovelace Byron");
      expect(box()).toHaveTextContent("A título personal · 99999999R");
    });

    it("shows a representative certificate company first, with the short second line", () => {
      renderSelect({ chosen: representative });

      expect(box()).toHaveTextContent("Reformas Martín SL");
      expect(box()).toHaveTextContent("Ada Lovelace Byron, representante");
      expect(box()).not.toHaveTextContent("B12345678");
    });

    it("says «Buscando certificados…» and does not open while the certificates are being listed", async () => {
      renderSelect({ certificates: [], searching: true });

      expect(box()).toHaveTextContent("Buscando certificados…");
      expect(box()).toBeDisabled();
      await userEvent.click(box());
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("does not open while disabled", async () => {
      renderSelect({ chosen: personal, disabled: true });

      expect(box()).toBeDisabled();
      await userEvent.click(box());
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });
  });

  describe("open", () => {
    it("turns the box into a focused search field and hangs the list below it", async () => {
      renderSelect();

      await userEvent.click(box());

      const search = screen.getByRole("combobox", { name: "Certificado" });
      expect(search).toHaveAttribute("placeholder", "Nombre, empresa, NIF o almacén");
      expect(search).toHaveFocus();
      expect(search).toHaveAttribute("aria-expanded", "true");
      expect(search).toHaveAttribute("aria-controls", screen.getByRole("listbox").id);
    });

    it("mounts the list in a portal outside its own container", async () => {
      const { container } = renderSelect();

      await userEvent.click(box());

      expect(container.querySelector('[role="listbox"]')).toBeNull();
      expect(document.body.querySelector('[role="listbox"]')).not.toBeNull();
    });

    it("always opens downwards, even with little room below the box", async () => {
      const { container } = renderSelect({ listMaxHeight: 300 });
      const frame = container.querySelector(".certificate-select__frame") as HTMLElement;
      vi.spyOn(frame, "getBoundingClientRect").mockReturnValue(
        DOMRect.fromRect({ x: 20, y: 300, width: 480, height: 52 }),
      );

      await userEvent.click(box());

      const layer = document.querySelector(".certificate-select__layer") as HTMLElement;
      expect(layer.style.top).toBe("356px");
      expect(layer.style.bottom).toBe("");
      expect(layer.style.width).toBe("480px");
      expect(layer.style.maxHeight).toBe("300px");
    });

    it("keeps the list inside the window, which is its ceiling", async () => {
      const { container } = renderSelect();
      const frame = container.querySelector(".certificate-select__frame") as HTMLElement;
      vi.spyOn(frame, "getBoundingClientRect").mockReturnValue(
        DOMRect.fromRect({ x: 20, y: window.innerHeight - 252, width: 332, height: 52 }),
      );

      await userEvent.click(box());

      const layer = document.querySelector(".certificate-select__layer") as HTMLElement;
      expect(layer.style.maxHeight).toBe("188px");
    });

    it("groups «Disponibles» over «No se pueden usar», alphabetical by first line", async () => {
      renderSelect();

      await userEvent.click(box());

      const available = screen.getByRole("group", { name: "Disponibles" });
      const unusable = screen.getByRole("group", { name: "No se pueden usar" });
      expect(within(available).getAllByRole("option")).toHaveLength(2);
      expect(rows().map((option) => option.textContent)).toEqual([
        expect.stringMatching(/^Ada Lovelace Byron/),
        expect.stringMatching(/^Reformas Martín SL/),
        expect.stringMatching(/^Beatriz Núñez/),
        expect.stringMatching(/^Carlos Peña/),
      ]);
      expect(within(unusable).getAllByRole("option")).toHaveLength(2);
    });

    it("shows no group header when there is a single certificate", async () => {
      renderSelect({ certificates: [personal] });

      await userEvent.click(box());

      expect(screen.queryByText("Disponibles")).not.toBeInTheDocument();
      expect(rows()).toHaveLength(1);
    });

    it("puts the company first on a representative row, with the entity's tax id", async () => {
      renderSelect();

      await userEvent.click(box());

      expect(row(1)).toHaveTextContent("Reformas Martín SL");
      expect(row(1)).toHaveTextContent("Ada Lovelace Byron, representante · B12345678");
      expect(row(0)).toHaveTextContent("A título personal · 99999999R");
    });

    it("tags every store the certificate is in, and says when it expires", async () => {
      renderSelect();

      await userEvent.click(box());

      expect(within(row(0)).getByText("Firefox")).toBeVisible();
      expect(within(row(0)).getByText("Chrome")).toBeVisible();
      expect(row(0)).toHaveTextContent("Caduca en 03/2028");
      expect(within(row(1)).getByText("Instalado en rFirma")).toBeVisible();
    });

    it("keeps the issuer for the tooltip, naming the other stores of the same certificate", async () => {
      renderSelect();

      await userEvent.click(box());

      expect(row(0)).toHaveAttribute(
        "title",
        "Emitido por AC FNMT Usuarios · el mismo certificado en Firefox y Chrome",
      );
      expect(row(1)).toHaveAttribute("title", "Emitido por AC Representación");
      expect(row(0)).not.toHaveTextContent("AC FNMT Usuarios");
    });

    it("lists an unusable certificate with its reason, in bold with its icon, and refuses it", async () => {
      const { onChoose } = renderSelect();
      await userEvent.click(box());

      const expiredRow = row(2);
      expect(expiredRow).toHaveAttribute("aria-disabled", "true");
      expect(expiredRow).toHaveTextContent("Caducó el 3 de marzo de 2025");
      expect(expiredRow).toHaveAttribute("title", "Caducó el 3 de marzo de 2025");
      expect(expiredRow.querySelector(".certificate-select__reason svg")).not.toBeNull();
      expect(row(3)).toHaveTextContent("Revocado (keyCompromise)");

      fireEvent.pointerDown(expiredRow);

      expect(onChoose).not.toHaveBeenCalled();
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    it("marks the chosen row", async () => {
      renderSelect({ chosen: representative });

      await userEvent.click(box());

      expect(row(1)).toHaveAttribute("aria-selected", "true");
      expect(row(0)).toHaveAttribute("aria-selected", "false");
    });
  });

  describe("choosing", () => {
    it("chooses the row that is pressed and closes", async () => {
      const { onChoose } = renderSelect();
      await userEvent.click(box());

      fireEvent.pointerDown(row(1));

      expect(onChoose).toHaveBeenCalledWith(representative);
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
      expect(box()).toHaveFocus();
    });

    it("tells two certificates with the same label apart by their handle", async () => {
      const twins = [
        aCertificate({ id: "aaaa", store: "chrome", stores: ["chrome"] }),
        aCertificate({ id: "bbbb", store: "firefox", stores: ["firefox"] }),
      ];
      const { onChoose } = renderSelect({ certificates: twins });
      await userEvent.click(box());

      fireEvent.pointerDown(row(1));

      expect(onChoose).toHaveBeenCalledWith(twins[1]);
    });

    it("closes when something outside is pressed", async () => {
      const { onChoose } = renderSelect();
      await userEvent.click(box());

      fireEvent.pointerDown(document.body);

      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
      expect(onChoose).not.toHaveBeenCalled();
    });
  });

  describe("search", () => {
    it("filters by company and heads the list with «N de M»", async () => {
      renderSelect();
      await userEvent.click(box());

      await userEvent.keyboard("reformas");

      expect(rows()).toHaveLength(1);
      expect(row(0)).toHaveTextContent("Reformas Martín SL");
      expect(screen.getByText("1 de 4")).toBeVisible();
    });

    it.each([
      ["the entity's tax id", "b1234"],
      ["the store", "instalado"],
      ["the issuer", "representación"],
      ["the holder, without accents", "nunez"],
    ])("filters by %s", async (_what, query) => {
      renderSelect();
      await userEvent.click(box());

      await userEvent.keyboard(query);

      expect(rows()).toHaveLength(1);
    });

    it("filters personal certificates by the word «personal», leaving out the representative one", async () => {
      renderSelect();
      await userEvent.click(box());

      await userEvent.keyboard("personal");

      expect(rows()).toHaveLength(3);
      expect(screen.queryByText("Reformas Martín SL")).not.toBeInTheDocument();
    });

    it("says «Ningún certificado coincide» when nothing matches", async () => {
      renderSelect();
      await userEvent.click(box());

      await userEvent.keyboard("zzz");

      expect(screen.queryAllByRole("option")).toHaveLength(0);
      expect(screen.getByText("Ningún certificado coincide")).toBeVisible();
    });

    it("empties the search after choosing", async () => {
      renderSelect();
      await userEvent.click(box());
      await userEvent.keyboard("reformas{Enter}");

      await userEvent.click(box());

      expect(rows()).toHaveLength(4);
    });
  });

  describe("keyboard", () => {
    it("opens with the arrow from the box and walks the list without choosing", async () => {
      const { onChoose } = renderSelect();
      box().focus();

      await userEvent.keyboard("{ArrowDown}");
      await userEvent.keyboard("{ArrowDown}");

      expect(box()).toHaveAttribute("aria-activedescendant", row(1).id);
      expect(onChoose).not.toHaveBeenCalled();
    });

    it("chooses the row under the cursor with Enter", async () => {
      const { onChoose } = renderSelect();
      box().focus();

      await userEvent.keyboard("{ArrowDown}{ArrowDown}{Enter}");

      expect(onChoose).toHaveBeenCalledWith(representative);
      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("stops on an unusable row so its reason is read, but Enter does not choose it", async () => {
      const { onChoose } = renderSelect();
      box().focus();

      await userEvent.keyboard("{ArrowDown}{ArrowDown}{ArrowDown}{Enter}");

      expect(box()).toHaveAttribute("aria-activedescendant", row(2).id);
      expect(onChoose).not.toHaveBeenCalled();
    });

    it("puts the cursor on what is already chosen when it opens", async () => {
      renderSelect({ chosen: representative });

      await userEvent.click(box());

      expect(box()).toHaveAttribute("aria-activedescendant", row(1).id);
    });

    it("closes on Escape without choosing, empties the search and gives the focus back", async () => {
      const { onChoose } = renderSelect();
      await userEvent.click(box());
      await userEvent.keyboard("reformas");

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
      expect(onChoose).not.toHaveBeenCalled();
      expect(box()).toHaveFocus();
      await userEvent.click(box());
      expect(rows()).toHaveLength(4);
    });
  });
});
