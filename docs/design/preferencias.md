# Preferencias

Los ajustes de la aplicación. Se abre desde el menú de la
[cabecera](cabecera.md) y ocupa la ventana entera bajo ella.

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido, en cualquier momento.

## Estructura

**Un diálogo a pantalla completa**, no una pantalla a la que se navegue: ocupa
todo lo que hay bajo la [cabecera](cabecera.md), que se queda **intacta con su
estado de documento** detrás. Se cierra con `Escape` y con `Cerrar`, y mientras
está delante el foco no se escapa a los controles de la ventana.

Tres regiones, de izquierda a derecha y de arriba abajo:

1. **Índice de secciones**, columna fija a la izquierda. Encabezado
   «Preferencias» y una fila por sección —*Firma*, *Privacidad*, *Apariencia*—.
   La sección activa lleva fondo y borde; las demás van en
   `--rf-text-muted` sin borde. Existe para el desplazamiento vertical que
   traerán los ajustes de la ficha 19 en adelante —el selector de módulo
   PKCS#11 y lo que venga con los instaladores nativos—: con tres secciones aún
   se ven todas de un vistazo, pero el sitio ya está.
2. **Columna de contenido**, centrada y desplazable, con las secciones apiladas.
   Cada sección abre con su rótulo en versalitas y un `.rf-divider` debajo.
3. **Pie fijo** con `Cerrar` abajo a la derecha. Fijo y no al final del
   contenido: en una pantalla que se desplaza, un botón de cierre que se va con
   el desplazamiento es un botón que no está.

Los cambios se aplican al hacerlos: **no hay «Guardar» ni «Cancelar»**.

### Geometría

- Índice de **220 px**, con `--rf-space-sm` de relleno y 2 px entre filas. Cada
  fila mide 36 px de alto, con `--rf-radius-md` y `--rf-space-xs` de relleno
  lateral.
- Columna de contenido con **720 px** de ancho máximo, centrada, y
  `--rf-space-md` de relleno lateral. El aire sobrante queda a los lados: a
  ancho completo un interruptor y su ayuda acaban separados por medio metro de
  nada.
- Pie fijo con `--rf-space-sm` de relleno y borde superior de 1 px en
  `--rf-border-subtle`, sobre `--rf-surface`.
- Cada interruptor es una fila con la pastilla de **40×24 px** (pomo de 16 px)
  **delante** del texto, 16 px de separación, y el texto en `.rf-prose` con su
  ayuda en `.rf-hint` 4 px debajo. La ayuda se sangra hasta la columna del
  texto: 40 px de pastilla más los 16 de separación.
  Esos 16 px son **de esta pantalla**: el panel de firma usa el mismo
  interruptor a 8 px (`panel-de-firma.md`), así que aquí se piden aparte con
  `switch--wide`. La sangría de la ayuda se calcula con esa misma separación y
  no con un número escrito a mano, que es lo que la mantiene en la misma
  columna que «Vaciar la lista».
  **Los 4 px separan el botón de la ayuda, no el rótulo de la ayuda** (ID-44).
  El artboard mete rótulo y ayuda en la misma columna, los dos dentro de la
  fila; aquí la ayuda queda **fuera** del botón —dentro se sumaría al nombre
  accesible y el lector de pantalla leería el párrafo entero al llegar al
  interruptor—, y el botón conserva sus 44 px de alto mínimo de área de
  pulsación (sección 8 del [sistema de diseño](design-system.md)). Con un
  rótulo de una línea eso deja aire dentro del botón, así que el hueco que se
  ve bajo el texto es mayor que esos 4 px. Entre copiar el hueco del canvas y
  conservar el área de pulsación, manda el área de pulsación.
- «Vaciar la lista» y «Cambiar carpeta…» cuelgan del ajuste que los explica y
  siguen esa misma sangría. Son botones secundarios de 32 px de alto, 8 px de
  relleno lateral y 12 px de cuerpo — el mismo tamaño menor que el `Cambiar`
  del pie del panel.
- Los dos desplegables son `.rf-field` con `.rf-label` y un cierre que
  reutiliza `.rf-input`, con el chevrón a la derecha y la lista flotando 4 px
  por debajo.

