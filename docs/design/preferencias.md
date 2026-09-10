# Preferencias

Los ajustes de la aplicación. Se abre desde el menú de la
[cabecera](cabecera.md) y ocupa la ventana entera bajo ella.

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido, en cualquier momento.

## Estructura

**Un visor de pestañas en vertical**, dentro de un diálogo a pantalla completa:
ocupa todo lo que hay bajo la [cabecera](cabecera.md), que se queda **intacta
con su estado de documento** detrás, y mientras está delante el foco no se
escapa a los controles de la ventana.

Tres regiones, de izquierda a derecha y de arriba abajo:

1. **Índice de secciones**, columna fija a la izquierda, **permanente y siempre
   visible**. Encabezado «Preferencias» y una fila por sección —*General*,
   *Firma*, *Certificados*, *Apariencia*—. La sección activa lleva fondo y
   borde; las demás van en `--rf-text-muted` sin borde. El índice **es** la
   pestaña: pulsar una fila no navega a ningún sitio, cambia el panel de la
   derecha.
2. **Panel de la sección activa**, centrado, y **solo esa**: a la derecha se
   pinta una sección y nada más. Abre con el rótulo de la sección en versalitas
   y un `.rf-divider` debajo, que es el **título de la página** que se está
   viendo. Ahí «Certificados» recupera su nombre completo, *Certificados en
   fichero*, que en la fila del índice no cabe.
3. **Pie fijo** con `Cerrar` abajo a la derecha, sobre `--rf-surface`.

**No hay vuelta atrás, ni botón ni migas**, porque no se navega: el índice
nunca desaparece, así que desde cualquier panel se ve dónde se está y se llega
a los otros tres de un gesto. Las dos salidas son `Cerrar` y `Escape`, y
**`Escape` cierra Preferencias entera** desde cualquier sección y sin
excepción: nunca «vuelve» al índice ni al primer panel.

**Se entra siempre por *General***, el primer elemento del índice. No se
recuerda la última sección visitada: lo que una pantalla de ajustes tiene que
recordar es el ajuste, no el sitio desde el que se miró.

**El índice no colapsa ni cambia de forma con el ancho.** Un solo
comportamiento a cualquier tamaño de ventana: ni versión estrecha, ni icono de
hamburguesa, ni panel que se retrae.

**El desplazamiento es del panel, no de la pantalla.** El índice y el pie no se
mueven nunca; que una sección se desplace es un detalle interno suyo, y con el
reparto de hoy solo se espera en *Certificados en fichero*.

Los cambios se aplican al hacerlos: **no hay «Guardar» ni «Cancelar»**, y
cambiar de panel no lo enturbia, porque no hay ninguna transacción que
confirmar al salir de una sección.

### Geometría

- Índice de **220 px**, con `--rf-space-sm` de relleno y 2 px entre filas. Cada
  fila mide 36 px de alto, con `--rf-radius-md` y `--rf-space-xs` de relleno
  lateral.
- Panel con **720 px** de ancho máximo, centrado, y `--rf-space-md` de relleno
  lateral. El aire sobrante queda a los lados: a ancho completo un interruptor
  y su ayuda acaban separados por medio metro de nada.
- Pie fijo con `--rf-space-sm` de relleno y borde superior de 1 px en
  `--rf-border-subtle`, sobre `--rf-surface`.
- **Dos niveles de encabezado dentro de un panel**, y ninguno estrena estilo:
  el **título de página** es `.rf-label` —12 px, peso 600, `--rf-text-muted`—
  en versalitas, con 0,6 px de espaciado entre letras y su divisoria debajo; el
  **encabezado de grupo** es `.rf-title` a 14 px, peso 700, color de texto
  pleno, en caja baja y **sin divisoria**; y la etiqueta de un control sigue
  siendo `.rf-label` a 12 px en caja baja y apagada. Los tres se separan por
  tamaño, peso y color a la vez, que es lo que impide confundir un grupo con la
  etiqueta del control de al lado. Van `--rf-space-md` entre el título de
  página y el primer grupo y entre grupos, con `--rf-space-xs` de respiro extra
  sobre «Privacidad», contra los `--rf-space-sm` que separan un encabezado de
  su primer control. **Solo *General* tiene grupos**; los otros tres paneles
  llevan únicamente título de página.
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

Son **siete**, repartidos en cuatro paneles.

