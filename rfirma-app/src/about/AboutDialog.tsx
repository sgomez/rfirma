import type { TFunction } from "i18next";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";
import { ArrowUpIcon, CheckIcon, CopyIcon } from "../design-system/icons";
import type { NewVersion } from "../updates/newVersion";

/**
 * Las órdenes de alta del repositorio, una por canal (ID-181).
 *
 * **No se traducen**: son texto de terminal, igual que `about.repository`.
 * Las rutas son las que sirve `rfirma.sgomez.me`, documentadas en el
 * [ADR-0015](../../../docs/adr/0015-canal-de-distribucion-propio.md) y
 * publicadas en `packaging/repo/index.html`, que es de donde salen literales:
 * si esas órdenes cambian, cambian aquí también.
 */
type UpdateChannel = "flatpak" | "deb" | "rpm";

/** Las tres pestañas del selector de canal, en el orden del artboard. */
const UPDATE_CHANNELS: readonly UpdateChannel[] = ["flatpak", "deb", "rpm"];

const UPDATE_COMMANDS: Record<UpdateChannel, string> = {
  flatpak: "flatpak install https://rfirma.sgomez.me/rfirma.flatpakref",
  deb: `curl -fsSL https://rfirma.sgomez.me/rfirma.asc | sudo tee /usr/share/keyrings/rfirma.asc >/dev/null
sudo tee /etc/apt/sources.list.d/rfirma.sources <<'EOF'
Types: deb
URIs: https://rfirma.sgomez.me/apt/
Suites: stable
Components: main
Signed-By: /usr/share/keyrings/rfirma.asc
EOF
sudo apt update && sudo apt install rfirma`,
  rpm: `sudo tee /etc/yum.repos.d/rfirma.repo <<'EOF'
[rfirma]
name=rfirma
baseurl=https://rfirma.sgomez.me/rpm/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=https://rfirma.sgomez.me/rfirma.asc
EOF
sudo dnf install rfirma`,
};

interface AboutDialogProps {
  /** La versión que se enseña. Sale de `package.json` en tiempo de compilación. */
  version: string;
  /**
   * Lo que contestó la comprobación de versión, o `null` si no hay una más
   * nueva —o no se ha podido preguntar—. Los dos estados miden lo mismo: solo
   * cambia la línea de arriba del bloque de «cómo actualizar» (ID-181).
   */
  newVersion: NewVersion | null;
  onClose: () => void;
}

/** La etiqueta de cada pestaña, ya traducida: `t()` no admite una clave armada. */
function channelLabel(t: TFunction, channel: UpdateChannel): string {
  switch (channel) {
    case "flatpak":
      return t("about.update.channel.flatpak");
    case "deb":
      return t("about.update.channel.deb");
    case "rpm":
      return t("about.update.channel.rpm");
  }
}

/**
 * Identidad de la aplicación, cómo actualizar, aviso de independencia y
 * licencias.
 *
 * El requisito de esta pantalla es de **contenido**, no de estética: dice que
 * esto **no es el cliente oficial**. Una aplicación que firma ante la
 * Administración con la misma criptografía que la oficial se puede confundir
 * con ella, y esa confusión hay que deshacerla en el sitio donde la gente va a
 * preguntar qué es esto.
 *
 * **Cómo actualizar** (ID-181) no es un botón de descarga: son **las órdenes
 * de alta del repositorio**, copiables, para los tres canales de la v0.4. No
 * hay enlace porque `opener:deny-open-url` sigue denegado (ID-85), así que la
 * URL solo aparece **dentro** de la orden. El bloque mide lo mismo en los dos
 * estados de versión —solo cambia la línea de arriba— y por eso sigue
 * enseñando cómo darse de alta a quien entra sin tener versión nueva, que es
 * justo a quien el mecanismo quiere reducir.
 *
 * El aviso de independencia va **como párrafo, sin icono ni recuadro**: es un
 * hecho sobre el proyecto, no una advertencia sobre un riesgo del usuario, y
 * enmarcarlo como alarma le daría un peso que no le corresponde.
 *
 * El artboard dibuja las dos líneas de licencia **desplegadas**; eso es el
 * estado congelado del canvas y aquí es lo que revela «Ver las licencias»
 * (ID-43). Lo que se ve siempre es la dirección del repositorio, que es adónde
 * va quien quiera comprobar cualquiera de las dos.
 */
export function AboutDialog({ version, newVersion, onClose }: AboutDialogProps) {
  const { t } = useTranslation();
  const [showingLicenses, setShowingLicenses] = useState(false);
  const [channel, setChannel] = useState<UpdateChannel>("flatpak");
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div className="rf-dialog about" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="about__identity">
          <p className="rf-heading about__name" id={titleId}>
            {t("app.name")}
          </p>
          <p className="rf-body rf-text-muted">{t("about.version", { version })}</p>
        </div>

        <p className="rf-prose">{t("about.whatItDoes")}</p>

        <hr className="rf-divider" />

        <div className="about__update">
          <div className="rf-row rf-gap-xs about__updateStatus">
            {newVersion !== null ? (
              <ArrowUpIcon size={18} />
            ) : (
              <span className="about__upToDateIcon">
                <CheckIcon size={18} strokeWidth={1.5} />
              </span>
            )}
            <p className="rf-prose">
              {newVersion !== null
                ? t("about.update.newVersion", { version: newVersion.version })
                : t("about.update.upToDate")}
            </p>
          </div>

          <div className="about__channels" role="tablist">
            {UPDATE_CHANNELS.map((each) => (
              <button
                key={each}
                type="button"
                role="tab"
                aria-selected={each === channel}
                className={
                  each === channel ? "about__channel about__channel--active" : "about__channel"
                }
                onClick={() => setChannel(each)}
              >
                {channelLabel(t, each)}
              </button>
            ))}
          </div>

          <div className="rf-row rf-gap-xs about__commands">
            <pre className="about__commandsText">{UPDATE_COMMANDS[channel]}</pre>
            <button
              type="button"
              className="rf-btn rf-btn--ghost about__copy"
              onClick={() => void navigator.clipboard.writeText(UPDATE_COMMANDS[channel])}
            >
              <CopyIcon size={14} />
              {t("actions.copy")}
            </button>
          </div>
        </div>

        <p className="rf-prose">{t("about.independence")}</p>

        <hr className="rf-divider" />

        <div className="about__licenses">
          {showingLicenses && (
            <>
              <p className="rf-body rf-text-muted">{t("about.licenses.afirma")}</p>
              <p className="rf-body rf-text-muted">{t("about.licenses.rfirma")}</p>
            </>
          )}
          <p className="rf-body">{t("about.repository")}</p>
        </div>

        <hr className="rf-divider" />

        <div className="rf-row about__footer">
          <button
            type="button"
            className="rf-btn rf-btn--ghost"
            aria-expanded={showingLicenses}
            onClick={() => setShowingLicenses((shown) => !shown)}
          >
            {t("about.licenses.view")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