**El desplegable no es un `<select>` nativo.** Se intentó, y no vale: el cierre
se estila con CSS, pero la lista que se abre la pinta el sistema de ventanas
—GTK, bajo WebKitGTK— y no la hoja de estilos, así que las opciones salían con
los colores del escritorio en medio de una pantalla hecha con los tokens del
sistema de diseño. No es una limitación que se pueda rodear con más CSS: ese
trozo de interfaz no es nuestro. A cambio hay que reponer a mano lo que el
elemento nativo daba gratis —`combobox` + `listbox` con
`aria-activedescendant`, flechas, Inicio, Fin, Intro, Escape, cierre al pulsar
fuera y foco de vuelta—, y eso es lo que hace `Select`. Un `<div>` con un
`onClick` no es un desplegable, es un dibujo de uno.

## Los ajustes

### Firma

1. **Recordar la última configuración de firma visible** (interruptor, activo
   por omisión). El interruptor, las cinco casillas, el motivo y el tamaño del
   recuadro se reutilizan en el siguiente documento. Apagado significa **no
   guardarla**: el recuadro arranca en el valor por omisión en cada documento.

   **La posición no se recuerda aquí.** Va por documento, en su fila de
   recientes, porque reponer sobre otro documento una posición elegida para uno
   distinto es lo que rechaza el ID-22: el recuadro acaba fuera de página o
   encima del texto. Este ajuste gobierna lo global; la posición la gobierna la
   [bandeja](bandeja-de-documentos.md).

2. **Dónde se guarda el documento firmado**. Una fila con el **nombre** de la
   carpeta —no su ruta— y un botón **«Cambiar carpeta…»** al lado, que abre el
   selector de directorio del sistema. Por omisión, la carpeta de documentos
   del usuario.

   **No es un desplegable.** Lo fue, con una sola opción dentro, que es un
   control que finge elegir. Bajo el arenero la aplicación escribe en la
   carpeta pero la única palabra que tiene de ella es su último segmento, y
   enseñar la ruta donde se puede y el nombre donde no sería la misma pantalla
   contando cosas distintas según el empaquetado; un selector de directorio
   devuelve exactamente ese último segmento en los cuatro canales.

   Se comprueba **antes de firmar**: si no está o no se puede escribir, se
   avisa en el pie del panel y ahí mismo se ofrece `Cambiar`; ni se degrada a
   otro sitio ni se apaga el botón de firmar. La carpeta **no se crea nunca**
   si no está.

   *Junto al documento original* **no aparece aquí en el flatpak**: bajo el
   arenero la aplicación no puede saber de qué carpeta salió el original. Es
   una capacidad que llegará con los instaladores nativos, no una opción
   atenuada que le cuente al usuario nuestros problemas de empaquetado.
   Razonamiento y alternativas descartadas en el
   [ADR-0011](../adr/0011-destino-del-documento-firmado.md).

### Privacidad

3. **Recordar mi actividad** (interruptor, activo por omisión), con un botón
   **«Vaciar la lista»** al lado. Cubre los documentos recientes y el
   certificado usado la última vez: es la misma promesa a quien firma en un
   ordenador compartido. Apagarlo **borra** lo ya guardado, previa
   confirmación; vaciar sin apagar es «hoy no, mañana sí».

### Apariencia

4. **Tema** (desplegable): *El del sistema*, *Claro* u *Oscuro*. Por omisión,
   el del sistema, que **no es «claro»**: es no forzar nada y dejar que mande
   `prefers-color-scheme`. Los otros dos escriben `data-theme` en `<html>`, que
   es lo que los tokens de color del bundle leen para redefinir los roles. El
   cambio se aplica en caliente, como el resto de la pantalla.
5. **Idioma** (desplegable). Español, català, euskara, galego e inglés: son
   cinco desde el ID-124, que sacó el valencià porque sus reglas de plural no
   son las del castellano. El cambio se aplica en caliente. Un idioma solo
   aparece aquí si tiene **todas** las cadenas traducidas. En la primera
   ejecución sale del locale del sistema cotejado contra esos cinco, con
   español como recurso; no hay diálogo de bienvenida que pregunte lo que la
   aplicación ya sabe.

Los valores posibles viven **dentro** de los desplegables. Nada de textos
debajo enumerando lo que el propio control ya muestra al abrirse.

## Estados

- **Normal**: los ajustes tienen siempre valor, y se guardan al elegirlos.
- **Confirmando el borrado**: apagar «Recordar mi actividad» abre un
  `.rf-dialog` pequeño **encima** de la pantalla completa, con lo que se va a
  perder —los documentos recientes y el certificado—, `Cancelar` como `--ghost`
  y `Borrar y apagar` como primario. El interruptor **no se mueve** hasta que
  se confirma.

  Un diálogo y no una confirmación en línea: el borrado es irreversible y no
  admite «casi», y en una pantalla con tres secciones a la vista una
  confirmación en línea compite por la atención con otras cinco filas. Tampoco
  se hace al revés —borrar y ofrecer deshacer— porque un aviso temporal sobre
  algo ya borrado es el fallo silencioso otra vez, y en un ordenador compartido
  puede caducar sin que nadie lo lea.
