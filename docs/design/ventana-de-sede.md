# Ventana de sede

La ventana que abre rFirma cuando una **sede electrónica** lo invoca con
`afirma://`, o una orden de terminal con `-certgui`. Cubre el trámite entero: la
espera del canal, el consentimiento, la firma, el desenlace y la salida sin
certificado utilizable. Lo que se ve de cada momento lo dicen sus historias, no
esta ficha: ver «Historias». Aquí van el flujo entre momentos y el porqué de cada
decisión.

**Una sola ficha para toda la ventana, no una por momento.** Es una ventana con
una secuencia, no cinco pantallas independientes, y partirla obligaría a leer
varios ficheros para saber qué ve la persona de principio a fin. Es la excepción
declarada a la regla de «una ficha por pantalla» de
[prototyping.md](../agents/prototyping.md).

## Casos de uso que la usan

- **Firmar desde una sede electrónica**, de principio a fin: `sign` y `cosign`.
- **Ceder los datos de identidad a una sede** (`selectcert`): el mismo recorrido
  sin el momento de firma. **No es identificarse**: la operación devuelve el
  certificado público X.509 y nada más —ni reto, ni firma, ni prueba de que la
  clave privada sea de quien consiente—, así que la ventana nombra una cesión de
  datos personales y no promete una identificación.
- **Firmar un lote**, remoto o local, con el mismo consentimiento.
- **Guardar o cargar ficheros** por orden de la sede, donde la ventana solo
  nombra lo que el diálogo del portal está preguntando.
- **Firmar desde la terminal con `-certgui`** (ADR-0041): solo el consentimiento y
  el secreto; ver «Origen orden de terminal».

No la usa el recorrido de firma local, que es de
[`ventana-principal.md`](ventana-principal.md) y [`panel-de-firma.md`](panel-de-firma.md).

## Qué resuelve

AutoFirma, invocado por una sede, enseña un selector de certificados a secas: no
dice quién pide la firma, ni qué se va a firmar, ni que haya una sede detrás; y
entre que se acepta y que la sede responde no aparece nada. Esta ventana pone lo
que falta: una **confirmación escrita** antes de firmar y un **acuse visible**
después, sin arrastrar la ventana principal a un trámite ajeno y corto.

## Forma de la ventana

**Diálogo de 520 × 420 px con la barra de título del sistema**, sin cabecera de
aplicación, menú, bandeja de recientes ni pie de destino: sugerir que hay más
dentro invita a buscar cosas que no están. Dos regiones fijas en todos los
momentos: un **cuerpo** que se desplaza en vertical y un **pie** con raya
superior y las acciones a la derecha. En los momentos de firma y de salida el pie
mide **56 px clavados**, para que aparecer o desaparecer una acción no mueva nada.

- **La barra de título es del escritorio, no del frontal.** Una barra pintada en
  HTML no la conoce el gestor de ventanas: la ventana no se podía arrastrar. Con
  las decoraciones del sistema vienen el título, la cruz, el menú del gestor y el
  arrastre, ya en el idioma y el tema del escritorio; y la ventana mide lo que se
  le pide, sin el inset que GNOME/Wayland daba a las ventanas sin decorar.
