# Retirar el certificado

**La verdad del dibujo es el código y sus historias:**
`status/WithdrawCertificateView.stories.tsx` («Flujos/Estado/WithdrawCertificateView»),
una historia por tiempo y por desenlace. Esta ficha cuenta el flujo y el porqué;
los textos salen del catálogo (claves `status.withdrawal.*`) y no se copian.

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
territorio de rFirma**, que son dos cosas y solo dos:

1. Las **dos CA locales** —la vigente y la siguiente— del almacén de cada perfil
   de navegador, más los ficheros PEM de donde sale su huella.
2. El registro que hace que **las sedes abran rFirma**. Quitarlo devuelve el
   esquema a quien lo declare; no hay que elegir por nadie. En el flatpak este
   paso no existe, porque ahí rFirma nunca llegó a escribirlo.

**Lo que vive en las carpetas propias de rFirma no se toca, y por eso tampoco se
nombra**: la rúbrica, los certificados en fichero importados y la configuración
se quedan, como al desinstalar cualquier otra aplicación, y cada uno ya tiene su
retirada por partes en [Preferencias](preferencias.md).

**La aplicación no se cierra al retirar.** Nada reinstala la CA a tus espaldas:
solo la instala el arranque del proceso de sede, que solo ocurre si una sede abre
rFirma, y la retirada se lleva también el registro. Firmar un PDF arrastrándolo a
la ventana sigue funcionando.

## Estructura

Es un `Dialog` de rol `alertdialog` sobre el cuerpo del panel, de 420 px, con
título, cuerpo, el aviso del navegador solo en el desenlace y una fila de
botones alineada a la derecha que no cambia de sitio entre tiempos. Su vista es
`WithdrawCertificateView`, sin estado ni puertos; `WithdrawCertificateDialog`
lleva el momento y el informe del último intento y llama al puerto
(`withdrawRfirma`).

### La cabecera no se tapa

**El velo empieza bajo la cabecera, que se queda viva y sin atenuar**
(`WithdrawCertificateView.css`). La cabecera es permanente y el cuerpo es lo que
cambia: `Estado de rFirma` y `Preferencias` son vistas del cuerpo, y un modal que
nace dentro del cuerpo no puede tapar lo que no es suyo. La cabecera sigue
alcanzable durante la retirada, incluso mientras trabaja: irse no cancela nada,
porque el panel de detrás ya lo cuenta. **El velo es una comodidad, no la
fuente**: la verdad está en la tabla.

## Los cuatro tiempos

Son la misma pantalla-estado en cuatro momentos, no cuatro pantallas.

| Tiempo | Historia | Título | Botones |
| ------ | -------- | ------ | ------- |
| Pregunta | `Question` | `status.withdrawal.title.question` | `actions.cancel` · `status.withdrawal.confirm` |
| Trabajando | `Working` | `status.withdrawal.title.working` | `actions.close`, apagado |
| Resultado | `Done` | `status.withdrawal.title.done` | `actions.close` |
| Resultado con fallo | `Partial`, `HandlerFailed` | `status.withdrawal.title.partial` | `actions.close` · `actions.retry` |

### La pregunta enumera, no explica

Dos renglones, en lo que la persona reconoce (`status.withdrawal.body.certificate`
y `status.withdrawal.body.handler`). **`afirma://` no se nombra**, por la misma
regla del [primer arranque](primer-arranque.md) y del
[panel de estado](panel-de-estado.md): quien firma sabe qué programa abren las
sedes, no qué es un esquema de protocolo.

La única línea de prosa es la que quita el miedo, `status.withdrawal.hint`: sin
ella `Retirar` parece una puerta de un solo sentido, y no lo es.

**`Retirar` es el primario**, como `Borrar y apagar` en
[Preferencias](preferencias.md). La paleta es monocroma: lo destructivo se marca
con el peso y la posición, y `Cancelar` se queda en fantasma a su izquierda.

### Trabajando: avance almacén a almacén, y sin salida

Abrir la base NSS de cada perfil, encontrar las dos CA y borrarlas no es
instantáneo, así que la lista dice **por dónde va**, con el arco y
`status.withdrawal.waiting` en cada almacén. No se puede cancelar a media faena
—cortar entre dos escrituras deja un almacén a medias y otro sin tocar—, y eso lo
dice el `Cerrar` **apagado**, y Escape no hace nada: un botón desactivado ya
explica que todavía no se puede salir. No es una cárcel: la cabecera sigue ahí.

### El desenlace: la misma lista que cuando falla una instalación

Marca, nombre del navegador (`status.storeBrands.*`) y el motivo al lado del ✗,
como el desplegable del panel: dos formatos para la misma cuenta serían dos
vocabularios. La última línea es `Firma en sedes`, como se llama esa señal en el
panel, para que el desenlace se lea contra la tabla que hay detrás.

**El aviso del navegador (`status.withdrawal.restartBrowserNotice`) va en el
desenlace, no en la pregunta, y siempre.** Los navegadores abiertos siguen
confiando hasta que se reinician, y aunque no haya ninguno abierto ahora, la
sesión que lo estuviera antes ya se llevó la confianza en memoria. Va después
porque es una consecuencia de lo que acaba de pasar.

**Con fallo, `Reintentar` al lado de `Cerrar`, y el título cambia.** El motivo
típico es el perfil en uso, que se arregla cerrando el navegador y volviendo a
pulsar. `Reintentar` solo vuelve a tocar lo que falló, con el informe anterior
como referencia, y si la repetición lo arregla todo el velo pasa al desenlace
completo. El fallo nunca atrapa: `Cerrar` está siempre.

## Estados y flujo

**Al cerrarse el velo, el panel vuelve a medir**: la verdad sigue viviendo en la
tabla. Con la retirada hecha, `Firma en sedes` pasa a «Sin configurar» con
`Usar rFirma` y el certificado a «Incorrecto» con `Instalar`; con fallo, el
certificado queda «Atención» con lo que falte. Mientras el velo está delante, un
Escape no cierra además el panel.

## Componentes y tokens

No estrena nada: `Dialog`, `Button` (`primary`, `ghost`), `Row`, `Stack`, y las
clases `.rf-title`, `.rf-prose`, `.rf-body`, `.rf-hint`, `.rf-text-muted`. Tokens
`--rf-text`, `--rf-text-muted`, `--rf-space-xs|sm`; ni un color ni una sombra
literales. El ✓ y el ✗ son los iconos de la lista del panel; el arco es el de
«Comprobando».

## Decisiones

**El velo se dispara desde el panel, no desde Preferencias.** Retirar no es cómo
se comporta la aplicación, es deshacer una escritura, y lo escrito es exactamente
lo que la fila `Certificado de rFirma` informa. El botón cuelga de la señal que
lo cuenta, como cuelga `Instalar`.

**Un velo con cuatro tiempos, no cuatro diálogos.** Son momentos de la misma
pantalla-estado, y separarlos obligaría a leer cuatro sitios para saber qué ve la
persona de principio a fin.

**No se detecta si hay un navegador abierto.** Detectar no aporta: lo ya resuelto
en memoria no se invalida aunque no haya ninguno abierto. Advertir siempre cuesta
menos y no miente.

Validado el 17/09/2026 en Claude Design. El dibujo ya no se guarda en el
repositorio.