- **No se ha podido guardar el ajuste**: `ErrorNotice` **dentro de la sección
  donde se pulsó**, con el título en `.rf-title` a 14 px y el detalle en
  `.rf-hint`. Dice que se ha vuelto al valor anterior.
- **No se ha podido vaciar la lista**: el mismo `ErrorNotice`, siempre en
  *Privacidad*, pegado a «Vaciar la lista». Dice que los recientes siguen
  guardados.

Los dos avisos van por sección y no uno solo arriba: con tres secciones, un
aviso común obliga a leer el texto para saber qué se rompió. Antes esta ficha
decía «## Estados — Uno. Los ajustes tienen siempre valor», y por eso los dos
fallos de `App.tsx` —guardar la configuración y `forgetActivity`— no tenían
dónde pintarse y se tragaban en silencio.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-field`, `.rf-label`, `.rf-input`,
`.rf-hint`, `.rf-divider`, `.rf-btn--primary|--secondary|--ghost`,
`.rf-badge`, `--rf-surface`, `--rf-border-subtle`, `--rf-border-strong`. El
interruptor, el desplegable y las filas del índice se maquetan con tokens;
ninguno está en el sistema de diseño.

## Decisiones

**Por qué deja de ser un modal de 480 px.** Con cinco ajustes ya iba justo, y
lo que viene —el selector de módulo PKCS#11 de la ficha 19, y lo que traiga
cada hito— no cabe. Se descartó una ruta de un router: con guardado automático
y `Cerrar` como única salida no hay ningún estado al que navegar ni nada que
confirmar, así que lo que queda es un diálogo, solo que grande. Así `Escape`
sigue valiendo y `Cmd+,` en macOS sigue prometiendo lo que abre.

**La cabecera no cambia** mientras Preferencias está delante, estado del
documento incluido: la aplicación no se ha ido a ninguna parte y el documento
sigue cargado detrás. Apagar o sustituir ese estado contaría que se ha
navegado. El [ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md) sigue
intacto: una cabecera única, y `Preferencias…` sigue siendo una de sus dos
entradas.

**Los ajustes se guardan al elegirlos, en el disco.** La pantalla llama a
`PreferencesStore`, y debajo son `read_configuration` y `write_configuration`,
que pasan por `memory::Memory::remember_configuration`: el único sitio donde el
borrado del estado al apagar «Recordar mi actividad» no se puede olvidar.

**Dónde cae el documento firmado** está fijado en el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md): la carpeta por
omisión, el nombre en vez de la ruta, la comprobación previa sin degradación y
el `Cambiar` del pie que vale solo para una firma. **Queda pendiente enmendarlo**
con el selector de directorio que sustituye al desplegable: lo hace el
sub-issue de implementación al que toca, no esta ficha.

**Qué se recuerda entre sesiones y dónde vive** está fijado en el
[ADR-0010](../adr/0010-memoria-entre-sesiones.md): los dos interruptores, el
borrado que provoca apagar el segundo, y la comprobación previa de la carpeta
salen de ahí. El tema entró después, por la enmienda de ese mismo ADR.

El **alcance de la traducción** está fijado en
[#16](https://github.com/sgomez/rfirma/issues/16): las seis lenguas, cadenas
propias escritas desde cero con el vocabulario de `CONTEXT.md`, mensajes de
error que traducen situaciones nuestras y no el texto de PKCS#11 ni el del
puente Java, y texto de la firma visible que sigue al idioma de la aplicación.

Preferencias existe desde el primer día en lugar de esperar a tener «algo que
configurar»: el propio recorrido ya generó dos ajustes reales, y un menú que
promete preferencias y abre un diálogo vacío es peor que no tenerlas.

**Las tres diferencias con el canvas original han desaparecido**, y con ellas
la sección que las listaba: el artboard se rehizo el 02/09/2026 y ya trae
«Recordar mi actividad» con su «Vaciar la lista», el tema, y el destino sin
«Junto al documento original».

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Preferencias · a pantalla completa»
(`PreferenciasPantalla`). Decidido en el
[#123](https://github.com/sgomez/rfirma/issues/123).
