# Cabecera

La parte de arriba de la ventana: los documentos, el aviso y el único menú. En
Windows y macOS es una sola barra; en Linux, la barra de título nativa de GTK
con una tira de pestañas debajo.

**La verdad del dibujo es el código y sus historias.** Esta ficha cuenta el
porqué y el flujo entre estados; para ver la barra, las historias de
`rfirma-app/src/shell/Header.stories.tsx` («Flujos/Ventana principal/Header») y
las de la ventana entera, en `MainWindow.stories.tsx`. Los textos salen del
catálogo i18n (`po/messages.pot`) y no se copian aquí.

## Casos de uso que la usan

- Firmar un PDF en local — en todos los estados.
- El panel de estado — es la puerta: el [panel de estado](panel-de-estado.md) se
  abre desde su menú, y el botón de aviso lleva a él cuando hay algo que mirar
  ahí.
- Preferencias y el primer arranque — con la variante sin documentos.

## Flujo entre estados

La cabecera no navega: es el mismo componente en cuatro estados que dependen de
la plataforma y de lo que haya que decir.

| Estado | Historia | Qué cambia |
| --- | --- | --- |
| Sin documentos | `WithoutDocuments` | Solo el aviso, si lo hay, y el menú; ni botón de abrir ni pestañas. Es la de Preferencias, el panel de estado y el primer arranque |
| Con documentos | `WithDocuments` | El hueco de las pestañas, con el botón partido de abrir, y el menú al extremo |
| Con aviso | `WithAttention` | El botón de aviso a la izquierda del menú |
| Barra de título nativa | `NativeTitlebarWithDocuments` | Linux: la cabecera de 44 px no existe y solo queda la tira de pestañas |

El menú **arranca cerrado** y lo abre el botón. En Linux no lo pinta React:
es el menú GTK de la barra de título, con F10, y lo cuenta el backend.

## Estructura

**Windows y macOS.** Una fila, de izquierda a derecha: el botón partido de
abrir, las pestañas con su «+N» si no caben, un hueco flexible, el botón de
aviso si hay algo que atender y el botón de menú. El hueco no baja de un mínimo
para que la última pestaña no toque el menú.

**Linux** (en todo Linux, sin detectar el escritorio). La barra de título es la
nativa de GTK y lleva el botón partido de abrir, el título, el aviso, el menú y
los botones de ventana del tema. Debajo, dentro de la ventana, una tira con solo
las pestañas, del mismo gris que la barra. **Sin ninguna pestaña la tira no
existe**: ni en el inicio sin documentos, ni en Preferencias, ni en el panel de
estado, ni en el primer arranque. La barra sigue la preferencia de tema de
rFirma —claro, oscuro o sistema—, no solo la del escritorio.

Lo que va de los documentos es de [Pestañas de documentos](pestanas-de-documentos.md);
esta ficha cuenta la barra y el menú. **Nada más**: ni certificado ni estado del
documento. El certificado lo dice el selector del
[panel de firma](panel-de-firma.md) y el estado, la marca de la pestaña.

## El menú

