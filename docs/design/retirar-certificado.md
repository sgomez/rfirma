# Retirar el certificado

El velo que confirma, ejecuta y cuenta la retirada del certificado de rFirma de
los navegadores donde esté. Lo abre el botón `Retirar…` de la fila
`Certificado de rFirma` del [panel de estado](panel-de-estado.md), que es la
señal que informa de lo que se va a deshacer.

## Casos de uso que la usan

- **La retirada desde dentro**
  ([#660](https://github.com/sgomez/rfirma/issues/660), mapa
  [#652](https://github.com/sgomez/rfirma/issues/652) «La instalación se explica
  sola») — de principio a fin.

No forma parte de ningún recorrido de firma. Se llega desde el panel de estado y
se vuelve a él, que es donde la retirada se lee de verdad.

## Qué resuelve

Deshacer lo que el [primer arranque](primer-arranque.md) escribió **fuera del
territorio de rFirma**, que son dos cosas y sólo dos:

1. Las **dos CA locales** —la vigente y la siguiente— del almacén de cada perfil
   de navegador, más los ficheros PEM de donde sale su huella.
2. El registro que hace que **las sedes abran rFirma**. Quitarlo devuelve el
   esquema a quien lo declare; no hay que elegir por nadie. En el flatpak este
   paso no existe, porque ahí rFirma nunca llegó a escribirlo.

**Lo que vive en las carpetas propias de rFirma no se toca, y por eso tampoco se
nombra**: la rúbrica, los certificados en fichero importados y la configuración
se quedan, como se quedan al desinstalar cualquier otra aplicación de Linux, y
cada uno ya tiene su retirada por partes en [Preferencias](preferencias.md).
Enumerarlos aquí para decir que se quedan abre una duda que nadie tenía.

**La aplicación no se cierra al retirar.** Nada reinstala la CA a tus espaldas:
sólo la instala el arranque del proceso de sede, que sólo ocurre si una sede abre
rFirma, y eso ya no pasa porque la retirada se lleva también el registro. Firmar
un PDF arrastrándolo a la ventana sigue funcionando: el proceso de escritorio
nunca ha tocado la CA.

## Estructura

**El velo empieza a los 56 px, bajo la cabecera, que se queda viva y sin
atenuar.** Detrás no hay una reconstrucción parecida del panel: es el panel, con
su misma cabecera, su misma fila de título, su misma tabla de cuatro columnas y
su mismo pie.

`.rf-scrim` sobre el cuerpo y `.rf-dialog` de 420 px centrado, con:

1. **Título**, en `.rf-title` a 16 px, distinto en cada tiempo.
2. **Cuerpo**, que es la enumeración de la pregunta o la lista por almacén.
3. **Aviso del navegador**, con el triángulo de «Atención» de 16 px, sólo en el
   desenlace.
4. **Fila de botones** alineada a la derecha, que no cambia de sitio entre los
   cuatro tiempos.

### La cabecera no se tapa

Y el porqué no es un parecido con otra pantalla, es un modelo: **la cabecera es
permanente y el cuerpo es lo que cambia**. `Estado de rFirma` y `Preferencias`
son **vistas del cuerpo**, no diálogos sobre la ventana, y un modal que nace
dentro del cuerpo no puede tapar lo que no es suyo.

La consecuencia es la que importa: **la cabecera sigue alcanzable durante la
retirada, incluso mientras trabaja**. Irse no cancela nada y no se pierde nada,
porque el panel de detrás ya está contando lo mismo en su celda de acción. **El
diálogo es una comodidad, no la fuente**: la verdad está en la tabla.

## Los cuatro tiempos

Son la misma pantalla-estado en cuatro momentos, no cuatro pantallas.

| Tiempo | Título | Cuerpo | Botones |
| ------ | ------ | ------ | ------- |
| Pregunta | `Retirar el certificado de rFirma` | Los dos renglones de lo que se lleva, y la pista de que se puede volver a instalar | `Cancelar` · `Retirar` |
| Trabajando | `Retirando…` | La lista, almacén a almacén, con el que va en marcha marcado con el arco y los siguientes `En espera` | `Cerrar`, apagado |
| Resultado | `Certificado retirado` | La lista con ✓ y el aviso del navegador | `Cerrar` |
| Resultado con fallo | `Retirado a medias` | La lista con el ✗ y su motivo, y el aviso del navegador | `Cerrar` · `Reintentar` |

### La pregunta enumera, no explica

Dos renglones, en lo que la persona reconoce: «El certificado de rFirma, de los
navegadores donde esté» y «Que las sedes abran rFirma». **`afirma://` no se
nombra**, que es la misma regla del [primer arranque](primer-arranque.md) y del
[panel de estado](panel-de-estado.md): quien firma no sabe qué es un esquema de
protocolo, sabe qué programa abren las sedes.

La única línea de prosa del diálogo es la que quita el miedo: «Se puede volver a
instalar desde este panel». Sin ella, `Retirar` parece una puerta de un solo
sentido, y no lo es.

**`Retirar` es el primario**, como `Borrar y apagar` en
[Preferencias](preferencias.md). La paleta es monocroma y no hay rojo que gastar:
lo destructivo se marca con el peso y con la posición, y `Cancelar` se queda en
fantasma a su izquierda.

### Trabajando: avance almacén a almacén, y sin salida

Abrir la base NSS de cada perfil, encontrar las dos CA y borrarlas no es
instantáneo, así que un giro indeterminado sólo diría «espera»; la lista dice
**por dónde va**. No se puede cancelar a media faena —cortar entre dos escrituras
deja un almacén a medias y otro sin tocar—, y eso lo dice el `Cerrar` **apagado**,
no una frase: un botón desactivado ya explica que todavía no se puede salir, y la
fila de botones no cambia de sitio al llegar el desenlace.

Ese `Cerrar` apagado **no es una cárcel**: dice que ese diálogo todavía no tiene
nada que contar, no que la aplicación esté bloqueada. La cabecera sigue ahí.

### El desenlace: la misma lista que cuando falla una instalación

Marca, nombre del navegador y el motivo al lado del ✗ — literalmente el desplegable
`Ver navegadores` del panel, que sólo estrecha el nombre de 230 a 190 px porque
dentro de 420 px el motivo no cabría al lado. Dos formatos para la misma cuenta
serían dos vocabularios. La cuarta línea es `Firma en sedes`, que es como se llama
esa señal en el panel: así el desenlace se lee contra la tabla que hay detrás.

**El aviso del navegador va en el desenlace, no en la pregunta.** Los navegadores
abiertos **siguen confiando hasta que se reinician**: la borrada entra bien, pero
lo ya resuelto en memoria no se invalida, y por eso retirar es asimétrico
respecto a instalar, que sí se ve en caliente. **Se advierte siempre**, sin
condicionarlo a detectar si hay un navegador abierto: aunque no lo esté ahora, la
sesión que lo estuviera antes ya se llevó la confianza en memoria. Y va después
porque es una consecuencia de lo que acaba de pasar; ponerla antes sería una
condición más que sopesar para decidir.

**`Cerrar` cierra el diálogo, no la aplicación.** Es el mismo rótulo que el pie
del panel, que también cierra lo suyo.

**Con fallo, `Reintentar` al lado de `Cerrar`, y el título cambia.** El motivo
típico es el perfil en uso, que se arregla cerrando el navegador y volviendo a
pulsar; por eso reintentar es la acción sugerida y se queda a la derecha. El
fallo nunca atrapa, y aquí pesa más que en ningún otro sitio, porque el camino
alternativo —volver a entrar después de haber decidido irse— no lo recorre nadie.

## Estados

Los cuatro tiempos, y **el panel de detrás cambia con ellos**, con las reglas del
propio panel y sin inventar ninguna:

| Tiempo | Firma en sedes | Certificado de rFirma |
| ------ | -------------- | --------------------- |
| Pregunta | `rFirma`, Correcto | `3 de 3 navegadores`, Correcto, `Retirar…` |
| Trabajando | `rFirma`, Correcto | `3 de 3 navegadores`, con `Retirando…` en la celda de acción y `Volver a comprobar` apagado |
| Resultado | `Sin configurar`, Atención, `Usar rFirma` | `0 de 3 navegadores`, Incorrecto, `Instalar` |
| Resultado con fallo | `Sin configurar`, Atención, `Usar rFirma` | `1 de 3 navegadores`, Atención, `Instalar`, con el detalle abierto |

`Retirando…` ocupa **el mismo sitio y sigue el mismo patrón** que `Instalando…`:
la celda de acción, con el arco. El velo deja ver lo que la retirada está
cambiando.

## Componentes y tokens

No estrena nada. `.rf-scrim` y `.rf-dialog` son los del diálogo que confirma
apagar `Recordar mi actividad` en [Preferencias](preferencias.md), con su ancho
de 420 px sin tocar; el ✓ y el ✗ son los caracteres que ya usa `Ver navegadores`;
el arco es el `path` de «Comprobando» y el triángulo, el de «Atención».

Clases: `.rf-scrim`, `.rf-dialog`, `.rf-row`, `.rf-stack`, `.rf-gap-xs`,
`.rf-title`, `.rf-prose`, `.rf-body`, `.rf-hint`, `.rf-text-muted`, `.rf-btn` con
`--primary` y `--ghost`.

Tokens: `--rf-text`, `--rf-text-muted`, `--rf-space-xs|sm`. Ni un color ni una
sombra literales.

## Decisiones

Validado el **17/09/2026** en el canvas
[Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132),
artboard `RetirarCertificado` de la página «Estado de rFirma», cuya anotación
guarda el porqué de cada punto. La copia legible sin cuenta está en
[`docs/design/artboards/`](artboards/README.md).

**El artboard es `PanelEstado` con el velo encima, y así se redactó.** El fichero
es una copia del suyo y lo único que se le añade es el velo, con la tabla pintada
por la misma función. Dos dibujos de la misma pantalla se desincronizan en el
segundo cambio.

**Se dispara desde el panel, no desde Preferencias.** Estaba previsto en
Preferencias y se cambia: retirar no es cómo se comporta la aplicación, es
deshacer una escritura, y lo escrito es exactamente lo que la fila `Certificado
de rFirma` informa. El botón cuelga de la señal que lo cuenta, como cuelga
`Instalar`.

**Un velo con cuatro tiempos, no cuatro diálogos.** Son momentos de la misma
pantalla-estado, y separarlos obligaría a leer cuatro sitios para saber qué ve la
persona de principio a fin.

**No se detecta si hay un navegador abierto.** Detectar no aporta: lo ya resuelto
en memoria no se invalida aunque no haya ninguno abierto ahora mismo. Advertir
siempre cuesta menos y no miente.