### General

Es la entrada, y lleva los dos grupos que antes eran sección propia: primero
**Sedes**, después **Privacidad**.

#### Sedes

**Un solo control, y solo donde se puede cumplir** (ID-238, ID-240).

- **Quién atiende los enlaces de las sedes** (desplegable), con lo que el
  escritorio diga que hay registrado para `afirma://`: **ningún nombre de
  aplicación está escrito en el código**, ni «AutoFirma» ni «rFirma». Elegir
  escribe un `default` explícito en el `mimeapps.list` del `$HOME`, y **solo
  ahí**. Mientras no haya ninguno escrito, el valor es *Lo que decida el
  escritorio*, que desaparece de la lista en cuanto se elige a alguien: enseñar
  el primero de la lista como si estuviera elegido sería mentir.
- Debajo, la ayuda que **no se puede deducir mirando** (§11 del
  [sistema de diseño](design-system.md)): «Firefox usa la elección que guarda en
  sus propias preferencias». Es cierta, no la ve nadie y cambia lo que la
  persona hará si el enlace le sigue abriendo otra aplicación (ID-241).
- **Preguntarme al arrancar** (interruptor, activo por omisión): es lo que
  deshace el «No volver a preguntar» del banner de la
  [ventana principal](ventana-principal.md). Vive aquí porque es donde alguien
  va a buscar la pregunta que apagó.

**En el flatpak no hay ni desplegable ni interruptor**, sino una frase fija:
«Esta versión no puede cambiarlo: se elige en los ajustes del escritorio».
Medido: dentro del sandbox GIO contesta `None` a todos los esquemas, no existe
ningún portal de manejadores predeterminados y `set_as_default_for_type()`
**devuelve `True` mintiendo** (ID-240). Un desplegable ahí sería un control que
finge elegir, que es justo lo que esta ficha ya descartó para el destino.

#### Privacidad

1. **Recordar mi actividad** (interruptor, activo por omisión), con un botón
   **«Vaciar la lista»** al lado. Su ayuda es una línea: «Los documentos
   recientes y el certificado que usaste la última vez». Que apagarlo borre lo ya
   guardado se ve al apagarlo, en el diálogo de confirmación, así que no se
   anuncia también aquí. Cubre los documentos recientes y el certificado usado la
   última vez: es la misma promesa a quien firma en un ordenador compartido.
   Apagarlo **borra** lo ya guardado, previa confirmación; vaciar sin apagar es
   «hoy no, mañana sí».

2. **Avisarme cuando haya una versión nueva** (interruptor, activo por omisión),
   sin ayuda debajo. Está bajo «Privacidad» porque la comprobación de versión es
   **la única conexión saliente que abre rFirma**, y ese es el encabezado donde
   alguien va a buscar si la aplicación habla con fuera.

   **Está siempre, y se avisa siempre.** El spec pedía que el ajuste existiera
   *sólo* si nadie gestionaba la instalación, detectándolo por la URL del
   repositorio dentro de `sources.list.d` / `yum.repos.d` (ID-179). Se descartó:
   esa señal **no es fiable**. Un `.deb` se instala a mano tanto como desde un
   repositorio, el repositorio puede estar dado de alta y no traer rFirma, en el
   flatpak no hay ninguno de esos dos ficheros que leer, y aun leyéndolos habría
   que hurgar en la configuración del gestor de paquetes del anfitrión para
   decidir si se pinta un interruptor. Avisar siempre y dejar apagarlo cuesta
   menos y no miente. Con eso **el ID-179 se queda sin consumidor y no se
   implementa**, y el ID-180 pierde su condición.

   Dónde sale el aviso lo decide
   [ventana-principal.md](ventana-principal.md): la franja bajo la cabecera. A
   dónde lleva, [acerca-de.md](acerca-de.md).

### Firma

3. **Recordar la última configuración de firma visible** (interruptor, activo
   por omisión), **sin texto de ayuda debajo**. El interruptor, las cinco
   casillas, el motivo y el tamaño del recuadro se reutilizan en el siguiente
   documento. Apagado significa **no guardarla**: el recuadro arranca en el valor
   por omisión en cada documento. Eso es lo que hace, y es lo que la ficha tiene
   que saber; la pantalla no lo explica, porque explicarlo no cambia lo que la
   persona puede hacer con el interruptor.

   **La posición no se recuerda aquí.** Va por documento, en su fila de
   recientes, porque reponer sobre otro documento una posición elegida para uno
   distinto es lo que rechaza el ID-22: el recuadro acaba fuera de página o
   encima del texto. Este ajuste gobierna lo global; la posición la gobierna la
   [bandeja](bandeja-de-documentos.md).

