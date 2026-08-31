# La firma visible se configura sobre el documento, no en un formulario

AutoFirma configura la firma visible con un formulario: página, posición
elegida en una rejilla, y un texto en el que el usuario escribe comodines como
`$$SUBJECTCN$$` y `$$SIGNDATE$$` que se sustituyen al firmar. El resultado no
se ve hasta que el PDF ya está firmado.

`rfirma` invierte las dos cosas.

**La posición es libre y se decide sobre el documento.** El recuadro se arrastra
sobre la página renderizada, con el zoom que haga falta. No hay rejilla de nueve
posiciones: dónde va la firma depende del documento —normalmente bajo el nombre
de la persona—, no de una casilla abstracta.

**El contenido se marca, no se escribe.** El usuario elige con casillas qué
aparece dentro del recuadro: su rúbrica, nombre y apellidos, DNI, fecha y hora,
y opcionalmente un motivo. La aplicación compone a partir de ahí la plantilla
con comodines que la biblioteca espera. Los comodines siguen existiendo en los
`extraParams` de PAdES, pero como detalle de implementación: no son vocabulario
de usuario.

**Firma visible y rúbrica son dos cosas distintas**, y la interfaz las separa:
la *firma visible* es el recuadro que se estampa en la página, que se activa o
no; la *rúbrica* es la imagen de la firma manuscrita escaneada que va dentro de
él, y es opcional. Confundirlas fue el fallo más caro del primer prototipo.

Como la apariencia queda decidida antes de la prefirma, se puede previsualizar
el resultado exacto antes de pedir el PIN.

## Consequences

- Hace falta **renderizar el PDF** en la interfaz, con paginación y zoom. Eso
  añade una dependencia de render que la firma en sí no necesitaba, y es el
  precio de que la posición sea libre.
- La posición viaja a la biblioteca como coordenadas en el sistema de
  referencia del PDF. Convertir de píxeles de pantalla con zoom a puntos PDF,
  con el origen abajo a la izquierda, es responsabilidad de la aplicación y es
  una fuente de errores de un solo píxel: necesita pruebas.
- La composición de la plantilla de texto a partir de las casillas es código
  nuestro, y es el único sitio donde aparecen los comodines. Añadir un dato
  nuevo al recuadro es añadir una casilla y una entrada a esa tabla.
- La sustitución de comodines la hace la biblioteca en la **prefirma**, y la
  postfirma debe recibir exactamente los mismos `extraParams` y el mismo
  instante de firma, o la firma sale inválida sin dar error. Ver el hallazgo
  registrado en `docs/research/firma-visible-trifasica.md`.
- Se pierde la posibilidad de teclear comodines arbitrarios que AutoFirma sí
  permite. Es deliberado: quien los necesite no es el usuario objetivo de este
  hito.
