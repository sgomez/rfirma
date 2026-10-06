# Diálogo de progreso de firma

Acompaña las tres etapas de la firma trifásica mientras se ejecutan. Bloquea la
ventana porque no hay nada que hacer hasta que termine, y porque interrumpir a
mitad rompe la firma.

Componente: `rfirma-app/src/signing/SigningProgressDialog.tsx`. Historias:
`SigningProgressDialog.stories.tsx`, en «Flujos/Firma/SigningProgressDialog», una por etapa en curso.

## Casos de uso que lo usan

- Firmar un PDF en local, al pulsar «Firmar», con la sesión del almacén ya
  abierta. Ocupa el mismo sitio que el [diálogo de secreto](dialogo-pin.md), que
  lo precede cuando hace falta.

## Flujo

Tres estados, uno por etapa en curso: prefirma, firma y postfirma. **No tiene
salida**: ni «Cancelar» ni cruz ni Escape, porque una vez empezada la firma no
hay marcha atrás. Al terminar, el velo se va y el [panel](panel-de-firma.md)
enseña el resultado, la firma o el error.

## Qué enseña

Un velo que cubre la ventana entera, con la cabecera y sus pestañas, y encima:

1. El título.
2. Las tres etapas, cada una con su marca: hecha, en curso o pendiente. La que
   está en curso lleva el peso y se anuncia como paso actual (`aria-current`),
   para que la forma no sea el único indicador (ver la sección 8 de
   [design-system.md](design-system.md#8-accesibilidad)).
3. Una barra de progreso fina, con rol de barra de progreso y nombrada por el
   título.
4. Una línea atenuada que pide no retirar la tarjeta.

## Variantes

- `Presign`: la primera etapa en curso, las otras pendientes.
- `Sign`: la primera hecha, la segunda en curso.
- `Postsign`: las dos primeras hechas, la última en curso.

## Textos

Claves: `progress.title`, `progress.stages.presign`, `progress.stages.sign`,
`progress.stages.postsign` y `progress.keepTheCard`. Los textos no se copian
aquí: se leen en `po/es.po`.

## Componentes y tokens

El primitivo `Dialog`, sin salida, y `CheckIcon`; `.rf-title`, `.rf-prose`,
`.rf-text-muted`, `--rf-primary`, `--rf-border-subtle`, `--rf-border-strong` y
`--rf-radius-pill`. Las medidas son de `SigningProgressDialog.css`.

## Decisiones

- **Se enseñan las tres etapas** porque la postfirma regenera el PDF entero y
  puede tardar: sin desglose, una espera larga tras teclear el PIN parece un
  cuelgue. Y cuando algo falla, saber en qué fase fue es lo primero que hace
  falta.
- **Lenguaje llano**: la etapa se nombra por lo que hace, no por el término del
  dominio.
- **Diálogo con velo y no etapas en el pie.** Se mantiene el velo para que el
  secreto, el progreso y el paso al resultado ocurran en el mismo sitio.
- **Sin salida a propósito.** Retirar la tarjeta a mitad rompe la firma; por eso
  no hay botones y el primitivo `Dialog` no instala Escape cuando no tiene
  salida.
- **La línea de la tarjeta sigue en el código.** Una versión anterior de esta
  ficha decía que se había ido con las tarjetas de la v0.4; el componente la
  conserva, y manda el componente.