4. **Dónde se guarda el documento firmado**. Una fila con el **nombre** de la
   carpeta —no su ruta— y un botón **«Cambiar carpeta…»** al lado, que abre el
   selector de directorio del sistema. Por omisión, la carpeta de documentos
   del usuario.

   **No es un desplegable.** Lo fue, con una sola opción dentro, que es un
   control que finge elegir. Bajo el sandbox la aplicación escribe en la
   carpeta pero la única palabra que tiene de ella es su último segmento, y
   enseñar la ruta donde se puede y el nombre donde no sería la misma pantalla
   contando cosas distintas según el empaquetado; un selector de directorio
   devuelve exactamente ese último segmento en los cuatro canales.

   Se comprueba **antes de firmar**: si no está o no se puede escribir, se
   avisa en el pie del panel y ahí mismo se ofrece `Cambiar`; ni se degrada a
   otro sitio ni se apaga el botón de firmar. Bajo el ajuste queda **una sola
   línea de ayuda**, «La carpeta no se crea nunca», que es la única de las tres
   que había cuyo borrado cambiaría lo que la persona puede esperar: las otras
   dos contaban cuándo se comprueba y qué pasa si falla, y eso ya lo cuenta el
   pie del panel en el momento en que ocurre.

   **«Junto al documento original» es condicional** (ID-184): la opción se
   ofrece **sólo cuando el entorno sabe devolver la ruta real del documento**.
   Donde no la sabe —el sandbox del flatpak, que entrega el fichero por un
   portal— la opción **no aparece**, y el ajuste se queda en la carpeta con su
   `Cambiar carpeta…`, exactamente como estaba.

   El motivo **no es privacidad**: enseñar la ruta de un documento que el
   usuario acaba de abrir no revela nada que su gestor de ficheros no enseñe
   todo el día, y el ID-185 retira esa justificación. El motivo es
   **corrección**: devolver una ruta que no se conoce es devolver una mentira, y
   una opción atenuada le contaría al usuario nuestros problemas de empaquetado.

   Los **dos estados miden lo mismo**: el bloque lleva `min-height` de 200 px,
   medido sobre el artboard, para que lo de debajo no salte según el canal.
   Razonamiento y alternativas descartadas en el
   [ADR-0011](../adr/0011-destino-del-documento-firmado.md).

### Certificados en fichero

5. **La lista de certificados instalados en rFirma**, con **dos gestos y nada
   más**: **«Añadir…»**, arriba a la derecha del panel, que abre el selector
   de ficheros del sistema, y **«Quitar»** al final de cada fila (ID-198).

   **Lo que identifica cada fila es el certificado, no el fichero.** Titular en
   negrita y, debajo, `DNI · emisor · caduca el …`. **Del fichero no se recuerda
   nada, ni la ruta** (ID-196): instalar copia lo que hace falta al almacén de
   rFirma, y el `.p12` de origen deja de importar en cuanto se cierra el
   selector. Por eso aquí no se pinta ninguna ruta, y no hay «volver a
   localizar»: un fichero que se mueve o se borra no rompe nada.

   Un certificado caducado **se queda en la lista**, con su insignia
   `Caducado`, por lo mismo que se queda en el desplegable del
   [panel de firma](panel-de-firma.md): que desaparezca no le explica nada a
   quien lo instaló.

   **Sin ninguno instalado**, un recuadro punteado con «Todavía no has instalado
   ninguno». Sin instrucciones dentro: el botón «Añadir…» ya está encima.

   **No se copia el registro de almacenes de AutoFirma**, con sus seis casillas
   y sus diálogos anidados. Allí hace falta porque la aplicación **elige** un
   almacén y sólo enseña ese; rFirma los barre todos y concatena el resultado,
   así que aquí no hay nada que elegir: sólo una lista de lo que se ha añadido a
   mano.

   **Una clave elíptica se rechaza al instalar, no al firmar** (ID-197), con un
   `ErrorNotice` en este mismo panel y un solo renglón: **«Ese certificado no
   es compatible con rFirma»**. Sin explicación técnica debajo: la curva, el
   mecanismo y la constante RSA-SHA256 no le sirven de nada a quien acaba de
   elegir un fichero, y quien sí sabe lo que es una clave elíptica no necesita
   que se lo cuenten aquí. El sitio importa más que el texto: sin esta guarda, la
   pantalla construiría el camino más corto al tropiezo —el kit de pruebas de la
   FNMT trae una carpeta entera de claves ECC—, y el fallo aparecería al firmar,
   con el documento delante.

   Es el único panel del que se espera que se desplace, y por eso el
   desplazamiento vive dentro de él y no en la pantalla.

