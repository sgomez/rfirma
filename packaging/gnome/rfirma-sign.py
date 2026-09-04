"""«Firmar con rFirma» en el menú contextual de Nautilus, sobre un PDF.

Es la mitad GNOME del ID-156: lo que en KDE hace `packaging/kde/rfirma-sign.desktop`
con un `Type=Service`, aquí lo hace una extensión de `nautilus-python`. Y como allí,
rFirma no declara `application/pdf` en ningún lanzador (ADR-0018): el tipo sólo filtra
en qué menú aparece el verbo, no registra a nadie como candidato a abrir nada.

Este fichero se ejecuta **dentro del proceso de Nautilus**, así que lo que falle aquí
rompe el gestor de ficheros de quien lo tenga instalado, no rFirma. De ahí las dos
reglas que sigue: nada de trabajo en el hilo del menú más allá de mirar el fichero
seleccionado, y ningún fallo que salga de este módulo.

Lo instalan el `.deb` y el `.rpm` en `/usr/share/nautilus-python/extensions/`, y el
paquete que trae `nautilus-python` viaja como recomendación (se llama distinto en cada
familia). Sin ese paquete el fichero queda inerte: nadie lo carga y no pasa nada.
"""

import gi

# Nautilus 4.0 es la API de Nautilus 43 en adelante; 3.0 la de las versiones sobre GTK3
# que siguen vivas en las distribuciones de soporte largo. `require_version` con una que
# no está instalada lanza `ValueError`, así que se prueban en orden y gana la primera.
for _api_version in ("4.0", "3.0"):
    try:
        gi.require_version("Nautilus", _api_version)
        break
    except ValueError:
        continue

from gi.repository import Gio, GLib, GObject, Nautilus

PDF_MIME_TYPE = "application/pdf"
MENU_LABEL = "Firmar con rFirma"
EXECUTABLE = "rfirma"


def _is_a_signable_pdf(document):
    """Un PDF de verdad, en el sistema de ficheros local y que no sea un directorio."""
    try:
        return (
            document.get_uri_scheme() == "file"
            and not document.is_directory()
            and document.get_mime_type() == PDF_MIME_TYPE
        )
    except Exception:  # noqa: BLE001 — corre dentro de Nautilus: nada sale de este módulo
        return False


def _path_of(document):
    """La ruta local del documento, o `None` si no la tiene."""
    try:
        location = document.get_location()
        return location.get_path() if location is not None else None
    except Exception:  # noqa: BLE001 — corre dentro de Nautilus: nada sale de este módulo
        return None


class SignWithRfirma(GObject.GObject, Nautilus.MenuProvider):
    """Pone el verbo al primer nivel cuando hay exactamente un PDF seleccionado."""

    def get_file_items(self, *args):
        # Nautilus 3.0 pasa `(window, files)` y 4.0 sólo `(files)`: la selección es
        # siempre el último argumento.
        files = args[-1] if args else []
        if len(files) != 1 or not _is_a_signable_pdf(files[0]):
            return []
        item = Nautilus.MenuItem(name="SignWithRfirma::sign", label=MENU_LABEL)
        item.connect("activate", self._on_activate, files[0])
        return [item]

    def _on_activate(self, _menu_item, document):
        path = _path_of(document)
        if path is None:
            return
        try:
            # `Gio.Subprocess` recoge al hijo por su cuenta: lanzar rFirma no deja un
            # zombi colgando del proceso de Nautilus.
            Gio.Subprocess.new([EXECUTABLE, path], Gio.SubprocessFlags.NONE)
        except GLib.Error:
            # rFirma desinstalada o sin permiso de ejecución. No hay dónde enseñar el
            # error desde aquí, y reventar se llevaría por delante el gestor de ficheros.
            pass
