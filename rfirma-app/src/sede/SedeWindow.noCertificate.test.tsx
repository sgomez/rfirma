import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RenderErrorBoundary } from "../errors/RenderErrorBoundary";
import type { Certificate } from "../signing/certificate";
import { elapse } from "../testing/elapse";
import { renderWithCatalog } from "../testing/render";
import { OUTCOME_CLOSE_MS } from "./errand";
import * as noCertificateModule from "./SedeNoCertificate.stories";
import * as outcomeModule from "./SedeOutcome.stories";
import { SedeWindow } from "./SedeWindow";
import { scriptedErrand, scriptedFrom } from "./sedeWindowFixtures";

/** Grada A: el momento 5 (sin certificado utilizable), el fallo de un hijo y la forma de la ventana. */

const { NoneInstalled, ExcludedBySite } = composeStories(noCertificateModule);
const { Cancelled } = composeStories(outcomeModule);

describe("5 · no usable certificate", () => {
  it("leaves through the footer, and the window paints no cross of its own", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(NoneInstalled);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(calls.cancel).toHaveBeenCalledOnce();
    expect(calls.close).not.toHaveBeenCalled();
  });

  it.each([
    [1, "No se acepta tu certificado"],
    [3, "No se acepta ninguno de tus 3 certificados"],
  ])(
    "titles the exclusion of %i without naming a site when the request brings no origin",
    (owned, title) => {
      const { port } = scriptedErrand(
        { kind: "noCertificate", reason: "excluded", owned },
        { origin: null },
      );
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(title)).toBeInTheDocument();
      expect(screen.queryByText(/La petición/)).not.toBeInTheDocument();
    },
  );

  it.each([
    ["none installed", NoneInstalled],
    ["excluded by the site", ExcludedBySite],
  ])("focuses the main action when %s: installing another can still fix it", (_, story) => {
    const { port } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("button", { name: "Instalar un certificado…" })).toHaveFocus();
  });

  it("looks again through the port, for a certificate installed with the window open", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(NoneInstalled);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(calls.lookAgain).toHaveBeenCalledOnce();
  });

  it("installs from the excluded screen too, because a new certificate might not be excluded", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(ExcludedBySite);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Instalar un certificado…" }));

    expect(calls.installCertificate).toHaveBeenCalledOnce();
  });
});

describe("5 · install failure", () => {
  it("shows the failure in line instead of discarding it", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(NoneInstalled);
    calls.installCertificate.mockRejectedValueOnce({
      situation: "pkcs12Unreadable",
      detail: "SEC_PKCS12DecoderUpdate",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Instalar un certificado…" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Ese fichero no sirve como certificado");
    expect(calls.lookAgain).not.toHaveBeenCalled();
  });

  it("shows no error when the file dialog is cancelled", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(NoneInstalled);
    calls.installCertificate.mockResolvedValueOnce(false);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Instalar un certificado…" }));

    expect(await screen.findByRole("button", { name: "Instalar un certificado…" })).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(calls.lookAgain).not.toHaveBeenCalled();
  });
});

describe("when a child throws", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows the failure screen instead of closing itself", async () => {
    const { port, calls } = scriptedErrand({
      kind: "consent",
      document: null,
      signs: null,
      signing: "pdf",
      items: null,
      certificates: undefined as unknown as Certificate[],
      narrowed: false,
    });
    renderWithCatalog(
      <RenderErrorBoundary>
        <SedeWindow errands={port} />
      </RenderErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await elapse(OUTCOME_CLOSE_MS * 2);

    expect(calls.close).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});

describe("the window's shape", () => {
  it("has no application header, no menu, no tray and no destination footer", () => {
    const { port } = scriptedFrom(Cancelled);
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.queryByRole("banner")).not.toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.queryByText(/se guardará en/i)).not.toBeInTheDocument();
  });
});
