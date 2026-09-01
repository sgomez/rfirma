# Artboards importados de Claude Design

**Esto es una importación de un solo uso, y se borra al terminar el [#80].**

Son los trece artboards del canvas de Claude Design «Autofirma de escritorio en
Rust», bajados literalmente, más el `canvas.json` que los ordena y los titula.
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
| 5 | `Main` | Configurando la firma visible — el nudo del recorrido |
| 6 | `EstadoPin` | Pidiendo PIN |
| 7 | `EstadoPinIncorrecto` | PIN incorrecto |
| 8 | `EstadoFirmando` | Firmando, con las tres fases |
| 9 | `EstadoExito` | Firmado |
| 10 | `EstadoErrorFirma` | Error de firma, en el pie del panel |
| — | `EstadoPreferencias` | Diálogo de preferencias |
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
ficheros lo llevan byte a byte idéntico, y `comprueba.sh` lo verifica.

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

Tres cosas que el canvas da por buenas y el código no puede sostener tal cual.
No las resuelvas por tu cuenta: son cambios de ficha (ID-44).

1. **`EstadoPreferencias` ofrece «Junto al documento original»** como destino
   del firmado. El ADR-0011 dice que eso **no es implementable** bajo el
   arenero: escribir un hermano del fichero del portal deja un `.xdp-…`
   huérfano y **no da error**. El canvas es anterior a esa medición.
2. **`EstadoPreferencias` no tiene «Recordar mi actividad» ni «Vaciar la
   lista»**, que sí existen en el código y los exige el ID-34.
3. **El panel enseña datos que hoy nadie calcula**: «27 páginas · 2,4 MB», «Ya
   lleva 1 firma: la tuya será una cofirma» y el resumen «2 firmas». El código
   pasa el tamaño y las firmas como desconocidos, y detectar si un PDF ya viene
   firmado está fuera del alcance del #81.

[#80]: https://github.com/sgomez/rfirma/issues/80
