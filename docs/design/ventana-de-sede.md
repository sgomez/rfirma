# Ventana de sede

La ventana que abre rFirma cuando una **sede electrónica** lo invoca con
`afirma://`. Es la única interfaz nueva del hito v0.5 «Que llame la sede»
([#317](https://github.com/sgomez/rfirma/issues/317), mapa
[#308](https://github.com/sgomez/rfirma/issues/308)), y cubre el trámite entero:
la espera del canal, el consentimiento, la firma, el desenlace y la salida sin
certificado utilizable.

**Una sola ficha para toda la ventana, no una por momento.** Lo decidió el
[#332](https://github.com/sgomez/rfirma/issues/332): es **una ventana con una
secuencia**, no cinco pantallas independientes, y partirla obligaría a leer dos
ficheros para saber qué ve la persona de principio a fin. Esto es la excepción
declarada a la regla de «una ficha por pantalla» de
[prototyping.md](../agents/prototyping.md).

## Casos de uso que la usan

- **Firmar desde una sede electrónica** (v0.5) — de principio a fin: es el único
  caso de uso que abre esta ventana.
- **Identificarse ante una sede** (`selectcert`) — el mismo recorrido sin el
  momento de firma: se elige certificado, se envía la identidad y se acaba.

No la usa el recorrido de firma local: ahí la interfaz es
[`ventana-principal.md`](ventana-principal.md) y su
[`panel-de-firma.md`](panel-de-firma.md), y esta ventana **no sustituye nunca**
lo que hubiera abierto en ellas.

## Qué resuelve

Hoy AutoFirma, invocado por una sede, enseña un selector de certificados a
secas: **no dice quién pide la firma, ni qué se va a firmar, ni que haya una
sede detrás**; y entre que se acepta y que la sede responde no aparece nada en
absoluto. Esta ventana pone las dos cosas que faltan —una **confirmación
escrita** antes de firmar y **acuse visible** después— sin arrastrar la ventana
principal a un trámite que es ajeno y corto.

## Forma de la ventana

**Ventana tipo diálogo de 520 × 420 px, con la barra de título del sistema.**
No lleva la cabecera de la aplicación, ni menú, ni bandeja de recientes, ni pie
de destino: sugerir que hay más dentro invita a buscar cosas que no están. Dos
regiones fijas en los cinco momentos:

1. **Cuerpo**, `flex:1`, con 16 px de relleno arriba y abajo y `--rf-space-md` a
   los lados: la caja útil son **≈329 px**.
2. **Pie**, con raya superior y las acciones a la derecha. En los momentos de
   firma y de salida mide **56 px clavados**, con `height` fijo, para que
   aparecer y desaparecer «Cancelar» no mueva nada de sitio.

**La barra de título la pone el escritorio, no el frontal.** Se dibujó una
propia de 32 px con el nombre y la cruz, y era un error: una barra pintada en
HTML no la conoce el gestor de ventanas, así que la ventana **no se podía
mover, ni arrastrar, ni nada de lo que se espera de una ventana**. Con las
decoraciones del sistema vienen el título, la cruz, el menú del gestor y el
arrastre, ya en el idioma y el tema del escritorio. Además, quitarlas hacía que
GNOME/Wayland diera 26 px de inset por lado —+52 en cada eje, constante sea cual
sea el tamaño pedido—, y con ese margen el diálogo quedaba flotando dentro de un
marco vacío; con las decoraciones puestas, la ventana mide lo que se le pide.

En los artboards la ventana se dibuja centrada sobre un lienzo de 720 × 600 px
que representa el escritorio, para que se vea su tamaño real.

### Cuatro invariantes

1. **Una sede nunca provoca una firma silenciosa.** Los parámetros `headless` y
   `mandatoryCertSelection` se ignoran los dos: la pantalla de consentimiento
   aparece siempre, también cuando sólo hay un certificado.
2. **No hay visor, ni bandeja, ni destino, ni memoria.** El documento que manda
   la sede no se recuerda en ninguna parte y no entra en recientes.
3. **Nunca se enumera lo que la sede descartó**, ni el criterio con que lo
   descartó. Eso es política de la sede, no información de quien firma.
4. **Los dos canales van desacompasados a propósito.** Lo que la sede recibe
   —la firma, `CANCEL` o el código de error— sale de inmediato, sin esperar a
   que nadie cierre nada ([#316](https://github.com/sgomez/rfirma/issues/316)).
   Esta ventana no es el acuse: es donde vive la precisión que el código de
   error no puede llevar.

### Un solo documento por petición

`dat` trae **un** PDF. El `batch` está **fuera de la v0.5** (contrato del
protocolo §6; «Out of scope» del mapa #308). Se dibujó igualmente como
comprobación de resistencia, y la respuesta está medida: **la ventana no crece
con N documentos**, mengua. El `batch` viaja con URL de prefirma y de posfirma y
los documentos los resuelve el `DocumentManager` **del servidor de la sede**, así
que rFirma no recibe los PDF sino una definición de lote: no hay título, ni
páginas, ni tamaño que listar. Si algún día entra, la ventana enseña **un
recuento** —«12 documentos»— y nada más: ni lista, ni nombres, ni desplazamiento.

## Estructura, momento a momento

### 1 · Esperando el canal — `SedeEspera`

Qué se ve mientras el canal no se abre, y qué se ve cuando ya no va a abrirse.

- **Retardo de ~400 ms antes de pintar nada.** El camino feliz abre el canal en
  ~44 ms, así que quien llega a ver esta pantalla es quien espera de verdad.
- Un solo umbral de **~30 s** para pasar de «Conectando con la sede» a «La
  petición no ha llegado». **Nunca se cierra sola.**
- El camino de reparación **no diagnostica**: rFirma no puede saber si el
  permiso se denegó, así que es un **conmutador de dos recetas** —Chrome y
  Firefox— y la persona elige la suya. Sólo texto, sin capturas: el aviso del
  navegador se describe **por su forma** («la franja bajo la barra de
  direcciones», «el panel junto a la barra») citando el botón `Permitir`.
- **El bloque de la CA local va aparte del conmutador y primero en Chrome**: sin
  CA el navegador ni llega a preguntar.
- La dirección `chrome://settings/content/loopbackNetwork` **se copia, no se
  pulsa**: un `chrome://` no es navegable desde fuera.
- **La frase obligatoria vive en el pie**, no como tercer paso de cada receta:
  «Tras permitir, vuelve a la sede y pulsa **Reintentar**». `Reintentar` es un
  botón **de la sede**, y por eso esta ventana no lo tiene.
- Cerrar durante la espera **abandona el trámite** y libera el `idsession`, sin
  confirmación.

Medido con la caja útil de 328–333 px: «esperando» ocupa 207 px, la receta de
Firefox 260 px y la de Chrome **322 px** — le quedan seis píxeles. Si la prosa
vuelve a crecer, lo primero que cae bajo el pliegue es el botón `Copiar` y la
ruta por el candado, que es justo lo que hay que enseñar.

### 2 · Consentimiento — `SedeConsentimiento`

El corazón del ticket: la pantalla que hoy no existe.

- **Origen**: `sede.ejemplo.gob.es pide tu firma.` (o `pide que te
  identifiques.`). Nombrar el origen a secas **atribuye sin afirmar**, que es lo
  que pedía el [#312](https://github.com/sgomez/rfirma/issues/312).
- **Documento**: sólo lo que el PDF dice de sí mismo —título de sus metadatos si
  lo trae, páginas, tamaño, y si ya viene firmado, con el aviso de **cofirma**—.
  **No hay nombre de fichero ni ruta**, porque el protocolo no los trae: el
  `extraData` con el nombre va en la **respuesta**, no en la petición.
- **Certificado**: **el mismo desplegable de `panel-de-firma.md`**, sin
  reinventarlo — mismas clases, mismo relleno de fila, misma agrupación
  `Disponibles` / `No utilizables` y el mismo alto máximo de lista de **232 px**.
  Ver [«Los desplegables flotan»](design-system.md) para por qué esa lista no se
  recorta aunque se salga de los 420 px de la ventana.
- **Qué se envía**, en una línea: «Se enviarán tu **nombre**, tu **DNI**, el
  **emisor** del certificado y su **número de serie**».
- **Acción principal**: `Firmar`, o `Identificarse` cuando la operación es
  `selectcert`. `Cancelar` en `--ghost`.

Cinco situaciones dibujadas: un certificado; varios **acotados por la sede** —con
la nota «*sede* ha limitado los certificados válidos» **debajo** del desplegable,
porque es una nota sobre lo que la lista contiene y se lee después de verla—;
entrega de identidad sin firma; el PDF **sin título y sin origen**, que junta
los dos silencios y **no rellena ninguno con un invento**; y **ya firmado y con
alguna firma no reconocida** (#355, #363).

En esta quinta situación el PDF trae alguna firma cuyo `/SubFilter` rFirma no
sabe leer. **No es un rechazo**: el PDF certificado sí invalida con certeza y
por eso se rechaza sin preguntar; esto es desconocimiento nuestro, y rechazarlo
dejaría a rFirma rechazando documentos que AutoFirma sí firma (ID-298). Se
pregunta, y la pregunta vive **dentro del mismo consentimiento** — no hay un
sexto momento. La frase es de información, no de alarma: «rFirma no reconoce
alguna de las firmas que ya tiene este documento, y al añadir la tuya podrían
dejar de verse como válidas», con el mismo icono de información y el mismo
borde de 1 px que el origen sin identificar (ID-302). El botón sigue diciendo
`Firmar`, y **«firmas sin registrar» no aparece en la interfaz**. Cancelar aquí
es cancelar el trámite, como en cualquier otra situación del consentimiento.
No se enseña recuento ni titulares de las firmas que sí se entienden: rFirma no
tiene validador y no lo va a tener en esta versión, y enseñar «válida» sin
poder sostenerlo es peor que el silencio (ID-305).

### 3 · Firmando — `SedeFirmando`

Qué enseña la ventana entre que la persona acepta y que la firma vuelve a la
sede. Hoy no enseña nada, y ése es exactamente el fallo.

- **No es el diálogo de progreso de la ventana principal.** Allí se listan las
  tres fases —prefirma, firma, posfirma— porque la persona ha pedido un fichero
  y el reparto trifásico explica por qué tarda. Aquí no hay destino que enseñar
  y contar «prefirma» sería estado interno del motor. Ver
  [dialogo-progreso-firma.md](dialogo-progreso-firma.md).
- **Dos momentos, y ninguno es criptográfico**: «Firmando», con el certificado
  que la persona acaba de elegir —lo único que puede reconocer como suyo—, y
  «Enviando la firma a *sede*», que importa porque es el tramo en el que ya no
  depende de rFirma.
- **La barra avanza de verdad** entre los dos, 45 % y 88 %: si marcaran lo mismo,
  las dos fases se verían iguales.
- **Hasta dónde se puede parar**: mientras rFirma firma, `Cancelar` es limpio
  —la sede no ha recibido nada—. Cuando la respuesta ya va de camino no hay nada
  que cancelar, y **el pie se queda vacío** en vez de ofrecer un botón que
  mentiría.
- **Cero acciones principales** en toda la pantalla.

### 4 · Desenlace — `SedeDesenlace`

Tres desenlaces, y en los tres la sede ya ha recibido su respuesta.

| Desenlace | Título | Lo que añade |
| --------- | ------ | ------------ |
| Firmado | «Firmado y enviado» | «La firma ya está en *sede*» y, como nota, **«rFirma no guarda copia»** — la única frase de las tres que no se deduce mirando, porque la aplicación **sí** tiene bandeja de recientes y aquí no entra nada |
| Cancelado | «Has cancelado la firma» | Nada: el título ya lo dice |
| Rechazo | «rFirma ha rechazado la petición» | La incompatibilidad enunciada nombrando el origen, más un **detalle copiable** |

En **firmado** y en **cancelado** el cuerpo enseña además **la fila del
documento** —título de los metadatos, páginas y tamaño, la misma del
consentimiento—: es lo único que dice *qué* se acaba de firmar, y hace falta
justo en la pantalla que avisa de que rFirma no guarda copia. En el **rechazo**
no la hay, porque ahí nunca llegó a haber documento. Cada desenlace se encabeza
con su icono: visto, aspa y triángulo.

El **rechazo** cubre los del transporte (#316) —filtro no reconocido,
`signaturePages=append`, versión de protocolo no soportada, falta `format`, un
segundo `afirma://` con un trámite vivo—, que ocurren **antes** de que haya nada
que consentir. El argumento para enseñarlo no es que la persona pueda
arreglarlo, porque no puede: es que **acaba de arrancarse un programa en su
equipo a petición de una web**, y un rFirma que aparece y desaparece en silencio
es indistinguible de uno roto. Lo único accionable es el detalle copiable, para
llevárselo a quien mantiene la sede.

**La ventana se cierra sola a los 15 segundos**, no a los 5: con 5 no da tiempo a
leer, y que hiciera falta más tiempo era la prueba de que sobraba texto. El
botón dice `Cerrar` —«Cerrar ahora» sobraba— y es el único `--primary` de la
pantalla.

### 5 · Sin certificado utilizable — `SedeSinCertificado`

**No es una variante del consentimiento**: es otra situación. Ahí hay algo que
consentir y un certificado que elegir; aquí no hay ni una cosa ni la otra, el
desplegable no pinta nada y el botón principal no puede decir `Firmar`.

Las dos opciones **se tienen que sentir distintas porque la salida es distinta**:

- **No tienes ninguno.** Tiene arreglo y el arreglo no depende de la sede: hay
  acción principal —`Instalar un certificado…`, el único `--primary`— y la
  microacción `Volver a buscar` en `--ghost`, copiada de «4 · Sin certificados»,
  por si se instaló mientras la ventana estaba abierta.
- **La sede los ha excluido todos.** Instalar otro no arregla nada, porque quien
  decide es la sede: la pantalla se queda **sin acción principal**. Se dice
  cuántos tienes —«tus 3 certificados»— porque eso es estado del almacén de la
  persona, y ahí se acaba.

`Cerrar` está en el pie de las dos, porque en las dos hay que poder salir con
una etiqueta y no sólo por la cruz. `Volver a buscar` es una **microacción del
cuerpo**, no del pie. Y salir de aquí **abandona el trámite**: la sede no ha
recibido nada todavía, así que las dos puertas —el pie y la cruz del sistema—
liberan el `idsession`, igual que durante la espera y el consentimiento. La del
pie pasa por el frontal; la del sistema llega al backend como `CloseRequested`,
y allí abandona el trámite igual. Sólo el desenlace
cierra sin cancelar, que es donde la sede ya tiene su respuesta.

### El diálogo del secreto no cambia

Cuando el almacén pide PIN o contraseña, **es exactamente el diálogo de
[dialogo-pin.md](dialogo-pin.md)**, sin ninguna diferencia: mismo objetivo,
mismo texto, mismo pie. No tiene artboard propio ni palanca de contexto en esta
página, y la frase que se llegó a escribir para el caso de sede —que cancelar
cancela— se borró por explicar lo evidente.

## Estados

| Estado | Artboard | Acción principal |
| ------ | -------- | ---------------- |
| Esperando el canal | `SedeEspera` · `momento = esperando` | ninguna; `Cancelar` en `--ghost` |
| El canal no se abre (Chrome / Firefox) | `SedeEspera` · `no-va-chrome`, `no-va-firefox` | `Instalar…` (la CA local) |
| Consentimiento de firma | `SedeConsentimiento` · `forma = confirmacion` | `Firmar` |
| Consentimiento de identidad | `SedeConsentimiento` · `situacion = entregar identidad` | `Identificarse` |
| Firmando | `SedeFirmando` · `firmando · se puede cancelar` | ninguna; `Cancelar` en `--ghost` |
| Devolviendo a la sede | `SedeFirmando` · `devolviendo a la sede` | ninguna; el pie queda vacío |
| Firmado / cancelado / rechazado | `SedeDesenlace` | `Cerrar` |
| Sin ningún certificado | `SedeSinCertificado` · `ninguno` | `Instalar un certificado…` |
| Todos excluidos por la sede | `SedeSinCertificado` · `excluidos` | ninguna; `Cerrar` |

## Componentes y tokens

Clases: `.rf-root`, `.rf-row`, `.rf-stack`, `.rf-gap-xs|sm`, `.rf-surface`,
`.rf-title`, `.rf-prose`, `.rf-body`, `.rf-hint`, `.rf-label`, `.rf-text-muted`,
`.rf-input`, `.rf-btn` con `--primary` y `--ghost`.

Tokens: `--rf-bg`, `--rf-surface`, `--rf-text`, `--rf-text-muted`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-primary`, `--rf-on-primary`,
`--rf-radius-md|lg|pill`, `--rf-shadow-elevated`, `--rf-space-xs|sm|md`.

**Un solo criterio de botones**, copiado de `Main`, «2b · Elegir certificado» y
«5b · Páginas sin sello», sin inventar ninguno: **una** acción principal por
pantalla en `--primary`; `--ghost` para salir, cancelar y para las microacciones
en línea (`Copiar`, `Ver`, `Cambiar`); `--secondary` sólo para una alternativa de
peso al lado de la principal, que en esta ventana **no existe en ninguna
pantalla**.

Ni un color ni una sombra literales: el panel del desplegable se ordena con
`z-index:5` —el mismo de la cabecera de la ventana principal— y
`--rf-shadow-elevated`.

## Decisiones

Validado el **05/09/2026** en el canvas
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
página **«Ventana de sede · v0.5»**, con los cinco artboards `SedeEspera`,
`SedeConsentimiento`, `SedeFirmando`, `SedeDesenlace` y `SedeSinCertificado`. La
copia legible sin cuenta está en
[`docs/design/artboards/`](artboards/README.md), y las anotaciones de esa página
guardan cada medida.

**Lo que se descartó, y por qué:**

- **Reutilizar el selector de certificados tal cual** (palanca `forma`, opción
  «hoy · selector de certificado»). Es lo que hace AutoFirma y se dibujó para
  poder compararlo: no dice **quién** pide, ni **qué** se firma, ni que haya una
  sede detrás. La palanca se conserva como registro de la comparación.
- **No enseñar nada mientras se firma** (palanca `momento`, opción «hoy · la
  ventana no aparece»), dibujada como el hueco vacío de 520 × 420 que es. Ése es
  el fallo actual.
- **Listar las tres fases criptográficas** durante la firma, como hace el
  diálogo de la ventana principal: aquí no hay destino que enseñar y las fases
  son estado interno del motor.
- **Cerrar sola a los 5 segundos.** No daba tiempo a leer, y el caso que decide
  es «rechazo × se cierra sola»: cerrarse sola reproduciría el síntoma que el
  aviso venía a evitar.
- **Mutilar el desplegable a 152 px** para que cupiera en la ventana. Era tapar
  el fallo real; la lista vuelve a 232 px y **sobresale**, que es lo correcto
  ([design-system.md](design-system.md)).
- **Una quinta situación de consentimiento, «cero tras el filtro de la sede».**
  No era una variante del consentimiento sino otra situación, y se mudó entera a
  `SedeSinCertificado` · `excluidos`: el caso vive en un solo sitio.
- **Un artboard propio para el PIN** con una palanca de contexto. La pantalla es
  idéntica a la del recorrido local; dos sitios donde mirarla serían dos
  verdades.
- **Un lote de documentos** (palanca `lote`), que está fuera del hito. La palanca
  se conserva porque su respuesta —un recuento, nunca una lista— ya está medida.
- **Toda la prosa que ponía en guardia sin dar información.** El detalle está en
  la regla de redacción de [design-system.md](design-system.md), con los ejemplos
  de esta tanda.
