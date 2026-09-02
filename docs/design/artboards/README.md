# Artboards importados de Claude Design

**Esto es una importación de un solo uso, y se borra al terminar el [#80].**

Son los trece artboards del canvas de Claude Design «Autofirma de escritorio en
Rust», bajados literalmente, más el `canvas.json` que los ordena y los titula.
Tres de ellos —`Main`, `EstadoExito` y `PreferenciasPantalla`— se rehicieron el
02/09/2026 con las decisiones de v0.2 del
[#123](https://github.com/sgomez/rfirma/issues/123); ver «Lo que cambió en v0.2»
al final.
Están aquí para que la transcripción a JSX se pueda hacer y revisar **sin
cuenta de Claude**, y porque el repositorio es público y su interfaz no puede
estar especificada detrás de un servicio con acceso restringido.

Cuando el #80 termine, este directorio desaparece y la referencia duradera de
la interfaz pasa a ser las fichas de `docs/design/`, que es lo que ya dice
`docs/agents/prototyping.md`. Si estás leyendo esto y el #80 está cerrado,
alguien se ha dejado el borrado.

## Qué es cada fichero

`canvas.json` numera los estados y los reparte en dos páginas. El orden es el
del recorrido de la ficha `ventana-principal.md`:

| # | Artboard | Estado |
| - | -------- | ------ |
| 1 | `EstadoVacio` | Vacío, con el menú de la cabecera **dibujado abierto** |
| 2 | `EstadoDocumentoCargado` | Documento cargado, sin certificado |
| 2b | `EstadoElegirCertificado` | Eligiendo entre varios certificados |
| 3 | `EstadoCargandoCertificados` | Buscando certificados |
| 4 | `EstadoSinCertificados` | Sin certificados, con salida |
| 5 | `Main` | Configurando la firma visible — el nudo del recorrido, con el pie del destino |
| 6 | `EstadoPin` | Pidiendo PIN |
| 7 | `EstadoPinIncorrecto` | PIN incorrecto |
| 8 | `EstadoFirmando` | Firmando, con las tres fases |
| 9 | `EstadoExito` | Firmado — el resumen, sin la ficha 14 |
| 10 | `EstadoErrorFirma` | Error de firma, en el pie del panel |
| — | `PreferenciasPantalla` | Preferencias, a pantalla completa |
| — | `EstadoAcercaDe` | Diálogo de «acerca de» |

No se ha importado `firmar-fichero-local.dc.html`: `canvas.json` lo aparta en
la página «Otros» y lo marca como ajeno al recorrido.

## Cómo leerlos

Un `.dc.html` es un **artboard**, no un componente. Lleva andamiaje que no va a
la aplicación y que el ID-43 del #80 manda quitar: la etiqueta `<x-dc>`, el
`<helmet>`, la prop `{{tema}}`, el bloque `data-dc-script` y `support.js` (que
no se ha importado: sin él los ficheros no se renderizan solos, y no hacen
falta para transcribir).

**El `<helmet>` no es la fuente del sistema de diseño.** Es una copia
comprimida y le faltan tokens; manda el bundle versionado (ID-47). Los trece
ficheros lo llevan byte a byte idéntico al de **`_helmet.part`**, que es de
donde se copia al redactar uno nuevo —nunca de un `get_file` del proyecto de
Claude Design, cuya copia se queda atrás—, y `comprueba.sh` lo verifica contra
ese fichero. Compararlos solo entre sí no valía: trece ficheros de acuerdo
entre ellos dan verde con el sistema de diseño equivocado entero.

## `EstadoElegirCertificado` no viene del canvas original

Los otros doce se bajaron del canvas tal cual. Este se **añadió después**, el
01/09/2026, porque el recorrido no tenía pantalla para elegir entre varios
certificados: con más de uno el panel enseñaba «Elegir certificado» y ese botón
se limitaba a volver a buscar, así que no había forma de elegir ninguno.

Es el único que **se puede pulsar**: abre el desplegable, se desplaza y se
elige, y lleva tres palancas —estado inicial, cuántos certificados hay y si se
listan los que no sirven— para poder decidir mirando en vez de suponiendo. Lo
que se decidió con él:

- **Desplegable superpuesto**, y no un acordeón en flujo ni un diálogo: la lista
  flota sobre el panel, así que la firma visible y el botón de firmar no se
  mueven al abrirla.
- **Un certificado caducado o revocado se lista, dice por qué y no se deja
  elegir** (`disabled`). Que falte de la lista no le explica nada a quien viene
  a firmar justo con él.
- **La fila lleva el almacén** —`DNI · emisor · almacén`—, porque el mismo
  certificado en el perfil de Firefox y en `~/.pki/nssdb` es indistinguible sin
  él. El disparador cerrado no lo lleva: elegido ya no desambigua nada.
- **Sin preselección la primera vez**: elegir con qué identidad se firma no lo
  hace la aplicación, y el orden de la lista solo dice en qué orden cargaron los
  módulos.
- **El certificado se recuerda al firmar con él**, no al elegirlo en la lista.

Sustituye al botón `Cambiar` de la tarjeta de certificado que declaraba
`panel-de-firma.md`: el disparador del desplegable ya es el sitio donde se
cambia.

## Lo que hay que decidir al transcribir

Una cosa que el canvas da por buena y el código no puede sostener tal cual.
No la resuelvas por tu cuenta: es un cambio de ficha (ID-44).

1. **El panel enseña datos que hoy nadie calcula**: «27 páginas · 2,4 MB», «Ya
   lleva 1 firma: la tuya será una cofirma» y, con la palanca «Ficha 14»
   levantada, «2 firmas» y la lista de firmas. El código pasa el tamaño y las
   firmas como desconocidos, y detectar si un PDF ya viene firmado está fuera
   del alcance del #81. El tamaño **sí** se recupera en el resumen: lo conoce
   `finish_signing` y hoy `SignedDocumentView` lo descarta.

Los otros dos puntos que había aquí —«Junto al documento original» como destino,
y la ausencia de «Recordar mi actividad» y «Vaciar la lista»— los resolvió
`PreferenciasPantalla`, y ya no hay nada que decidir.

## Lo que cambió en v0.2

Decidido en la conversación del
[#123](https://github.com/sgomez/rfirma/issues/123) y dibujado el 02/09/2026.

**`PreferenciasPantalla` sustituye a `EstadoPreferencias`**, que se ha borrado.
El diálogo de 480 px pasa a ocupar toda la ventana bajo la cabecera, que se
queda intacta con su estado de documento: sigue siendo un diálogo, no una
navegación. Índice de secciones a la izquierda —Firma, Privacidad,
Apariencia—, columna de contenido en medio y **pie fijo con `Cerrar`**, que así
no se pierde al desplazarse. Los ajustes se siguen guardando al cambiarlos: no
hay «Guardar» ni «Cancelar». El destino deja de ser un desplegable de una sola
opción y pasa a un selector de directorio con `Cambiar carpeta…`.

Tres palancas: **Ficha 5b** saca cada uno de los dos avisos de error que hoy se
tragan —el de guardar un ajuste y el de vaciar la lista—, **Apagar Recordar mi
actividad** saca la confirmación destructiva, que sigue siendo un diálogo
pequeño encima porque el borrado es irreversible, y **Maquetación** decide el
ancho de la columna.

**`Main` estrena el pie del destino**: carpeta *más* nombre de fichero, con la
última carpeta precedida de `…/` y el nombre recortado por el medio. La carpeta
entera, la extensión y la cola —`-firmado` y su número de desempate— no se
recortan nunca; la línea envuelve a dos renglones antes que cortarse. La palanca
**Pie · destino** recorre los cinco casos, incluido el destino no disponible en
el que el botón de firmar **no** se apaga; **Pie · recorte** lleva dos
deslizadores, uno para el nombre y otro para la carpeta.

**`EstadoExito` estrena el resumen sin la ficha 14**: nombre, tamaño,
encabezado `Resumen` con la insignia `PAdES` sola guardando el sitio, y tres
botones verticales —`Abrir el PDF` primario, `Abrir la carpeta` secundario,
`Volver a firmar` fantasma—. `Firmar otro documento` se retira: la bandeja ya
abre y acepta arrastre. La palanca **Ficha 14** enseña qué ocupará ese hueco en
v1.0.

**Los otros ocho artboards que llevan pie se barrieron a la vez**: ninguno
enseña ya `~/Documentos/…`, que es la ruta que el ADR-0011 prohíbe. Los suyos
son estáticos —solo `Main` lleva las palancas—, pero siguen la misma regla:
carpeta atenuada con `…/` delante, nombre en color de texto, y la línea
envolviendo en vez de cortarse.

[#80]: https://github.com/sgomez/rfirma/issues/80
