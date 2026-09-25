# Primer arranque

El asistente que ve quien abre rFirma por primera vez: cuenta qué es esto y qué
no es, y deja el equipo configurado —el certificado propio instalado y rFirma
puesta como programa que abren las sedes— sin mandar a nadie a buscarlo por su
cuenta. Es la primera pantalla de la aplicación, antes de que haya documento
alguno.

Sustituye al diálogo `trust/TrustNotice.tsx`, que sólo contaba, y absorbe el
disparo inicial de `desktop/UrlHandlerBanner.tsx`, que **desaparece del todo**
([#661](https://github.com/sgomez/rfirma/issues/661)): lo que quedaba de él
—elegir quién atiende los enlaces de las sedes— vive en la fila `Firma en sedes`
del [panel de estado](panel-de-estado.md). La diferencia de fondo con lo que hay
hoy es que este **informa y además hace**.

## Casos de uso que la usan

- **Primer arranque de rFirma** ([#658](https://github.com/sgomez/rfirma/issues/658),
  mapa [#652](https://github.com/sgomez/rfirma/issues/652) «La instalación se
  explica sola») — de principio a fin: es el único caso de uso que abre esta
  pantalla.

No la usa ningún otro recorrido. La reparación posterior de las dos cosas que
configura aquí vive en el panel de estado de rFirma
([#655](https://github.com/sgomez/rfirma/issues/655)), no aquí.

## Qué resuelve

Instalada la aplicación, quedan dos cosas por hacer que nadie va a adivinar: el
certificado propio que hace segura la conexión del navegador con rFirma, y que
las sedes electrónicas abran rFirma en vez de AutoFirma. Hoy la primera se
cuenta en un diálogo que no la hace, y la segunda aparece en una franja que se
dispara sola. El asistente junta las dos donde la persona ya está mirando, y de
paso pone por delante el deslinde: rFirma no es la aplicación oficial.

## Estructura

**Es la ventana principal de 1180 × 700 px, no un diálogo sobre ella.** El
primer arranque no tiene documento que tapar, y un modal con la aplicación
muerta detrás miente sobre lo que hay debajo. Lleva la cabecera única del
[ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md) —52 px, el nombre y
el menú— y nada más de la ventana principal: ni tira de pestañas, ni panel de
firma, ni recientes, porque todavía no hay ningún documento que enseñar.

Tres regiones:

1. **Cabecera**, 52 px, con raya inferior de 1 px `--rf-border-subtle`. En la
   ventana principal esa raya la pone la tira de pestañas; aquí no hay tira, así
   que la lleva la propia cabecera.
2. **Cuerpo**, `flex:1`, desplazable, con una **columna de lectura de 640 px
   centrada**. Arriba de todo, el indicador de paso: dos rayas de 22 × 4 px y
   «Paso *n* de 2».
3. **Pie**, con raya superior y las acciones a la derecha. **No flota sobre el
   contenido**: es hermano de la zona que se desplaza, así que no puede tapar
   un control enfocado.

### Dos pantallas

| # | Qué lleva | Pie |
| - | --------- | --- |
| 1 | Bienvenida: titular, qué es rFirma, el deslinde de independencia y el idioma | `Omitir configuración` · `Continuar` |
| 2 | Las dos acciones, una tarjeta cada una | `Atrás` · `Terminar` |

**El rechazo es por acción, y el recorrido entero se puede omitir.** Cada
tarjeta de la segunda pantalla lleva su «Ahora no» en secundario al lado del
botón, y no hay primario desactivado con una línea que diga qué falta. Además,
la bienvenida lleva `Omitir configuración`, fantasma y a la izquierda de `Continuar`:
rFirma firma documentos sin sedes, y a quien sólo quiere eso no se le hace
recorrer una configuración que no necesita. Omitir cuenta como haber visto el
asistente; las dos acciones siguen en el panel de estado.

## Los textos

Son el entregable del ticket, y van **literales**. El marcador `%{version}` es
un parámetro, no texto.

### Pantalla 1 · Bienvenida

> **Configurar rFirma**
>
> rFirma es una aplicación compatible con AutoFirma %{version}. Para poder
> usarla con sedes electrónicas necesitamos configurar tu equipo.

Y, en tarjeta:

> **Proyecto independiente**
>
> rFirma no está relacionada con AutoFirma ni con la Administración General del
> Estado, que publican el cliente oficial, ni cuenta con su respaldo. Si
> necesitas la aplicación oficial, descárgala de su web.

Y, en otra tarjeta con la forma de las de la pantalla 2:

> **Idioma**
>
> Elige el idioma de rFirma. Puedes cambiarlo más adelante en Preferencias.
>
> [ Español ▾ ]

Pie: `Omitir configuración` · `Continuar`.

### Pantalla 2 · El certificado de rFirma

> **El certificado de rFirma**
>
> Cuando firmas en una sede electrónica, tu navegador tiene que conectarse a
> rFirma. Para que esa conexión sea segura necesitamos instalar un certificado
> propio.

Acciones: `Instalar` (primario) · `Ahora no` (secundario).

Resultados, en el mismo nodo:

- Trabajando: **Instalando…**
- Bien: **Instalado en tus navegadores.** más, siempre, la línea *Si tienes
  alguno abierto, reinícialo para que lo reconozca.*
- Mal: **No se ha podido instalar en todas partes.** más la lista con marca por
  destino —✓ Firefox, ✓ Chrome y Chromium, ✗ Otros navegadores— y el
  botón `Reintentar` en secundario.

### Pantalla 2 · Usar rFirma por defecto

> **Usar rFirma por defecto**
>
> Ahora mismo las sedes electrónicas abren AutoFirma.

Acciones: `Que abran rFirma` (primario) · `Ahora no` (secundario). Hecho:
**Ahora abren rFirma.**

Pie de la pantalla 2: `Atrás` · `Terminar`.

## Estados

| Estado | Artboard | Acción principal |
| ------ | -------- | ---------------- |
| Bienvenida | `PrimerArranque` · `momento = 1 · bienvenida` | `Continuar` |
| Las dos acciones, sin hacer nada | `2 · acciones · sin hacer nada` | `Instalar`, `Que abran rFirma` |
| Instalando el certificado | `2 · certificado · instalando` | ninguna; la tarjeta no ofrece botón |
| Certificado instalado | `2 · certificado · instalado` | `Que abran rFirma` |
| El certificado ha fallado | `2 · certificado · ha fallado` | `Reintentar`, en secundario |
| Todo hecho | `2 · acciones · todo hecho` | `Terminar` |

Los dos pasos son **independientes**: cualquiera de los dos puede estar hecho,
declinado o fallado sin que el otro se entere.

**El fallo nunca atrapa.** Un paso fallado deja seguir y el pie no se bloquea:
impedir avanzar por algo que la persona no puede arreglar sería un callejón sin
salida, y la reparación ya tiene sitio propio en el panel de estado.

**WCAG 2.2 AA**, los dos criterios que esta pantalla incumpliría por omisión
([#654](https://github.com/sgomez/rfirma/issues/654)):

- **4.1.3 Status Messages.** El salto de «Instalando…» a un resultado se anuncia
  con `role="status"` sobre el bloque de resultado —el mismo nodo cambia de
  contenido, no aparece uno nuevo— y **el foco no se mueve**: quien acaba de
  pulsar sigue donde estaba.
- **2.4.11 Focus Not Obscured.** El pie no flota; es hermano de la zona
  desplazable.

## Componentes y tokens

Clases: `.rf-root`, `.rf-row`, `.rf-stack`, `.rf-gap-xs|sm`, `.rf-card`,
`.rf-heading`, `.rf-title`, `.rf-body`, `.rf-prose`, `.rf-hint`,
`.rf-text-muted`, `.rf-btn` con `--primary` y `--secondary`.

Tokens: `--rf-surface`, `--rf-text`, `--rf-text-muted`, `--rf-border-subtle`,
`--rf-primary`, `--rf-radius-md|pill`, `--rf-focus-ring`,
`--rf-space-xs|sm|md|lg`. Ni un color ni una sombra literales.

El indicador de paso y las marcas ✓ / ✗ de la lista de destinos se maquetan con
tokens; ninguno de los dos es un componente del sistema de diseño.

## Decisiones

Validado el **17/09/2026** en el canvas
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
artboard `PrimerArranque` de la página «Recorrido de firma», cuya anotación
guarda el porqué de cada punto. La copia legible sin cuenta está en
[`docs/design/artboards/`](artboards/README.md).

**Sin tira de pestañas.** Al pasar la ventana principal a pestañas de
documentos (25/09/2026), esta pantalla tomó su cabecera de 52 px pero no la
tira: una tira vacía, o con solo el «+», invitaría a abrir un PDF antes de
terminar la configuración.

**Dos pantallas, y no tres ni una.** Se descartaron las otras dos estructuras
que se dibujaron:

- **Tres pantallas encadenadas** —bienvenida, certificado, protocolo— alargan a
  tres pasos lo que son dos decisiones.
- **Una sola página con los tres bloques apilados**, además, **no cabe**: medía
  unos 770 px contra los 648 px del hueco, así que se desplazaba. Una sola
  página se lee de un vistazo sólo si cabe de un vistazo.

**El deslinde reutiliza la clave i18n `independence`**, que ya existe y ya está
traducida —`gl` incluida—. No se redacta una variante: es **la misma cadena** que
enseña [«Acerca de»](acerca-de.md). Al transcribir, la tarjeta separa el
«Proyecto independiente» de cabeza como título y el resto como cuerpo; la cadena
no se parte ni se reescribe por eso.

**El idioma se elige en la primera pantalla** porque es la única que se lee
entera antes de decidir nada: quien no entiende el idioma del sistema no puede
llegar hasta Preferencias para cambiarlo. El desplegable arranca en el idioma
resuelto ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)), es el mismo
`Select` de Preferencias con su rótulo, y el cambio repinta el asistente al
momento.

**La versión de AutoFirma va como parámetro** (`%{version}`), no escrita en la
frase: si no, cada actualización del original obligaría a rehacer la traducción
en los cinco idiomas del
[ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-cinco-idiomas.md).

**`afirma://` no se nombra en ninguna parte de la interfaz.** Sigue siendo el
mecanismo por debajo —[ADR-0005](../adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md)—,
pero quien usa esto no es técnico y no tiene por qué saber qué es un esquema de
protocolo. La pantalla habla de **qué programa abren las sedes**: «Ahora mismo
las sedes electrónicas abren AutoFirma» → «Que abran rFirma» → «Ahora abren
rFirma».

**Se dice «certificado propio», no «autofirmado» ni «certificado» a secas.**
«Autofirmado» es la palabra de las pantallas rojas de advertencia del navegador,
y arrastra su alarma a algo que aquí es correcto; «certificado» a secas choca con
el certificado de firma de la persona, que es lo único que ese nombre significa
en el resto de la aplicación. Por la misma razón el certificado local se cuenta
por **lo que consigue** —que la conexión del navegador con rFirma sea segura— y
no por lo que es, una CA en los almacenes NSS.

**El canal cifrado es entre el navegador y rFirma**, los dos en el mismo
ordenador. No es un canal con la sede, y el texto no lo insinúa: «tu navegador
tiene que conectarse a rFirma».

**La línea de reiniciar el navegador sale siempre**, sin detectar nada. Lo midió
el [#653](https://github.com/sgomez/rfirma/issues/653): la escritura del
certificado **nunca falla ni se pierde** con el navegador abierto —tampoco desde
dentro del sandbox del flatpak— e instalar se ve en caliente **casi siempre**.
Quien caiga en el resto creería que no ha funcionado, así que el aviso acompaña
al éxito sin excepción, en una línea.

**No hay aviso previo de cerrar los navegadores, ni pantalla de «reinicia el
navegador»**, y no es un olvido: es la otra cara de la misma medida. Lo que no
se ve hasta reiniciar es la **retirada** del certificado, y la retirada no es
este ticket.

**El fallo se cuenta con una lista con marca por destino**, no en prosa. La
señal es agregada ([#655](https://github.com/sgomez/rfirma/issues/655)), así que
un «algunos» se dice como «algunos»; enumerar en una frase qué entró y qué no
obliga a releerla dos veces.

**La HIG no respalda nada de esto, y está asumido**
([#654](https://github.com/sgomez/rfirma/issues/654)): no tiene patrón de
asistente —ni de bienvenida ni de primer arranque— y `GtkAssistant` está
obsoleto desde GTK 4.10. Sirve para acotar —«minimize the number of steps»— no
para justificar.

## Lo que queda abierto

Está aquí porque no se ha decidido, no porque se haya olvidado.

**Tres botones primarios en la pantalla 2** —`Instalar`, `Que abran rFirma` y
`Terminar`—. Está dibujado así a propósito: cada tarjeta ofrece su acción y el
pie navega. Pero con los dos bloques sin hacer, el ojo no tiene dónde caer
primero. La salida, si se toma, es **degradar a secundario los dos botones de
tarjeta** y dejar el primario sólo en el pie; no toca ningún texto.

**El aviso del permiso de red local se ha sacado del asistente y se queda sin
sitio.** Contaba una escena futura, a nombre de la sede y no de rFirma, y en la
configuración no hay nada que hacer con él. Le toca aparecer en la **primera
firma**, que es cuando ocurre, y esa pantalla no está diseñada.

**«Ahora mismo las sedes electrónicas abren AutoFirma» da por supuesto que
AutoFirma está instalado.** Si no lo está, la frase es falsa. No hay todavía
redacción alternativa para ese caso.

**Qué se recuerda entre sesiones cuando el asistente queda a medias.**
`trustNoticeSeen` es un único booleano y se queda corto para **dos acciones
independientes** que pueden quedar declinadas, fallidas o a medias. Es una
enmienda al [ADR-0010](../adr/0010-memoria-entre-sesiones.md), y la decide el
ticket, no esta ficha.

**Desde dónde se vuelve a ver el asistente**, sin decidir.

**Cadenas nuevas para `rfirma-app/po/messages.pot`**: todas las de esta ficha
menos `independence`, que ya existe y se recicla.