- **El cuerpo no recorta** (`overflow: hidden` fuera): el desplegable de
  certificados flota por encima de la ventana, y recortarlo fue un defecto medido
  ([design-system.md](design-system.md#desplegable)).
- **Llena el hueco que le da la ventana** en lugar de fijar los 520 × 420 en el
  CSS: fijar el tamaño en dos sitios dejó un fondo desnudo alrededor cuando las
  cifras no coincidían.
- **Las historias la enseñan centrada, con su tamaño y un marco de ventana.**

## Flujo entre momentos

Los momentos los publica el backend por `SiteErrandPort`; la ventana los obedece
y no lleva relojes propios de espera. La numeración es la de las historias y la
de las cabeceras de los componentes.

1. **Aviso del cliente web antiguo (0).** Si la página trae un cliente anterior al
   mínimo, el aviso sale antes de abrir el canal y retiene el arranque sin
   detenerlo: mientras se lee, la sede no encuentra a nadie escuchando. Descartarlo
   —con su botón o con Escape— deja el trámite en la espera normal.
2. **Espera del canal (1).** Mientras el canal no se abre. Si el reloj de respaldo
   del backend vence sin que el navegador conecte, el momento pasa a «no ha
   llegado»; si rFirma ya sabe que no va a abrirse (sin puertos o sin CA local),
   se enseña la reparación sin esperar. Cerrar abandona el trámite.
3. **Marcar el área de la firma visible (1c).** Solo si la sede pide firma PAdES
   con firma visible. Llega antes del consentimiento y reutiliza el visor de la
   ventana principal. Cancelar cierra el diálogo del área, no el trámite
   (ADR-0019).
4. **Consentimiento (2).** El corazón del trámite. Siempre aparece, también con un
   solo certificado; consentir lleva a la firma.
5. **Confirmar lo que señala el validador (2b).** Solo cuando el original no
   firmaría sin confirmación. Continuar repite la validación y el backend publica
   el siguiente momento; cancelar contesta `CANCEL` a la sede y deja el desenlace
   «cancelado».
6. **Firmando (3).** Dos tramos: rFirma firma, y la respuesta viaja a la sede.
   Guardar y cargar ficheros pasan por una variante propia, donde la persona
   contesta en el diálogo del portal.
7. **Desenlace (4).** Firmado, cancelado, lote, guardado, cargado o rechazado. Se
   cierra solo a los 15 segundos, salvo los rechazos que piden actuar.
8. **Sin certificado utilizable (5).** Alternativa al consentimiento cuando no hay
   nada que elegir. Salir abandona el trámite.

### Cuatro invariantes

1. **Una sede no provoca una firma silenciosa.** El consentimiento aparece aunque
   la sede pida selección automática; solo se salta, con un único candidato, si la
   persona lo permitió en Preferencias y no hay ningún aviso que enseñar
   (ADR-0032).
2. **No hay bandeja, ni destino, ni memoria.** El documento de la sede no se
   recuerda ni entra en recientes. El visor solo aparece en el momento 1c.
3. **Nunca se enumera lo que la sede descartó**, ni su criterio: es política de la
   sede, no información de quien firma.
4. **Los dos canales van desacompasados a propósito.** Lo que recibe la sede (la
   firma, `CANCEL` o el código de error) sale de inmediato; la ventana no es el
   acuse, es donde vive la precisión que el código no puede llevar. La excepción
   es el rechazo de la petición misma al analizarla: el original lo enseña en su
   diálogo de error y no contesta hasta que se cierra, y rFirma hace lo mismo.

## Decisiones y su porqué

### Espera y reparación

- **La ventana nace oculta y se revela cuando hay algo que enseñar.** Un retardo
  de gracia para pintar no hace falta: la ventana nunca es visible antes de tener
  contenido.
- **Quien decide «no ha llegado» es el backend**, no un temporizador del frontal.
  Nunca se cierra sola.
- **La reparación no diagnostica.** rFirma no puede saber si el permiso del
  navegador se denegó, así que son dos recetas, Chrome y Firefox, y la persona
  elige la suya. Solo texto, sin capturas: el aviso del navegador se describe por
  su forma y se cita el botón que hay que pulsar.
- **El bloque de la CA local va aparte y primero en Chrome**: sin CA el navegador
  ni llega a preguntar.
- **La dirección de ajustes de Chrome se copia, no se pulsa**: un `chrome://` no
  es navegable desde fuera.
- **La frase obligatoria vive en el pie**, no como tercer paso de cada receta:
  «reintentar» es un botón de la sede, y esta ventana no lo tiene.
- **Quien avisa de que el canal no se abre es esta ventana, no el escritorio**: el
  aviso llega donde duele y con la reparación al lado, así que el escritorio no
  estrena ninguna franja de diagnóstico para lo mismo.
- **Límite de espacio medido**: con la caja útil de unos 329 px, la receta de
  Chrome deja seis píxeles de margen. Si la prosa crece, lo primero que cae bajo
  el pliegue es el botón de copiar, que es justo lo que hay que enseñar.

### Consentimiento

- **Orden del cuerpo**: origen, certificado, nota de acotado si la hay, caja del
  documento con sus firmas previas; pie fijo con cancelar y la acción principal.
  La nota de acotado va debajo del desplegable porque habla de lo que la lista
  contiene y se lee después de verla.
- **SHA-1 permitido**: cuando la persona lo permitió en Preferencias y la sede lo
  pide, una línea informativa, en el mismo lugar que la nota de acotado, dice «Esta
  sede pide SHA-1, un algoritmo obsoleto. Lo tienes permitido en Preferencias.» No
  corta el trámite.
- **SHA-1 por permitir**: cuando una firma suelta, un lote remoto o un lote
  local piden SHA-1 y la
  persona no lo permitió en Preferencias, un aviso con el triángulo, «Firma poco segura», y
  debajo «Esta sede usa SHA-1, un método de firma antiguo y menos seguro. Firma
  solo si confías en ella.». El botón dice «Firmar solo esta vez», siempre con
  la cuenta atrás, y consentir permite SHA-1 solo en esa operación (ADR-0023).
- **El origen se nombra a secas**: atribuye sin afirmar, porque el `Origin` es
  falsificable. Dice también el formato pedido, para que nadie firme a ciegas un
  reto de autenticación creyendo que es un documento. Sin origen válido queda una
  etiqueta serena en la misma línea, sin caja ni icono: una caja con dos líneas
  daba la misma información con más ruido.
- **El documento se nombra solo por lo que dice de sí mismo** (título de los
  metadatos, páginas, tamaño). La petición no trae el nombre del fichero, y
  fabricar uno sería inventarlo. Sin título y sin origen no se rellena ningún
  silencio con un invento.
- **La cofirma y la contrafirma se distinguen antes de consentir**: firmar junto a
  las firmas existentes no es firmar sobre ellas, y es la única diferencia que la
  persona puede juzgar. La contrafirma dice además sobre cuáles.
- **El selector de certificado es el de la ventana principal, el mismo
  componente** ([`panel-de-firma.md`](panel-de-firma.md#certificado)). La lista
  mide como mucho 300 px y queda anclada bajo el campo, flotando aunque el cuerpo
  se desplace. El rótulo es el mismo también en `selectcert`: el título y la línea
  de qué se envía ya distinguen el caso.
- **La rama de identidad nombra la cesión de datos**, no una identificación que
  no ocurre. Negarla con una frase («esto no es una firma») es la verborrea que
  la regla de redacción de [design-system.md](design-system.md) ya echó de la
  ventana: decir lo que se hace basta.
- **Las firmas previas son el aviso compacto del panel de firma**, el mismo, con
  el mismo diálogo «Ver firmas» ([`dialogo-ver-firmas.md`](dialogo-ver-firmas.md)),
  dentro de la caja del documento. El porqué está en
  [`panel-de-firma.md`](panel-de-firma.md#decisiones). Un lote no trae los PDF, así
  que no hay firmas previas que contar.
- **La sede no bloquea ni pide confirmación por una firma no válida**, como
  AutoFirma: esta pantalla ya es un consentimiento y lo informa. «¿Firmar de todos
  modos?» es solo de la ventana principal.
- **Una firma que rFirma no sabe leer no es un rechazo**: es desconocimiento
  nuestro, y rechazar dejaría a rFirma rechazando documentos que AutoFirma sí
  firma. La advertencia vive dentro del mismo consentimiento, con tono de
  información y no de alarma, y esa firma cuenta como un problema más en el aviso.
- **El lote remoto solo da un recuento**: rFirma no recibe los PDF, sino una
  definición de lote que resuelve el servidor de la sede, así que no hay títulos
  que listar. El lote local sí trae el resumen de cada elemento, en una lista
  desplazable dentro del marco fijo, sin tapar el desplegable ni el pie.
- **Se firma con el teclado, y por eso hay cuenta atrás.** Con el certificado
  recordado elegido, la acción principal se lleva el foco y un Intro consiente.
  Para que ese Intro no llegue por descuido, la acción nace desactivada con la
  cuenta atrás en su etiqueta y a los tres segundos queda activa y con el foco;
  si la persona ya movió el foco, no se le quita. Se apaga en
  [Preferencias](preferencias.md).
- **Una acción principal por pantalla**: `--primary` para ella, `--ghost` para
  salir y para las microacciones, y `--secondary` no se usa en ningún momento de
  esta ventana.

### Marcar el área

- **Es el único momento en que la ventana crece**, porque en la caja útil no cabe
  una página que se pueda marcar. Se reparte como la ventana principal: visor a
  la izquierda y barra a la derecha con el bloque «En qué páginas» del panel de
  firma.
- **Ni modelo ni rúbrica**: lo que va dentro del recuadro lo pone la sede
  (ADR-0019).
- **Cerrar con la cruz es pulsar cancelar**, como cerrar el diálogo del área en el
  original. Con la firma visible obligatoria y sin área en la petición, la sede
  recibe `SAF_43` y queda el desenlace «cancelado»; en los demás casos sigue el
  consentimiento.

### Confirmar

- **Se pregunta con las palabras del original**, elegidas por el código del
  mensaje que cruza la frontera. Un código que rFirma no sepa redactar sale con
  una frase genérica que lo nombra, porque es lo único que permite reportarlo:
  rFirma no reinterpreta ni gradúa un aviso cuyo sentido conoce el original.
- **Llega antes del consentimiento**, así que no hay documento que resumir ni
  certificado que elegir. El original pregunta lo del PDF certificado después de
  elegir certificado porque lo descubre al firmar; rFirma lo descubre al leer y
  pregunta antes, con el mismo resultado para la sede.
- **Con `headless=true` no hay pregunta**: la sede recibe `SAF_50`. Con
  `allowSigningCertifiedPdfs=false`, el PDF certificado se rechaza sin preguntar
  con `SAF_35`.

### Firmando

- **No es el diálogo de progreso de la ventana principal.** Allí se listan las
  fases trifásicas porque hay un fichero que guardar; aquí contar la «prefirma»
  sería estado interno del motor
  ([`dialogo-progreso-firma.md`](dialogo-progreso-firma.md)).
- **Dos momentos y ninguno criptográfico**: firmar, con el certificado que la
  persona acaba de elegir —lo único que reconoce como suyo—, y devolver la firma,
  que importa porque es el tramo que ya no depende de rFirma. La barra avanza de
  verdad entre ambos para que no se vean iguales.
- **Hasta dónde se puede parar**: mientras rFirma firma, cancelar es limpio —la
  sede no ha recibido nada—. Con la respuesta ya en camino no hay nada que
  cancelar y el pie se queda vacío en vez de ofrecer un botón que mentiría.
- **Guardar y cargar no preguntan en esta ventana** (ADR-0011): la orden abre el
  diálogo del portal y aquí solo se nombra el fichero que propone la sede, nunca
  una ruta.

### Desenlace

- **Tres tipos de final y en todos la sede ya tiene su respuesta.** Firmado y
  cancelado llevan la fila del documento —es lo único que dice qué se acaba de
  firmar—; el rechazo no, porque ahí nunca llegó a haber documento.
- **Firmado dice que rFirma no guarda copia**, la única frase que no se deduce
  mirando: la aplicación sí tiene recientes y aquí no entra nada. El cancelado no
  añade nada, porque el título ya lo dice.
- **El rechazo se enseña aunque la persona no pueda arreglarlo**: acaba de
  arrancarse un programa en su equipo a petición de una web, y un rFirma que
  aparece y desaparece en silencio es indistinguible de uno roto. Lo accionable es
  el detalle copiable, para llevárselo a quien mantiene la sede.
- **Cada rechazo propio de la sede lleva una acción**, salvo las firmas que rFirma se niega a hacer (reintentar, contactar con la
  sede, cerrar la otra aplicación, elegir otro certificado) y es lo que protege
  `REFUSAL_ACTION_OF`. Los rechazos del token, el puente y el documento se cuentan
  con el título que les da la ventana principal, sin redactarlos dos veces. Solo
  cae en la frase genérica lo que ni el backend sabe clasificar.
- **Las firmas que rFirma se niega a hacer** (SHA-1, cofirma o
  contrafirma de factura, contrafirma fuera de CAdES, CMS y XAdES; ADR-0023) dicen
  que no es un fallo de quien firma, y esa causa ocupa el lugar de la acción. La
  nota para quien mantiene la sede va dentro de la caja del detalle y se copia con él.
  El rechazo por SHA-1 añade, antes de la caja, la pista «Si confías en esta sede,
  puedes permitir SHA-1 en Preferencias → Firma y volver a firmar desde la sede.»
- **La caja del detalle es de la sede y solo de la sede.** El enlace a comentarios
  y ayuda, que solo aparece cuando ni se sabe qué se rechazó, va fuera: eso no se
  lleva a la sede, se reporta a rFirma.
- **Se cierra solo a los 15 segundos, no a los 5**: con 5 no da tiempo a leer, y
  el caso que lo decide es el rechazo, donde irse solo reproduciría el síntoma que
  el aviso venía a evitar. Los rechazos que piden actuar o que llevan la ayuda se
  quedan abiertos. El botón de cerrar es el único `--primary`.
- **El rechazo de la petición misma por un canal ya abierto no contesta hasta
  cerrar**: el `SAF_NN` sale al pulsar cerrar o al cerrar la ventana; si llegan
  varios, se enseñan y se contestan de uno en uno. Los demás rechazos de
  transporte se contestan en el acto y no enseñan nada, porque el original
  tampoco.

### Sin certificado

- **No es una variante del consentimiento**: no hay nada que consentir ni elegir,
  el desplegable no pinta nada y el botón principal no puede decir «firmar».
- **Las dos causas se sienten distintas porque la salida lo es.** «No tienes
  ninguno» tiene arreglo que no depende de la sede. «La sede los excluyó todos»
  ofrece lo mismo, porque el recién instalado quizá sí valga, y dice cuántos
  certificados tiene la persona —estado de su almacén—, sin decir qué rechazó la
  sede de cada uno. Los caducados no se entregan nunca (ADR-0023).
- **Volver a buscar es una microacción del cuerpo**, por si se instaló un
  certificado con la ventana abierta; cerrar está en el pie.
- **Salir abandona el trámite**, por el pie o por la cruz del sistema: la sede no
  ha recibido nada todavía. Solo el desenlace cierra sin cancelar.

### Origen orden de terminal

La acción es de la persona, que acaba de escribir la orden: no hay frase de
origen ni nada dirigido a la sede, y no se repite lo que la orden ya dice.

- **Consentimiento**: el título nombra el documento de `-i`, recortado por el
  medio conservando principio, final y extensión, con la ruta completa en el
  tooltip. Sin rutas, destino ni nota de `-filter`.
- **La ventana solo elige**: se cierra al aceptar el secreto o al cancelar, y el
  resultado sale en la terminal; no hay firmando ni desenlace en ventana.
- **Sin certificado utilizable** se abre la ventana aunque no haya terminal.
  «Excluidos» es que el `-filter` no deja ninguno, y solo ofrece cerrar.
- **`-config` con firma visible o comprobación de firmas** no añade pantallas.

### El diálogo del secreto no cambia

Es exactamente [dialogo-pin.md](dialogo-pin.md): sin artboard propio ni palanca de
contexto, porque dos sitios donde mirar la misma pantalla serían dos verdades.

### Lo que se descartó

- **Una frase de origen, filas de carpeta y destino o «firmando» en ventana para
  la terminal**: atribuían a un tercero lo que lanza la propia persona, repetían
  rutas recién escritas y ensanchaban sin necesidad el contrato del elector
  gráfico.
- **El nombre del fichero en el botón**: el botón lo recorta a unos 40 caracteres.
- **Reutilizar el selector de certificados de AutoFirma tal cual**, sin decir
  quién pide ni qué se firma.
- **No enseñar nada mientras se firma**: es el fallo actual.
- **Listar las fases criptográficas durante la firma**: son estado interno del
  motor.
- **Mutilar el desplegable para que cupiera** (152 y luego 156 px con
  desplazamiento propio): las dos veces era tapar el fallo real.
- **Una quinta situación de consentimiento para «cero tras el filtro»**: es otra
  situación y vive en un solo sitio, «sin certificado».
- **Un lote de documentos listado para el lote remoto**: su respuesta es un
  recuento.
- **Prosa que pone en guardia sin dar información**: la regla de redacción está en
  [design-system.md](design-system.md).
- **Temporizadores propios en la ventana** para decidir «no ha llegado» o para
  retrasar el pintado.
- **Una caja aparte para el origen sin identificar** y **la línea fija «ya lleva
  una firma»**, sustituida por el aviso de firmas previas.

## Componentes y tokens

La ventana se compone con los primitivos `Button`, `Stack` y `Row` del sistema de
diseño, el selector de certificado compartido con el panel de firma, el visor del
documento en el momento 1c y el aviso de firmas previas del panel. Los iconos son
los del sistema de diseño.

Clases `rf-*` de texto y campo —`rf-title`, `rf-prose`, `rf-hint`, `rf-label`,
`rf-input`— y las propias de la ventana en `SedeWindow.css`.

Tokens: `--rf-bg`, `--rf-surface`, `--rf-text`, `--rf-text-muted`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-primary`, `--rf-on-primary`,
`--rf-radius-md|lg|pill`, `--rf-shadow-elevated`, `--rf-space-xs|sm|md`. Ni un
color ni una sombra literales.

## Historias

La verdad de lo que se ve es el código y estas historias, junto a sus componentes
en `rfirma-app/src/sede/`. Cada momento pinta su componente, desnudo y con props
finas, sobre el fondo de la ventana. `SedeView.stories.tsx`
(«Pantallas/Ventana de sede») compone la ventana entera con su marco: `Waiting`,
`Consent` y `Signed` a 520 × 420 y `MarkingTheArea` a 1080 × 660. `SedeView` no
conoce ningún puerto de Tauri; `SedeWindow` es la parte conectada a
`SiteErrandPort`.

| Momento | Historias |
| --- | --- |
| 0 · aviso del cliente antiguo | `SedeOldWebClient.stories.tsx`: `OldWebClient` |
| 1 · espera | `SedeWaiting.stories.tsx`: `Waiting`, `Unreachable`, `NoChannel` |
| 1c · marcar el área | `SedeMarking.stories.tsx`: `Marking`, `UnreadableDocument` |
| 2 · consentimiento | `SedeConsent.stories.tsx`: una por certificado, varios, acotado, sin título, cofirma, contrafirma sobre todas o sobre las últimas, firmas previas válidas o con problema, lote remoto y local, cesión de identidad, sin origen, cuenta atrás y orden de terminal |
| 2b · confirmar | `SedeConfirm.stories.tsx`: `ShadowAttackSuspect`, `ModifiedForm`, `CertifiedPdf`, `UnknownMessage` |
| 3 · firmando | `SedeSigning.stories.tsx`: `Signing`, `Returning` |
| 3 · guardar y cargar | `SedeTransfer.stories.tsx`: `Saving*`, `Loading*` |
| 4 · desenlace | `SedeOutcome.stories.tsx`: un final por tipo y un rechazo por cada acción, el desconocido, el del escritorio y el de sin origen |
| 5 · sin certificado | `SedeNoCertificate.stories.tsx`: `NoneInstalled`, `ExcludedBySite`, `InstallFailed` y las dos de terminal |

`src/stories.test.tsx` las pinta todas con axe, así que una historia nueva queda
revisada sin escribir otro test. El momento de consentimiento, que comparten las
pruebas, está en `sede/testing/fixtures/sedeWindow.ts`; el resto de lo que usan las historias:
las firmas previas y el PDF en blanco en `sede/testing/fixtures/previousSignatures.ts`,
los espías de las órdenes en `sede/testing/fixtures/sedeView.ts` y los
marcos —520 × 420, y 1080 × 660 al marcar el área— en
`rfirma-app/.storybook/decorators/sedeWindow.tsx`.

## Claves i18n

Todos los textos salen del catálogo; el castellano está en `po/es.po`. Por
momento:

- **Cliente antiguo y espera**: `sede.oldWebClient.*`, `sede.waiting.*`,
  `sede.unreachable.*` y `sede.repair.*`.
- **Marcar el área**: `sede.marking.*`.
- **Consentimiento**: `sede.consent.*` (origen según la operación y el formato en
  `asksSignature*`, `asksIdentity` y las variantes `unknownOrigin*`; lo del
  documento en `untitled`, `counterSignature*` y `signingKind.*`; el lote en
  `batch*` y `localBatchRound*`; lo que se envía en `willSend`; la acotación en
  `narrowed*`; la cuenta atrás en `countdown`; la terminal en `terminalTitle`).
- **Confirmar**: `sede.confirm.messages.*`.
- **Firmando y ficheros**: `sede.signing.with`, `sede.returning.*`,
  `sede.saving.*` y `sede.loading.*`.
- **Desenlace**: `sede.outcome.*`, y para los rechazos `sede.refusals.*`,
  `sede.refusalCauses.*` y `sede.siteNotes.*`.
- **Sin certificado**: `sede.noCertificate.*`.
- **Acciones comunes**: `actions.sign`, `actions.cancel`, `actions.close`,
  `actions.continue`, `actions.dismiss`, `actions.lookAgain` y `actions.copy`.
