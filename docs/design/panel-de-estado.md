# Panel de estado

La pantalla donde vive la verdad de la instalación: qué ha quedado montado en
este equipo, si funciona, y qué hacer con lo que no. Se abre desde la primera
entrada del menú de la [cabecera](cabecera.md), en cualquier momento y fuera de
todo recorrido de firma.

## Casos de uso que la usan

- **El panel de estado y el menú de la cabecera**
  ([#659](https://github.com/sgomez/rfirma/issues/659), mapa
  [#652](https://github.com/sgomez/rfirma/issues/652) «La instalación se explica
  sola») — de principio a fin.
- **La retirada desde dentro**
  ([#660](https://github.com/sgomez/rfirma/issues/660), el mismo mapa) — la fila
  del certificado, cuando lo que toca hacer con él es quitarlo: el diálogo de
  [retirar el certificado](retirar-certificado.md) se dispara desde aquí.

No forma parte de ningún recorrido: no se llega aquí firmando. Es el sitio al
que se vuelve cuando algo que el [primer arranque](primer-arranque.md) dejó a
medias hay que rematar, o cuando se quiere saber si esto está en orden.

## Qué resuelve

La regla que lo separa de [Preferencias](preferencias.md) es de reparto: **el
panel es donde se actúa sobre lo que la máquina informa; Preferencias, sobre
cómo se comporta la aplicación**. Las tres acciones caben en esa redacción, y
cada una cuelga de la señal que la pide: una **reparación** donde la señal está
mal, una **elección declarada** —qué programa firma— donde hay más de un
candidato, y una **retirada** de lo que rFirma escribió fuera de su territorio,
que es justo lo que la fila del certificado informa.

## Estructura

**Es la ventana de 1180 × 700 px, no un diálogo sobre ella.** La
[cabecera](cabecera.md) del
[ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md) se queda intacta
arriba, con su insignia de documento; el panel ocupa todo lo que hay debajo.

Tres regiones:

1. **Cabecera**, 56 px, sin tocar.
2. **Cuerpo**: una fila de título con `Estado de rFirma` a la izquierda y
   `Volver a comprobar` a la derecha, y bajo ella la **tabla**, que es la zona
   que se desplaza.
3. **Pie**, con raya superior sobre `--rf-surface` y `Cerrar` a la derecha.
   **No flota sobre el contenido**: es hermano de la zona desplazable, así que
   no puede tapar un control enfocado.

**No hay franja de resumen arriba.** Cada fila ya trae su veredicto, incluso
mientras se actualiza, así que una línea que resuma las cuatro es una línea más
que leer para saber lo mismo — y obligaría a decidir qué la pone en rojo, que es
una segunda fuente de verdad sobre lo que la tabla ya dice. Arriba sólo está el
título y el botón de remedir.

### La tabla

Cuatro columnas fijas, una fila por señal:

| Columna | Ancho | Qué lleva |
| ------- | ----- | --------- |
| Señal | 190 px | El nombre, en `.rf-prose` a peso 600 |
| Valor | flexible | El dato: texto pelado o desplegable |
| Veredicto | 130 px | Icono de 16 px más la palabra |
| Acción | 160 px | Un botón secundario, o lo que la reparación esté haciendo |

La cabecera de la tabla va en `.rf-label` en versalitas con `letter-spacing:
.6px` y se separa del cuerpo con `--rf-border-strong`; las filas entre sí, con
`--rf-border-subtle`.

Bajo la fila, y **sangrado a 206 px** —los 190 de la columna «Señal» más los 16
de separación—, cuelga lo que la fila necesite: la pista de una línea, o el
desplegable `ver almacenes` con su lista.

### Geometría

- Fila de título: `--rf-space-sm` de relleno vertical y `--rf-space-md`
  lateral, con el rótulo en `.rf-title` bajado a 17 px.
- Cada fila de señal: 10 px de relleno vertical y 36 px de alto mínimo en su
  línea principal. Ese alto lo fija la celda de valor, que lo reserva **haya o
  no desplegable**: así la fila no salta al cambiar de caso.
- Botones de la columna «Acción» y el de remedir: secundarios de 32 px de alto,
  `--rf-space-xs` de relleno lateral y 12 px de cuerpo. El `Cerrar` del pie va a
  36 px, que es el tamaño normal.
- El desplegable de valor mide 220 px de ancho mínimo y 36 px de alto, con
  `--rf-border-strong` y el galón de 16 px a la derecha.
- `ver almacenes` es un botón fantasma con un galón de 14 px que gira 90° al
  abrirse. La lista va debajo, con la marca en una columna de 12 px, el nombre
  del almacén en 230 px y la nota en `--rf-text-muted`.
- Pie: 12 px de relleno vertical, `--rf-space-md` lateral.

## Las cuatro señales

| Señal | Valor | Acción |
| ----- | ----- | ------ |
| Versión | `0.4.1` · `0.4.1 → 0.5.0` | `Actualizar` |
| Firma en sedes | `AutoFirma` · `rFirma` · `Sin configurar` · `No se puede consultar` | `Usar rFirma`, y un desplegable cuando hay dónde elegir |
| Certificado de rFirma | `2 de 3 almacenes` | `Instalar` o `Retirar…` según el veredicto, y `ver almacenes` |
| Tus certificados | `Ninguno` · `3 almacenes` | `Cómo instalar`, y `ver almacenes` |

**La redacción es telegráfica: etiqueta y valor, ni una frase dentro de una
celda.** Un panel de estado se mira, no se lee: `0.4.1 → 0.5.0` dice lo mismo
que «hay una versión nueva disponible, la 0.5.0» y se ve sin leerlo.

**La única prosa de la pantalla es una línea**, la pista `Firefox lo pregunta la
primera vez.`, debajo de la fila de la firma en sedes y **fuera** de su celda.
Su hueco se reserva siempre, así que ponerla o quitarla no mueve la tabla.

**`Certificado de rFirma` y `Tus certificados` se llaman así para no
confundirse.** Son dos cosas distintas que antes decían las dos «almacenes»: el
primero es el certificado propio que rFirma instala para que el navegador se fíe
de ella —el que instala el [primer arranque](primer-arranque.md)—; el segundo
son los tuyos, con los que firmas. Las dos llevan `ver almacenes`, y la lista
cuenta cosas distintas: dónde ha entrado la CA —Firefox, Chrome y Chromium,
Almacén del sistema, con ✓ o ✗ y el motivo del fallo al lado— frente a cuántos
certificados hay en cada almacén tuyo.

**La segunda fila se llama `Firma en sedes`, no `Aplicación de firma`.** Con las
dos filas acopladas, el nombre viejo se leía como «sin el certificado de rFirma
esto no firma», y eso es falso: arrastrar un PDF a la ventana y firmarlo no toca
la CA local en ningún punto. La CA sostiene el canal cifrado entre el navegador y
rFirma, así que lo único que se cae sin certificado es la firma que **empieza en
una sede**. El nombre nuevo hace verdadera la exclusión del desplegable y sigue
sin nombrar `afirma://`: nombra dónde empieza la firma, no el mecanismo.

**Y la fila no se parte en dos.** Una señal aparte para la firma de escritorio no
tendría nada que elegir ni nada que reparar, y una casilla que siempre dice
«Correcto» y no puede ponerse en rojo es lo que hizo caer a «Canal de
distribución». Cambia la etiqueta y nada más: los mismos valores, el mismo
desplegable, el mismo `Usar rFirma` y la misma pista de Firefox.

### Los cinco veredictos

Se distinguen por **silueta de icono, palabra y peso, nunca por color**: la
paleta es monocroma ([sistema de diseño](design-system.md), sección 2) y aquí no
hay verde ni rojo que gastar.

| Veredicto | Silueta | Peso |
| --------- | ------- | ---- |
| Correcto | Círculo con marca de comprobación | `--rf-text-muted` |
| Atención | Triángulo con admiración | 700 sobre `--rf-text` |
| Incorrecto | Círculo con aspa | 700 sobre `--rf-text` |
| No aplica | Círculo de trazo discontinuo con una raya | `--rf-text-muted` |
| Comprobando | Arco de tres cuartos | `--rf-text-muted` |

**«No aplica» y «Comprobando» están igual de apagados, y lo que los separa es la
silueta.** Ninguno de los dos es una media tinta de «Correcto»: no saber todavía
y no haber nada que saber son dos cosas, y ninguna es estar bien.

El triángulo de «Atención» es **el mismo `path`** que marca «Estado de rFirma»
en el menú de la [cabecera](cabecera.md). Dos dibujos distintos para lo mismo
serían dos vocabularios. Mismo dibujo, **regla distinta**: aquí lo pone el
veredicto de la fila, y allí solo lo encienden las averías que rFirma puede y
debe arreglar, que la [cabecera](cabecera.md) enumera.

### La fila de la firma en sedes

Cinco casos, y la fila **no cambia de alto en ninguno**:

| Caso | Valor | Veredicto | Qué ofrece |
| ---- | ----- | --------- | ---------- |
| AutoFirma es la aplicación | `AutoFirma` | Atención | `Usar rFirma`, y desplegable si el certificado está en algún almacén |
| rFirma, con AutoFirma instalado | `rFirma` | Correcto | Desplegable |
| rFirma, sin AutoFirma | `rFirma` | Correcto | Texto pelado |
| Sin configurar | `Sin configurar` | Atención | Desplegable y `Usar rFirma` |
| No se puede consultar | `No se puede consultar` | No aplica | Nada |

**El desplegable sólo aparece si hay dónde elegir, y hay dónde elegir cuando
queda algún candidato distinto del valor actual.** Con `rFirma` puesto y
AutoFirma sin instalar no queda ninguno, así que el valor va en texto pelado: un
desplegable cuya única entrada es lo que ya pone es un control que miente sobre
lo que se puede hacer con él. `Sin configurar` lo lleva siempre, porque no es
ninguno de los candidatos y cualquiera de ellos es una elección.

**Ese desplegable es el único sitio donde se elige el programa**
([#661](https://github.com/sgomez/rfirma/issues/661)).
[Preferencias](preferencias.md) tenía una copia suya, con la misma pista de
Firefox, y ya no: elegir y ver el veredicto son el mismo gesto, y partirlos en
dos pantallas obligaba a ir a mirar a una para entender la otra. Dos controles
para un mismo ajuste son además dos sitios donde mirar cuando no cuadra.

**`AutoFirma es la aplicación` dice «Atención» aquí, pero no enciende el
triángulo del menú de la [cabecera](cabecera.md).** Que las sedes abran
AutoFirma es una elección legítima, no una avería: la fila lo cuenta a quien
entra a mirar, y no sale a buscar a nadie. El veredicto de la fila y el disparo
del triángulo no son la misma regla — la fila informa, el triángulo llama.

**`No se puede consultar` es el sandbox del flatpak**, donde los manejadores
registrados no se pueden leer. No hay nada que configurar ni nada que reparar,
así que la casilla se apaga con «No aplica» y se queda sin botón, sin desplegable
y sin la pista de Firefox, que ahí no diría nada. **La fila no desaparece**: una
fila que a veces está obliga a reaprender la pantalla cada vez que se abre.

### La fila del certificado: `Instalar` o `Retirar…`

El botón lo fija el veredicto, y **nunca están los dos**: `Instalar` mientras
falte algún almacén, `Retirar…` cuando está en los tres. En la columna de acción
cabe una sola acción, y ofrecer retirar lo que aún no está entero es ofrecer dos
cosas para el mismo hueco. Los tres puntos dicen que abre un diálogo, como en el
resto de la interfaz: el de
[retirar el certificado](retirar-certificado.md). `ver almacenes` se queda debajo
con lo puesto y lo que falta, que es lo que hace falta para decidir entre una
cosa y la otra.

### Las dos señales están acopladas

La restricción es una sola: **sin certificado instalado, rFirma no puede firmar
en sedes.** Es una restricción sobre los dos gestos del panel, no un invariante
del sistema: si el certificado se va solo —caduca, o alguien lo borra a mano—,
rFirma sigue siendo quien firma en sedes y la fila dice «Incorrecto» con
`Instalar`. De ahí sale todo lo demás.

**El desplegable no ofrece `rFirma` si el certificado no está en ningún
almacén.** Un desplegable que ofrece lo imposible convierte una elección en un
error diferido.

**`Usar rFirma` instala también el certificado, y no cambia de rótulo.** El botón
promete un resultado —que firme rFirma—, no una lista de pasos; el procedimiento
no cabe en la columna de acción ni le importa a quien pulsa. Quien lo enumera es
el velo que confirma el gesto, antes de hacer las dos escrituras.

**Si firma AutoFirma, el certificado pasa a «No aplica» conservando su valor.**
La CA local existe para que el navegador se fíe de rFirma; si las sedes abren
AutoFirma, no hace falta. La fila sigue diciendo `2 de 3 almacenes`: apagar el
veredicto no es borrar el dato, y ese dato es justo el que decide si queda algo
que retirar.

**Y ahí «No aplica» sí lleva acción.** Rompe a propósito la regla de que no la
lleva, porque esa regla era una casualidad de los casos que había y no una ley:
«No aplica» quiere decir que no hace falta, no que no haya nada que hacer, y es
justo cuando el certificado ha dejado de hacer falta cuando tiene sentido
quitarlo de en medio. Con `0 de 3 almacenes` se queda sin botón, que ahí sí no
queda nada que hacer.

**`Sin configurar` y `No se puede consultar` no disparan «No aplica».** Sin nada
configurado la instalación está a medias y el certificado sigue haciendo falta;
dentro del contenedor no se sabe quién firma, y de un desconocido no se deduce
que algo sobre. Lo enciende **un programa declarado que no es rFirma**.

**«Comprobando» e `Instalando…` no se acoplan.** Mientras se mide no se sabe qué
hay, y mientras se repara la fila ya está diciendo algo más urgente que si hace
falta o no.

## Estados

Dos palancas, **independientes a propósito**: sin resumen global no hay nada que
se pueda contradecir entre ellas.

**Momento del panel**, cinco:

| Momento | Qué se ve |
| ------- | --------- |
| algo que reparar | La CA a medias y ningún certificado propio: dos «Atención» con su botón |
| todo correcto | Las cuatro en «Correcto», con los almacenes propios desplegados |
| a medio medir | Versión y certificado en «Comprobando», con el valor en `—` |
| reparando | La CA con `Instalando…` en su celda de acción, y `Volver a comprobar` apagado |
| la reparación falla | La CA en «Incorrecto», `0 de 3 almacenes`, con el motivo por almacén |

**Firma en sedes**, los cinco casos de la tabla de arriba.

**La celda de acción es donde una señal cuenta lo que está haciendo**, y hay dos
cosas que contar: `Instalando…` y `Retirando…`, las dos con el arco, en el mismo
sitio, dentro del mismo `role="status"` y con `Volver a comprobar` apagado
mientras duran. La retirada la cuenta además su diálogo, pero la verdad vive
aquí: ver [retirar el certificado](retirar-certificado.md).

**La pantalla en calma es `todo correcto` × `rFirma, sin AutoFirma`**: cuatro
veredictos apagados y un solo control, `Cerrar`.

**Se mide al arrancar la aplicación** —es lo que decide si el triángulo del menú
de la [cabecera](cabecera.md) se enciende antes de que nadie abra nada— y al
abrir el panel. **El refresco es a mano**, con `Volver a comprobar`, más el
automático que sigue a una reparación: toda reparación remide. Nada se remide
solo: ni periódicamente ni al volver la ventana al frente. Una tabla que cambia
bajo el cursor mientras se lee obliga a comprobar dos veces lo que ya se había
leído, y medir cuando nadie mira es trabajar de más. Mientras una reparación
corre, el botón de remedir se apaga.

**La carga se pinta progresivamente**, señal a señal. La alternativa —esperar a
tenerlo todo— deja la pantalla en blanco por culpa de la comprobación más lenta;
así, «Comprobando» es un veredicto más de la misma tabla y las filas ya medidas
se leen desde el primer momento.

**WCAG 2.2 AA**, los criterios que esta pantalla incumpliría por omisión
([#654](https://github.com/sgomez/rfirma/issues/654)):

- **1.4.1 Use of Color.** Los cinco veredictos se distinguen sin matiz: silueta,
  palabra y peso.
- **4.1.3 Status Messages.** La celda de acción que pasa a `Instalando…` y de
  ahí al resultado lleva `role="status"`, y **el foco no se mueve**: quien acaba
  de pulsar sigue donde estaba.
- **2.4.11 Focus Not Obscured.** El pie no flota; es hermano de la zona
  desplazable, así que ningún control enfocado en la tabla puede quedar debajo.

## Componentes y tokens

Clases: `.rf-root`, `.rf-row`, `.rf-stack`, `.rf-gap-xs|sm`, `.rf-title`,
`.rf-label`, `.rf-prose`, `.rf-body`, `.rf-hint`, `.rf-text-muted`, `.rf-badge`,
`.rf-btn` con `--secondary` y `--ghost`.

Tokens: `--rf-bg`, `--rf-surface`, `--rf-text`, `--rf-text-muted`,
`--rf-border-subtle`, `--rf-border-strong`, `--rf-radius-md`,
`--rf-space-xs|sm|md`. Ni un color ni una sombra literales; el borde de la
cabecera de la tabla y el del desplegable son `--rf-border-strong` porque son
contorno de control y separador fuerte, no decoración.

El galón es un `<svg>` **en línea** de contorno, sobre lienzo `0 0 24 24`, como
las tres rayas del menú. Los cinco iconos de veredicto también van en línea,
pero **macizos**: un contorno de 16 px tiene arcos de un píxel que el
antialiasing convierte en gris, y el icono se ve deslavazado junto al resto de
la pantalla. No hay biblioteca de iconos: los trazados se copian de Heroicons
(ver [sistema de diseño](design-system.md)). El icono de la columna
«Veredicto» toma **el mismo color que su palabra**.

La tabla no es un componente del sistema de diseño: se maqueta con `.rf-row` y
anchos fijos.

## Decisiones

Validado el **17/09/2026** en el canvas
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
artboard `PanelEstado` de la página «Estado de rFirma», cuya anotación guarda el
porqué de cada punto. La copia legible sin cuenta está en
[`docs/design/artboards/`](artboards/README.md).

**Tabla, y no lista ni tarjetas.** Las tres se dibujaron. La lista repite el
nombre de la señal dentro de cada frase y deja el veredicto en sitios distintos
de cada renglón; las tarjetas convierten cuatro datos de una línea en cuatro
bloques con aire, y a la cuarta ya no se ven todas juntas. Con columnas fijas,
el veredicto de las cuatro cae en la misma vertical y se barre de un vistazo,
que es lo que se viene a hacer aquí.

**No es un diálogo, y por eso cumple el 2.4.11 por construcción.** Un modal con
una tabla desplazable dentro pide un pie flotante, y un pie flotante tapa lo
enfocado; hacerlo ventana con el pie hermano de la zona que se desplaza resuelve
el criterio sin z-index que ajustar. De paso, la cabecera se queda donde estaba y
no hace falta contar qué hay detrás del velo.

**Cuatro señales, no cinco: «Canal de distribución» se cayó.** Estaba en el
inventario ([#655](https://github.com/sgomez/rfirma/issues/655)), pero una
casilla que siempre dice «Correcto» y **no puede ponerse en rojo** no es una
señal: es decoración que ocupa una fila y entrena a no mirar la columna. El
canal sigue existiendo por debajo —es lo que `Actualizar` hace por dentro— y por
eso **tampoco hay variante de `.deb` o `.rpm`** en el artboard: sería idéntica
píxel a píxel.

**La firma en sedes lleva desplegable, no un botón que alterna.** Elegir
qué programa abren las sedes es una **elección declarada**, no una reparación
disfrazada: un botón que va y viene esconde cuántos candidatos hay y no deja
elegir un tercero. `Usar rFirma` se queda sólo donde de verdad hay algo que
arreglar, que es cuando rFirma no es la aplicación.

**`afirma://` no se nombra, ni «los enlaces de las sedes».** Es la misma regla
del [primer arranque](primer-arranque.md): quien firma no sabe qué es un esquema
de protocolo, sabe **qué programa firma**. Por eso la señal nombra dónde empieza
la firma —«Firma en sedes»— y su valor es un nombre de programa.

**Sin número ni contador en ninguna parte.** Ni en la franja de arriba, que no
existe, ni en el aviso del menú. Contar obligaría a decidir qué se cuenta y a
mantener esa cuenta en dos sitios; la verdad está en la tabla, y la tabla se
mira.

**El nombre de la ventana es «Estado de rFirma», no «Estado».** En esa misma
franja vive la insignia del documento, y «Estado» a secas se leería como estado
del documento.

**El acoplamiento y la retirada se validaron el mismo día y en este mismo
artboard**, que estrena la palanca «Desplegable de Firma en sedes». Esa palanca
existe para poder **ver** que `rFirma` no aparece sin su certificado: con el
desplegable cerrado, una lista de dos entradas y otra de una se dibujan igual, y
una variante en reposo no decide nada. El diálogo que abre `Retirar…` tiene ficha
propia, [retirar el certificado](retirar-certificado.md).

**La retirada se dispara aquí y no en Preferencias**, que es donde estaba
prevista. La acción tiene que colgar de la señal que informa de lo que se va a
deshacer, igual que cuelga `Instalar`: retirar no es un comportamiento que se
ajuste, es deshacer lo que el [primer arranque](primer-arranque.md) escribió, y
lo escrito es exactamente lo que esta fila cuenta.