### Apariencia

6. **Tema** (desplegable): *El del sistema*, *Claro* u *Oscuro*. Por omisión,
   el del sistema, que **no es «claro»**: es no forzar nada y dejar que mande
   `prefers-color-scheme`. Los otros dos escriben `data-theme` en `<html>`, que
   es lo que los tokens de color del bundle leen para redefinir los roles. El
   cambio se aplica en caliente, como el resto de la pantalla.
7. **Idioma** (desplegable). Español, català, euskara, galego e inglés: son
   cinco desde el ID-124, que sacó el valencià porque sus reglas de plural no
   son las del castellano. El cambio se aplica en caliente. Un idioma solo
   aparece aquí si tiene **todas** las cadenas traducidas. En la primera
   ejecución sale del locale del sistema cotejado contra esos cinco, con
   español como recurso; no hay diálogo de bienvenida que pregunte lo que la
   aplicación ya sabe.

Los valores posibles viven **dentro** de los desplegables. Nada de textos
debajo enumerando lo que el propio control ya muestra al abrirse.

## Estados

- **Normal**: los ajustes tienen siempre valor, y se guardan al elegirlos. La
  sección activa es la del índice, y al abrir es *General*.
- **Confirmando el borrado**: apagar «Recordar mi actividad» abre un
  `.rf-dialog` pequeño **encima** de la pantalla completa —índice incluido, que
  es lo único que se ve desde cualquier sección—, con lo que se va a perder
  —los documentos recientes y el certificado—, `Cancelar` como `--ghost` y
  `Borrar y apagar` como primario. El interruptor **no se mueve** hasta que se
  confirma.

  Un diálogo y no una confirmación en línea: el borrado es irreversible y no
  admite «casi». Tampoco se hace al revés —borrar y ofrecer deshacer— porque un
  aviso temporal sobre algo ya borrado es el fallo silencioso otra vez, y en un
  ordenador compartido puede caducar sin que nadie lo lea.
- **No se ha podido guardar el ajuste**: `ErrorNotice` **dentro del panel donde
  se pulsó**, con el título en `.rf-title` a 14 px y el detalle en `.rf-hint`.
  Dice que se ha vuelto al valor anterior.
- **No se ha podido vaciar la lista**: el mismo `ErrorNotice`, siempre en
  *General*, pegado a «Vaciar la lista». Dice que los recientes siguen
  guardados.
- **El certificado elegido no sirve**: el mismo `ErrorNotice`, siempre en
  *Certificados en fichero*, entre «Añadir…» y la lista. Un solo renglón, «Ese
  certificado no es compatible con rFirma», sin detalle debajo. La lista **no
  cambia**: lo que no se ha podido instalar no aparece en ella.

