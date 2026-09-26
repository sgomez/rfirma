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

**El contenido se elige, no se escribe con comodines.** El usuario elige un
modelo —Completa, Solo rúbrica o Personalizada—, cada uno con la firma real en
miniatura, y aparte si lleva su rúbrica. En Personalizada escribe una frase en
la que los datos del certificado —firmante, emisor, fecha— entran como
pastillas desde un menú, ya resueltos. No hay motivo aparte, ni fuente, tamaño
o color: el texto se ajusta al recuadro. La aplicación compone a partir de ahí
el texto que la biblioteca espera. Los comodines siguen existiendo en los
`extraParams` de PAdES, pero como detalle de implementación: no son vocabulario
de usuario.

**Firma visible y rúbrica son dos cosas distintas**, y la interfaz las separa:
la *firma visible* es el recuadro que se estampa en la página, que se enciende
o no; la *rúbrica* es la imagen de la firma manuscrita escaneada que va dentro de
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
- La composición del texto a partir del modelo y de sus pastillas es código
  nuestro, y es el único sitio donde aparecen los comodines. Añadir un dato
  nuevo al recuadro es añadir una pastilla al menú y una entrada a esa tabla.
- La sustitución de comodines la hace la biblioteca en la **prefirma**, y la
  postfirma debe recibir exactamente los mismos `extraParams` y el mismo
  instante de firma, o la firma sale inválida sin dar error. Ver el hallazgo
  registrado en `docs/research/firma-visible-trifasica.md`.
- Se pierde la posibilidad de teclear comodines arbitrarios que AutoFirma sí
  permite. Es deliberado: quien los necesite no es el usuario objetivo de este
  hito.
- Se descartaron dos formas anteriores del contenido. **Una casilla por dato**
  —firmante, emisor, fecha, rúbrica, motivo— componiendo un párrafo: tres
  modelos con la firma real se leen sin leer, y Personalizada cubre lo demás.
  **El motivo** como campo propio: era un segundo texto libre junto a una frase
  que ya lo es.

## Enmienda: la colocación es un rectángulo y un conjunto de páginas

