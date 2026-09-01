# Artboards importados de Claude Design

**Esto es una importación de un solo uso, y se borra al terminar el [#80].**

Son los doce artboards del canvas de Claude Design «Autofirma de escritorio en
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
comprimida y le faltan tokens; manda el bundle versionado (ID-47). Los doce
ficheros lo llevan byte a byte idéntico, y `comprueba.sh` lo verifica.

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