Los avisos se pintan donde se hizo el gesto, que con las pestañas es siempre el
panel visible, y nunca en una franja común arriba: un aviso común obliga a leer
el texto para saber qué se rompió. Antes esta ficha decía «## Estados — Uno.
Los ajustes tienen siempre valor», y por eso los dos fallos de `App.tsx`
—guardar la configuración y `forgetActivity`— no tenían dónde pintarse y se
tragaban en silencio.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-field`, `.rf-label`, `.rf-title`, `.rf-input`,
`.rf-hint`, `.rf-divider`, `.rf-btn--primary|--secondary|--ghost`,
`.rf-badge`, `--rf-surface`, `--rf-border-subtle`, `--rf-border-strong`. El
interruptor, el desplegable y las filas del índice se maquetan con tokens;
ninguno está en el sistema de diseño.

## Decisiones

**Por qué es un visor de pestañas y no una navegación de dos niveles.** El
enunciado del [#657](https://github.com/sgomez/rfirma/issues/657) pedía resolver
la vuelta atrás, el `Escape` ambiguo y qué hacer en ventana ancha. Con el índice
permanente tres de esas preguntas dejan de existir en vez de contestarse: no hay
camino de vuelta que recorrer porque no se navega, `Escape` no tiene dos
significados que aprender, y no hay una interfaz que se comporte de dos maneras
según el tamaño. Se descartó `AdwNavigationView` —el índice como página y cada
sección como la siguiente—: es lo único que respaldaría una vuelta atrás de un
nivel, y compra un gesto de más en cada cambio de sección a cambio de un
retroceso que aquí nadie necesita.

**Que `Escape` cierre Preferencias entera es lo único que respalda la GNOME
HIG**, que dice que `Escape` cierra el contenedor transitorio y **no dice nada
de retroceder un nivel**. Sale barato porque no hay «Guardar»: cerrar de golpe
desde cualquier panel no pierde nada, porque no hay nada sin aplicar.

**Por qué el reparto pasa de cinco secciones a cuatro.** Las cinco de antes
salieron de tener que caber en una columna con desplazamiento continuo: el
índice era el remedio al desplazamiento, no navegación. Al volverse páginas esa
restricción desaparece y el fallo nuevo es el contrario, la página medio vacía.
*Sedes* era el único huérfano claro —un desplegable y un interruptor, y bajo
flatpak ningún control, solo una frase fija: una página entera para una
disculpa—. *Privacidad* se le une dentro de *General* para que sus dos
interruptores queden a la vista al abrir Preferencias, sin un clic, y va al
final a propósito, porque se leen como un par —qué recuerda de mí y con quién
habla— y el desplegable de sedes en medio los desemparejaría. Se conserva la
palabra «Privacidad» como encabezado de grupo, que era lo único que se perdía al
fundirla. **Ningún ajuste se ha añadido ni quitado**: los siete son los mismos,
con el mismo comportamiento y la misma ayuda; lo que cambia es dónde viven.

**Por qué deja de ser un modal de 480 px.** Con cinco ajustes ya iba justo, y lo
que traiga cada hito no cabe: la v0.4 sola le añadió una sección entera con una
lista. Se descartó una ruta de un router: con guardado automático y `Cerrar`
como única salida no hay ningún estado al que navegar ni nada que confirmar, así
que lo que queda es un diálogo, solo que grande. Así `Escape` sigue valiendo y
`Cmd+,` en macOS sigue prometiendo lo que abre.

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

El **alcance de la traducción** lo fija el
[ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-cinco-idiomas.md): cinco
idiomas y ninguna variante, cadenas propias escritas desde cero con el
vocabulario de `CONTEXT.md`, mensajes de error que traducen situaciones nuestras
y no el texto de PKCS#11 ni el del puente Java, y texto de la firma visible que
sigue al idioma de la aplicación. En el desplegable sólo aparece el idioma que
esté al 100 %: la completitud es una puerta de construcción, no un filtro que se
aplique aquí.

Preferencias existe desde el primer día en lugar de esperar a tener «algo que
configurar»: el propio recorrido ya generó dos ajustes reales, y un menú que
promete preferencias y abre un diálogo vacío es peor que no tenerlas.

**La ficha 19 —el selector de módulo PKCS#11— ya no reserva sitio aquí.** Esta
ficha guardaba dos huecos para él, en el índice de secciones y en el argumento
del tamaño. Se retiran: la v0.4 no toca tarjetas ni DNIe y además **retira** la
fontanería de tarjeta que hoy se compila sin que nadie la use (ID-201 a
ID-204), así que la ficha 19 queda fuera de alcance y no hay nada que reservar.
El artboard nunca llegó a dibujarlo.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Preferencias · a pantalla completa»
(`PreferenciasPantalla`), con la palanca **Sección visible** —las cuatro
pestañas, con *General* por omisión— y las de contenido: **Destino**, que
recorre los dos entornos, y **Certificados en fichero**, con cuatro instalados,
ninguno y el rechazo de la clave elíptica. El contenedor y el reparto se
decidieron en el [#657](https://github.com/sgomez/rfirma/issues/657); lo
anterior, en el [#123](https://github.com/sgomez/rfirma/issues/123) y, lo de la
v0.4, en el [#250](https://github.com/sgomez/rfirma/issues/250) (ID-180, ID-184,
ID-196 a ID-198).