Añadido con el hito v0.3 ([#148](https://github.com/sgomez/rfirma/issues/148)).
Este ADR daba por supuesta **una** página —la que se mira— y una posición
arrastrada sobre ella. Se sustituye por un concepto con nombre, **la
colocación**: un rectángulo en espacio de usuario y el **conjunto de páginas**
donde se estampa.

### Encender la firma visible la coloca

Hasta v0.2 el recuadro aparecía solo, de tamaño fijo, en la esquina de la
página que se mirara, y **seguía a quien pasaba de página**. Deja de ser así:
la firma visible está **apagada por defecto**, y firmar sin ella está permitido.
**Encenderla la coloca**: el recuadro aparece en la página a la vista, abajo a
la derecha, a un margen del borde, y desde ahí se arrastra o se redimensiona.
«Colocado» no es una bandera: **no existe «encendida y sin colocar»**. Apagarla
y volver a encenderla recupera la colocación que había.

La v0.3 hacía nacer el recuadro de un arrastre y exigía al menos una página con
firma visible: encendida y sin colocar, firmar estaba apagado. Se descartó
porque ese estado, con interruptor y sin recuadro, nadie sabía dibujarlo ni
explicarlo. Lo único que apaga firmar es un rango de páginas que no se puede
resolver (ID-22).

### Una página deja de ser el caso, y el conjunto es el caso

«Esta página», «algunas» y «todas» no son tres modos: son **un conjunto de
páginas** de tamaño 1, *k* o *n*. El puente acepta un conjunto cualquiera y
`all` produce exactamente el mismo resultado que la lista completa
([#150](https://github.com/sgomez/rfirma/issues/150)), así que «algunas» no
cuesta nada de más.

Lo que sí cuesta es la forma del PDF: es **un solo campo de firma con el widget
replicado**, o sea **el mismo sitio y el mismo tamaño en todas las páginas del
conjunto**. La alternativa estructuralmente limpia —un campo de firma por
página— convierte «estampar en 3 páginas» en «3 firmas del mismo certificado»,
cambia lo que `pdfsig` cuenta y lo que un validador informa, y no está medida.
Se acepta la limitación y **el trabajo se traslada a la interfaz**: decirla sin
engañar. El recuadro se dibuja **idéntico en todas las páginas del conjunto y en
ninguna más**; la página donde se arrastró no se dibuja distinta, y fuera del
conjunto la página va en blanco, sin fantasma a trazos.

### Donde el recuadro no cabe, la página se queda sin firma visible

`correctPositionSignature` recorta contra la **primera** página de la lista y
**descarta en silencio** aquellas donde no cabe la esquina inferior izquierda
([#150](https://github.com/sgomez/rfirma/issues/150)). Con tamaños mezclados en
el mismo PDF ocurre de verdad. El ID-22 rechaza la degradación **silenciosa**,
no la consentida: antes de firmar se avisa en un modal con «cancelar» o «firmar
de todos modos», que dice el recuento —*n* de las *m* **elegidas**, no de las
del documento— y **«sin firma visible», nunca «recortadas»**, porque la firma
criptográfica cubre el documento entero pase lo que pase.

### La previsualización que este ADR prometía se cumple, y no es una puerta

«Como la apariencia queda decidida antes de la prefirma, se puede previsualizar
el resultado exacto antes de pedir el PIN» era una promesa sin mecanismo, y el
recuadro se dejó vacío a propósito para no imitar en HTML al compositor
autoritativo. El sondeo [#115](https://github.com/sgomez/rfirma/issues/115)
desactivó la premisa: un **ciclo trifásico en seco** con un `PK1` inventado
produce bytes visibles idénticos a los del firmado de verdad, y `pdf.js` los
pinta sin código de dibujo nuevo. La regla que sale de ahí es una sola: **o es
la firma visible de verdad, o el recuadro va vacío** — no se enseña nunca una
aproximación. Sin certificado elegido no hay firma que componer, y la firma
visible no se enciende: el interruptor queda desactivado, con un aviso debajo,
hasta que se elige uno. Se descartó dejarla encender y colocar con el recuadro
vacío: un recuadro que no enseña nada no ayuda a decidir dónde va la firma.

Y una segunda regla, del mismo signo: **la vista previa no es una puerta**. Si
la firma visible no se puede componer, el recuadro lo dice y **se firma igual**; sobre
si se puede firmar manda el botón de firmar.

### Lo que este hito deja fuera, y por qué

- **Anclar a un campo de firma vacío preexistente.** Medido en el
  [#149](https://github.com/sgomez/rfirma/issues/149): `pdf.js` no distingue un
  campo vacío de uno ya firmado, equivocarse **no falla, borra** —el PDF sale
  con una sola firma, la nueva—, y el único filtro seguro deja fuera el caso
  estrella, el contrato a dos firmas. Además `signatureField` anula la geometría
  y apaga el multipágina sin avisar, así que sería otra rama, no un ajuste más.
- **«La última página» como ancla propia.** Se disuelve: si el arrastre fija la
  página, «la última» es «la página 4» en un documento de 4, y la colocación no
  viaja a otro documento porque se guarda en la fila del documento.
- **«Página nueva al final»** (`append`): inventa una página en el documento de
  otro.

### Consecuencias que se añaden

- La página **deja de ser un `u32` desnudo** y pasa a un tipo con nombre. El
  puente convive con dos convenios incompatibles para el valor `0`, y un número
  desnudo invita a importar el equivocado.
- **Rust valida el destino antes de llamar al puente.** `PdfUtil.getPages` no
  lanza nunca: recorta, avisa por `WARNING` y cae en la última página, de modo
  que `signaturePage=99` sobre un PDF de 3 páginas **firma en la 3 y devuelve
  éxito**. Es un agujero abierto hoy, sin multipágina de por medio, y no hay
  excepción que capturar.
- El **conjunto de páginas se recuerda por documento**, como el rectángulo: «las
  páginas 3, 7 y 9» no significa nada en otro PDF. El tamaño y el interruptor
  siguen siendo globales.
