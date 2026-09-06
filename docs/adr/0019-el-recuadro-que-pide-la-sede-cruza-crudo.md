# El recuadro que pide la sede cruza crudo al puente, sin la conversión del local

El [ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md) manda
sobre el recuadro que la persona **arrastra sobre el visor**: nace de un
arrastre, se guarda en espacio de usuario y llega al puente tras dos
conversiones. Un trámite de sede no tiene visor sobre el que arrastrar nada:
el recuadro, si lo hay, viene ya puesto en los `extraParams` de la petición
(`signaturePositionOnPage*` y `signaturePage` o `signaturePages`). Es **otro
recuadro**, con otro origen y otra regla, y este ADR es el que manda sobre él.

Hay entonces **dos caminos del recuadro, y no comparten conversión**:

- **Lo elige la persona.** Se aplica la inversa de la `/Rotate` de la página
  (`T⁻¹`, en `signing::placement`), porque ella apuntó a un punto de la
  pantalla y el recuadro tiene que caer ahí. Es el
  [ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md) y la
  medición de `docs/research/coordenadas-recuadro-pades.md`.
- **Lo elige la sede.** Sus claves cruzan al puente **tal y como vinieron**,
  letra por letra, sin pasar por ningún tipo nuestro.

## Por qué la sede no recibe el arreglo

Los números que manda una sede están ajustados contra AutoFirma, no contra
el PDF. Medido en el tag `v1.9.2`: la conversión del diálogo de colocación de
AutoFirma (`SignPdfUiPanel.java`) es una regla de tres sobre
`getPageSizeWithRotation`, no aplica ningún `T⁻¹`, y la CropBox no aparece en
todo el proyecto. Una sede que calibró su recuadro con ese cliente cuenta con
ese comportamiento. Pasar sus coordenadas por el camino del visor las movería
**fuera de donde ella las puso**, y en una firma de sede la persona no ve
dónde cae el recuadro hasta que el documento ya está firmado.

En este camino «hacerlo bien» y «ser conforme» no son lo mismo, y manda ser
conforme: rFirma sustituye a AutoFirma ante sedes que no van a cambiar.

## Cómo se honra el recuadro: no emitiéndolo

Los `extraParams` de la sede son la base y los ajustes de rFirma se escriben
**encima** (`app::policies::merged_with`). Ese orden es la decisión: si se
invirtiera, la sede podría reescribir el recuadro que la persona acaba de ver
y consentir en el camino local. Y como la configuración de firma de un
trámite de sede va **sin colocación** (`placement: None`), la geometría de
rFirma no emite ni una clave que pise a las de la sede. `None` no significa
«sin recuadro»: significa «el recuadro no lo pone este lado».

Por eso el tipo que resume la petición (`SiteVisibleSignature`) no lleva
coordenadas dentro: dice **qué va a ocurrir** —lo colocó la sede, o no hay
recuadro que colocar—, no cuánto mide nada. Y por eso la prefirma del trámite
no lo lee: su trabajo ya está hecho cuando llega.

## Qué se decide sobre la petición, y qué no

Se decide una sola cosa, sobre los `extraParams` **ya expandidos**, que es
donde mira el original: si la petición lleva recuadro, si no lo lleva, o si lo
que pide no se atiende.

- **Lleva recuadro** cuando están las cuatro esquinas y una página (singular o
  plural; el plural gana cuando vienen las dos, como en el puente). Se mira que
  las claves **estén**, sin interpretar lo que traen: quien lee esos valores es
  el puente, y adelantarse a él sería tener dos opiniones sobre el mismo texto.
- **Las páginas contadas desde el final las resuelve el puente, y solo él.**
  `normalizePage` ya convierte `-1` en la última; resolverlas también en Rust
  daría la página equivocada.
- **`visibleSignature=want` sin recuadro** se rechaza con `SAF_43` en el acto.
  AutoFirma enseña ahí su diálogo de colocación y emite ese código si se
  cancela; rFirma no tiene diálogo, así que llega al mismo código
  directamente. La bandera se compara **sin recortar espacios**, como el
  original: un `" WANT "` allí no es obligatorio y la firma sale invisible, y
  recortar aquí endurecería una negativa que el original no hace.
- **`visibleSignature=optional` sin recuadro** firma invisible, y un
  `visibleAppearance=custom` sin datos estampa el aspecto por omisión. Es en
  lo que queda «la persona ha declinado colocar» cuando no hay ni visor ni
  diálogo.
- **`signaturePages=append` con recuadro puesto** se rechaza con `SAF_03`
  nombrando `properties`, no con `SAF_43`: añadir una página en blanco es
  modificar el documento antes de firmarlo, y la sede tiene que poder
  distinguir «falta un recuadro» de «un valor que rFirma no atiende». La
  negativa va **detrás** de la comprobación del recuadro, porque sin las
  cuatro esquinas el original tampoco añade página: ahí se firma invisible,
  igual que allí.

## Consequences

- La colocación es **opcional** en la orden de firma y en la configuración,
  y esa opcionalidad tiene un solo significado: trámite de sede. En el
  camino local la ventana la manda siempre, y que faltara en el JSON es un
  error de deserialización, no una firma invisible en silencio.
- Un cambio en `signing::placement` no toca a las sedes, y un cambio aquí no
  toca al visor. Las pruebas de cada camino son independientes, y la de la
  mezcla comprueba que el recuadro de la sede llega al puente **exactamente
  como vino**.
- Si sube la versión de `afirma-lib-itext` o cambia la conversión del diálogo
  de AutoFirma, la conformidad hay que volver a medirla contra ese tag.

## Considered Options

**Unificar los dos caminos en una sola conversión.** Es lo natural al ver dos
conversiones parecidas, y **rompe uno de los dos**: o el recuadro del visor
deja de caer donde la persona apuntó, o el de la sede se mueve fuera de donde
ella lo puso. Descartada.

**Leer las claves de la sede a un `Placement` y volver a serializarlas.** Un
viaje de ida y vuelta que pierde por el camino `signaturePage`, los rangos y
los índices negativos, y que da una segunda opinión sobre un texto que ya
interpreta el puente. Descartada.

**Resolver en Rust las páginas negativas** para validar el destino como se
hace en el camino local. Daría la página equivocada, porque el puente las
vuelve a resolver. Descartada: en este camino el destino lo valida el puente.

**Enmendar el ADR-0006 en lugar de escribir este.** Aquel ADR manda sobre un
recuadro que nace de un arrastre y se guarda por documento; ninguna de sus
reglas aplica a una petición que llega ya puesta y sin visor. Meterlo allí
haría que la próxima persona buscara la conversión de la sede donde vive la
del visor, que es justo la confusión que este ADR existe para evitar.