Cuatro entradas en dos grupos, con claves `status.title`, `header.preferences`,
`header.help` y `header.about`: arriba lo de **esta instalación** (el estado) y,
tras un divisor, el grupo de siempre de la GNOME HIG, en su orden. La entrada de
ayuda lleva el icono de enlace externo salvo en Linux. No hay barra de menús
([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).

Toda entrada reserva a su derecha la columna del icono, lleve icono o no, para
que el texto de las cuatro quede alineado. La entrada enfocada lleva el anillo
`--rf-focus-ring` y fondo `--rf-surface`: forma y color, no color solo.

**Lo que no está en el menú:** abrir un documento (el botón partido), guardar (el
pie del panel), paginación y zoom (la píldora del
[visor](visor-de-documento.md)), atajos y guía rápida (no existen).

### El aviso

Cuando hay algo que **rFirma puede y debe arreglar**, aparece un botón de aviso
propio a la izquierda del menú (nombre accesible `header.attention`). Lo
encienden dos cosas y solo dos:

- **El certificado de rFirma ausente o a medias** en los navegadores.
- **Sin configurar** en la firma en sedes.

No lo encienden: que las sedes abran AutoFirma (es una elección), «No aplica»,
no tener certificados de firma (no lo arregla rFirma) ni una versión nueva (eso
lo dice la franja de la [ventana principal](ventana-principal.md)).

La fila del panel **informa**; el botón **llama**. Por eso hay «Atención» que no
lo encienden.

- **Solo existe cuando hay problema.** Con todo en orden no hay botón ni hueco.
- **Lleva al panel de estado**, y dentro del panel se oculta: ya estás donde te
  llevaría.
- Icono de advertencia en **el color del texto**, nunca en un color de alerta, y
  sin número ni contador: la marca es la silueta.
- Ni el menú ni la entrada del estado llevan marca.

Es igual en los tres escritorios; solo cambia el icono (el del tema en Linux).

## Componentes y tokens

`Button`, `Popover` y los iconos del sistema de diseño (`MenuIcon`, `AlertIcon`,
`ExternalLinkIcon`); `.rf-divider`; `--rf-bg`, `--rf-surface`,
`--rf-border-subtle`, `--rf-text`, `--rf-text-muted`, `--rf-shadow-elevated`,
`--rf-focus-ring`, `--rf-radius-md`. La geometría la dan `Header.css` y las
historias; el gris de la barra de GTK lo pone el tema del escritorio, no los
tokens.

La barra va por encima del contenido, y con cualquiera de sus menús abierto sube
un nivel más (capas en la [ventana principal](ventana-principal.md)).

## Decisiones

- **Sin barra de menús clásica.** GNOME la abandonó y Windows 11 no la usa; en
  macOS el menú es el mismo de la cabecera que en Windows
  ([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).
- **Una sola barra, no cabecera y tira** (27/09/2026). Se descartó la cabecera
  alta sobre `--rf-surface` con la tira de pestañas debajo: eran dos franjas para
  la parte de arriba de la ventana, y lo que sobra vuelve al visor. El fondo pasa
  a `--rf-bg`, el del visor, porque ya no hay una tira que se funda con la
  pestaña activa: la activa se marca con un subrayado.
- **Sin insignia de estado ni certificado** (25/09/2026). La insignia pasa a la
  marca de cada pestaña, que dice lo mismo por documento; el certificado, al
  selector del [panel de firma](panel-de-firma.md).
- **El divisor del menú.** Sin él las cuatro entradas se leen del mismo rango y
  el estado deja de ser lo primero.
- **El estado se llama «de rFirma», no a secas.** El nombre lo desambigua del
  estado del documento.
- **El aviso es una silueta, no un contador ni un punto de color.** Un número
  obligaría a contar en dos sitios; un punto de color sería el único indicador,
  que la sección 8 del [sistema de diseño](design-system.md) prohíbe.
- **En Linux, la barra de título nativa de GTK** (30/09/2026). Se descartó la
  barra HTML sobre una ventana sin decorar haciendo de barra de título: los
  botones de ventana dibujados en HTML nunca serían los del tema. La medición y
  el prototipo están en `docs/research/barra-de-titulo-en-linux.md`.
- **Todo Linux, no solo GNOME.** Se descartó montar la barra GTK solo en GNOME
  detectando el escritorio: en KDE la headerbar de GTK funciona como en
  cualquier aplicación GTK, y detectarlo añadía un tercer modo de cabecera a
  cambio de nada.
- **El menú GTK, sin icono de enlace externo.** Es el menú estándar, sin iconos;
  en Windows la entrada de ayuda lo conserva.
- **El menú de Linux, estándar pero con el aire de libadwaita** (01/10/2026).
  Mismo relleno, separación y esquinas que el popover de los recientes, para que
  los dos desplegables de la barra se lean iguales; se descartó dejarlo como lo
  dibuja GTK 3, sin relleno ni separación entre filas.
- **El aviso, un botón y no una marca** (30/09/2026). Se descartó el triángulo en
  el menú y en la entrada del estado: el menú GTK estándar va sin iconos, y un
  botón propio lleva al panel de un clic y se comporta igual en los tres
  escritorios.
