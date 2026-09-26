import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const { tauriDestinations, tauriLanguagePreference, tauriPreferences, tauriVersionCheck } =
  await import("./tauriPreferences");

const aConfiguration = {
  language: "es",
  destination: "Documentos",
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  theme: "system",
  offersTheOriginalFolder: false,
  setupWizardSeen: false,
  consentCountdown: true,
  honourAutomaticSelection: false,
};

/**
 * **Grada A**: los ajustes y el idioma son el mismo fichero debajo, y lo que
 * se comprueba aquí es justo eso —que ninguno de los dos puertos pisa lo que
 * el otro acaba de guardar—.
 */
describe("los puertos de la configuración sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("reads the settings the backend remembers, destination included", async () => {
    invoke.mockResolvedValue(aConfiguration);

    const read = await tauriPreferences().read();

    expect(invoke).toHaveBeenCalledWith("read_configuration");
    expect(read).toEqual({
      theme: "system",
      destination: "Documentos",
      offersOriginalFolder: false,
      rememberVisibleSignature: true,
      rememberActivity: true,
      notifyNewVersion: true,
      setupWizardSeen: false,
      consentCountdown: true,
      honourAutomaticSelection: false,
    });
  });

  it("carries the automatic selection requested by sites both ways", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, honourAutomaticSelection: true })
        : Promise.resolve(undefined),
    );

    const read = await tauriPreferences().read();
    await tauriPreferences().save({ ...read, honourAutomaticSelection: false });

    expect(read.honourAutomaticSelection).toBe(true);
    expect(invoke).toHaveBeenLastCalledWith("write_configuration", {
      configuration: { ...aConfiguration, honourAutomaticSelection: false },
    });
  });

  it("reads whether the environment allows Junto al documento original", async () => {
    invoke.mockResolvedValue({ ...aConfiguration, offersTheOriginalFolder: true });

    expect((await tauriPreferences().read()).offersOriginalFolder).toBe(true);
  });

  it("falls back to the system theme when what is stored is not one of the three", async () => {
    invoke.mockResolvedValue({ ...aConfiguration, theme: "sepia" });

    expect((await tauriPreferences().read()).theme).toBe("system");
  });

  /**
   * El idioma no es de `Preferences` sino de su propio puerto, así que
   * guardar los ajustes con una copia local en vez de releer devolvería el
   * idioma anterior y desharía el cambio.
   */
  it("keeps the language the other port saved when the settings are written", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, language: "en" })
        : Promise.resolve(undefined),
    );

    await tauriPreferences().save({
      theme: "dark",
      destination: "Documentos",
      offersOriginalFolder: false,
      rememberVisibleSignature: false,
      rememberActivity: true,
      notifyNewVersion: true,
      setupWizardSeen: false,
      consentCountdown: false,
      honourAutomaticSelection: false,
    });

    expect(invoke).toHaveBeenLastCalledWith("write_configuration", {
      configuration: {
        ...aConfiguration,
        language: "en",
        theme: "dark",
        rememberVisibleSignature: false,
        consentCountdown: false,
        honourAutomaticSelection: false,
      },
    });
  });

  /**
   * `save` proyecta las claves del contrato en vez de esparcir `preferences`
   * entero: `offersOriginalFolder` la contesta el backend y no cruza al
   * escribir (ID-184), así que mandarla sería una clave que serde tira en
   * silencio y que además nombra distinto al campo real (`offersTheOriginal
   * Folder`).
   */
  it("never sends offersOriginalFolder back when the settings are written", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, offersTheOriginalFolder: true })
        : Promise.resolve(undefined),
    );

    await tauriPreferences().save({
      theme: "dark",
      destination: "Documentos",
      offersOriginalFolder: true,
      rememberVisibleSignature: true,
      rememberActivity: true,
      notifyNewVersion: true,
      setupWizardSeen: true,
      consentCountdown: true,
      honourAutomaticSelection: false,
    });

    const [, { configuration }] = invoke.mock.calls.at(-1) as [string, { configuration: object }];
    expect(configuration).not.toHaveProperty("offersOriginalFolder");
    expect(configuration).toHaveProperty("offersTheOriginalFolder", true);
  });

  it("saves the language without touching the rest of the settings", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, theme: "dark" })
        : Promise.resolve(undefined),
    );

    await tauriLanguagePreference().save("en");

    expect(invoke).toHaveBeenLastCalledWith("write_configuration", {
      configuration: { ...aConfiguration, theme: "dark", language: "en" },
    });
  });

  it("falls back to Spanish when what is stored is not one of the six", async () => {
    invoke.mockResolvedValue({ ...aConfiguration, language: "fr" });

    expect(await tauriLanguagePreference().read()).toBe("es");
  });

  it("wires forgetting the activity to its own command", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriPreferences().forgetActivity();

    expect(invoke).toHaveBeenCalledWith("forget_activity");
  });

  it("picks the destination folder with the backend dialog and gets back a name", async () => {
    invoke.mockResolvedValue("Firmados");

    await expect(tauriPreferences().chooseFolder()).resolves.toBe("Firmados");
    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["choose_destination"]);
  });

  it("reads a cancelled directory picker as no choice, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriPreferences().chooseFolder()).resolves.toBeNull();
  });
});

/**
 * **Grada A**: el destino sobre Tauri. Quien lo compone —la carpeta comprobada
 * y el nombre con su homónimo resuelto— es `app::documents::where_it_lands`, y
 * está probado allí; aquí solo se comprueba la costura.
 */
describe("el puerto del destino sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks where the open document will land, by its identifier and never by a path", async () => {
    invoke.mockResolvedValue({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
      writable: true,
    });

    const destination = await tauriDestinations().previewFor("1e8b83b9");

    expect(invoke).toHaveBeenCalledWith("preview_destination", {
      id: "1e8b83b9",
      destination: null,
    });
    expect(destination).toEqual({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
      writable: true,
    });
  });

  it("asks with the single destination's id when this signature has one", async () => {
    invoke.mockResolvedValue({
      folder: "Escritorio",
      name: "contrato-firmado.pdf",
      writable: true,
    });

    await tauriDestinations().previewFor("1e8b83b9", "single-42");

    expect(invoke).toHaveBeenCalledWith("preview_destination", {
      id: "1e8b83b9",
      destination: "single-42",
    });
  });

  it("opens the save dialog for a single signature, by the document's identifier", async () => {
    invoke.mockResolvedValue({
      id: "single-42",
      folder: "Escritorio",
      name: "contrato-firmado.pdf",
      writable: true,
    });

    const chosen = await tauriDestinations().chooseSingle("1e8b83b9");

    expect(invoke).toHaveBeenCalledWith("choose_single_destination", { id: "1e8b83b9" });
    expect(chosen).toEqual({
      id: "single-42",
      folder: "Escritorio",
      name: "contrato-firmado.pdf",
      writable: true,
    });
  });

  it("reads a cancelled save dialog as no choice", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriDestinations().chooseSingle("1e8b83b9")).resolves.toBeNull();
  });
});

describe("el puerto de la versión sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks the backend whether there is a newer published version, and nothing else", async () => {
    invoke.mockResolvedValue({ version: "0.4.1" });

    expect(await tauriVersionCheck().latest()).toEqual({ version: "0.4.1" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("check_for_new_version");
  });

  // Sin versión nueva, sin red o dentro de las 24 h de caché la orden contesta
  // lo mismo: nada. La ventana no distingue los tres casos porque no tiene que
  // hacer nada distinto en ninguno.
  it("reads no answer as nothing to say, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    expect(await tauriVersionCheck().latest()).toBeNull();
  });
});
