# Panel de estado

**La verdad del dibujo es el código y sus historias:** `status/StatusView.stories.tsx`
(«Flujos/Estado/StatusView», la vista pura) y `status/StatusWindow.stories.tsx`
(«Pantallas/Estado/1 · Panel», la ventana con su puerto), con una historia por señal y veredicto, por caso de la
firma en sedes y por detalle plegado o desplegado. Esta ficha cuenta el flujo y
el porqué; los textos salen del catálogo (claves `status.*`) y no se copian.

La pantalla donde vive la verdad de la instalación: qué ha quedado montado en
este equipo, si funciona, y qué hacer con lo que no. Se abre desde la primera
entrada del menú de la [cabecera](cabecera.md), o desde su botón de aviso cuando
lo hay, en cualquier momento y fuera de todo recorrido de firma.

## Casos de uso que la usan

- **El panel de estado y el menú de la cabecera**
  ([#659](https://github.com/sgomez/rfirma/issues/659), mapa
  [#652](https://github.com/sgomez/rfirma/issues/652) «La instalación se explica
  sola») — de principio a fin.
- **La retirada desde dentro**
  ([#660](https://github.com/sgomez/rfirma/issues/660), el mismo mapa) — la fila
  del certificado, cuando lo que toca es quitarlo: el velo de
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

**Es una vista del cuerpo de la ventana, no un diálogo sobre ella.** La
[cabecera](cabecera.md) del
[ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md) se queda arriba en
su variante sin documentos: las pestañas son de documentos, y el panel no es de
ninguno. El botón de aviso no está, porque ya estás donde llevaría.

Tres regiones: la fila de título con `Volver a comprobar`, la tabla —que es la
zona que se desplaza— y un pie con `Cerrar`. **El pie no flota**: es hermano de
la zona desplazable, así que no puede tapar un control enfocado (WCAG 2.4.11), y
un test lo protege.

**No hay franja de resumen arriba.** Cada fila ya trae su veredicto, y una línea
que resumiera las cuatro sería una segunda fuente de verdad sobre lo que la
tabla ya dice, y obligaría a decidir qué la pone en rojo.

La tabla tiene cuatro columnas fijas —señal, valor, veredicto y acción— y **no
lleva cabecera de columnas**: las filas se leen solas. Bajo la fila, sangrado
bajo la columna del valor, cuelga el desplegable de su detalle o el aviso de
reinicio.

## Las cuatro señales

| Señal | Clave | Valor | Acción |
| ----- | ----- | ----- | ------ |
| Versión | `status.signals.version` | la versión, o `0.4.1 → 0.5.0` | `status.actions.update` |
| Aplicación predeterminada para firmar en sedes | `status.signals.siteSignature` | el programa, `status.values.siteSignature.*` | `status.actions.useRfirma`, y un desplegable si hay dónde elegir |
| Certificado de rFirma | `status.signals.localCaCertificate` | `status.values.localCaCertificate` | `status.actions.install` o `status.actions.withdraw`, y `status.detail.toggle` |
| Tus certificados | `status.signals.userCertificates` | `status.values.userCertificates.*` | `status.actions.howToInstall`, y `status.detail.where` |

Historias: `EverythingCorrect`, `SomethingToRepair` y `Checking` ponen las cuatro
a la vez; las demás, una señal cada una (`Version*`, `Sites*`, `Certificate*`,
`NoUserCertificates`, `SomeUserCertificates`).

**La redacción es telegráfica: etiqueta y valor, ni una frase dentro de una
celda.** Un panel de estado se mira, no se lee: `0.4.1 → 0.5.0` dice lo mismo
que «hay una versión nueva disponible» y se ve sin leerlo.

**«Certificado de rFirma» y «Tus certificados» se llaman así para no
confundirse.** El primero es el certificado propio que rFirma instala para que
el navegador se fíe de ella, el que instala el
[primer arranque](primer-arranque.md); el segundo son los tuyos, con los que
firmas. Las dos cuelgan una lista, y **no es la misma lista**: la de la CA dice a
qué navegadores ha entrado, con marca de acierto o de fallo; la tuya dice dónde
tienes certificados y cuántos en cada sitio, y ahí no se ha intentado nada, así
que no lleva marca.

**La lista dice «Instalado» o «No instalado», no «De confianza».** Lo que se
mide por debajo es la confianza, pero «No de confianza» junto al nombre de un
navegador se lee como una acusación y siembra justo la desconfianza que el panel
existe para disipar. ¿Está puesto o no? Eso contesta `status.detail.installed`, y
es lo que el botón de instalar promete cambiar.

**La CA se cuenta en navegadores, no en «almacenes».** Los sitios que rFirma
encuentra son bases NSS de un navegador, y la palabra que nombra el sitio en el
idioma de quien lee es «navegador». Los nombres de cada sitio salen de
`status.storeBrands.*`, uno solo que comparten la tabla, el asistente y la
retirada.

**La segunda fila se llama «Aplicación predeterminada para firmar en sedes», no
«Aplicación de firma».** Arrastrar un PDF a la ventana y firmarlo no toca la CA
local: lo único que se cae sin certificado es la firma que **empieza en una
sede**. El nombre dice qué aplicación atiende esa firma y dónde empieza, no el
mecanismo, y no nombra `afirma://`.

**La fila no se parte en dos.** Una señal aparte para la firma de escritorio no
tendría nada que elegir ni que reparar, y una casilla que siempre dice
«Correcto» y no puede ponerse en rojo es la que hizo caer a «Canal de
distribución».

### Los cinco veredictos

Se distinguen por **silueta de icono, palabra y peso, nunca por color**: la
paleta es monocroma ([sistema de diseño](design-system.md)). Claves
`status.verdicts.*`: correcto, atención, incorrecto, no aplica y comprobando.

«No aplica» y «Comprobando» están igual de apagados y los separa la silueta:
no saber todavía y no haber nada que saber son dos cosas, y ninguna es estar
bien. El triángulo de «Atención» es **el mismo dibujo** que el botón de aviso de
la [cabecera](cabecera.md), con regla distinta: aquí lo pone el veredicto de la
fila, y el botón solo aparece con las averías que rFirma puede arreglar.

### La fila de la firma en sedes

Cinco casos, cada uno con su historia: `SitesWithTwoCandidates` (AutoFirma es la
aplicación, con desplegable y `Usar rFirma`), `SitesHandledByRfirma` (texto
pelado), `SitesNotConfigured`, `SitesUnavailable` (con su variante
`SitesUnavailableDiagnosisExpanded`, el desplegable abierto) y, en
`EverythingCorrect`, rFirma ya elegida.

**El desplegable solo aparece si hay dónde elegir**, es decir, con dos o más
candidatos. Con rFirma puesta y AutoFirma sin instalar el valor va en texto
pelado: un desplegable cuya única entrada es lo que ya pone miente sobre lo que
se puede hacer. Es **el único sitio donde se elige el programa**
([#661](https://github.com/sgomez/rfirma/issues/661)); elegir y ver el veredicto
son el mismo gesto.

**«AutoFirma es la aplicación» dice Atención aquí, pero no enciende el botón de
aviso de la cabecera.** Que las sedes abran AutoFirma es una elección legítima,
no una avería: la fila informa, el botón llama. La regla del botón vive en
`hasMenuAttention` (`status/status.ts`).

**«Gestionada por tu escritorio» es el sandbox del flatpak**, donde los manejadores
registrados no se pueden leer: la fila no muestra veredicto ni botón de reparación,
solo un desplegable «Diagnosticar problemas» con las órdenes que se ejecutan fuera
del sandbox. Las órdenes llegan como detalle `handlerDiagnosis`, con el `.desktop`
del flatpak, y solo en ese canal: donde no se puede consultar por otro motivo, la
fila no ofrece órdenes `xdg-mime`. **La fila no desaparece**, porque una fila que a
veces está obliga a reaprender la pantalla.

**Que el escritorio pregunte qué aplicación usar no es un problema** y la fila no
lo explica: el desplegable solo cubre que se abra otra aplicación sin preguntar.

### Dónde y cuántos en «Tus certificados»

**El valor cuenta certificados, no sitios**, y lleva el sustantivo: la pregunta
que trae a mirar la fila es si hay con qué firmar. El sitio va en la lista, con
su cuenta a la derecha y **sin marca**: ✓ y ✗ dicen si algo salió bien, y aquí no
se ha intentado nada. El desplegable se llama `status.detail.where`, no «Ver
navegadores», porque en un sitio cabe una tarjeta. Historia:
`UserCertificatesDetailExpanded`.

**Dos perfiles del mismo navegador son una línea**: la lista es de sitios como
los nombra quien mira. **Con «Ninguno» no hay desplegable**: no es una lista
vacía, es que no hay lista, y la acción es `Cómo instalar`.

### La fila del certificado: instalar o retirar

El botón lo fija el veredicto, y **nunca están los dos**: `Instalar` mientras
falte algún navegador, `Retirar…` cuando está en todos. Los puntos suspensivos
dicen que abre un velo: el de [retirar el certificado](retirar-certificado.md).
Historias: `CertificateMissing`, `CertificateHalfInstalled`,
`CertificateInstalled`, `CertificateInstalledNeedsFirefoxRestart` (con
`status.notices.restartFirefox`) y `CertificateDetailExpanded`.

### Las dos señales están acopladas

La restricción es una: **sin certificado instalado, rFirma no puede firmar en
sedes**. De ahí sale que `Usar rFirma` instala también el certificado y no
cambia de rótulo —el botón promete un resultado, no una lista de pasos—, que al
elegir rFirma o AutoFirma las dos filas se vuelven a medir pasando por
«Comprobando», y que «Sin configurar» y «Gestionada por tu escritorio» no apagan el
certificado: de un desconocido no se deduce que algo sobre.

## Estados y flujo

Se mide **al abrir el panel** —y al arrancar la aplicación, para el aviso de la
cabecera— y después **a mano**, con `Volver a comprobar`, más la remedición que
sigue a una acción o al cerrarse el velo de la retirada. Nada se remide solo, ni
periódicamente ni al volver la ventana al frente: una tabla que cambia bajo el
cursor obliga a comprobar dos veces lo que ya se había leído. Mientras se
remide, el botón se apaga.

**La carga se pinta progresivamente**: primero las filas que `readStatus` trae,
y después se miden la CA y la versión, que nacen en «Comprobando» (historia
`Checking`). La alternativa, esperar a tenerlo todo, deja la pantalla en blanco
por culpa de la comprobación más lenta.

**Una acción deja la fila en «Comprobando» hasta que vuelve el resultado**: no
hay una celda de acción con un texto propio mientras trabaja. Escape y `Cerrar`
cierran el panel, salvo que el velo de la retirada esté delante y atienda su
propio Escape.

**La pantalla en calma es todo correcto con rFirma sola**: veredictos apagados y
un solo control, `Cerrar`.

**WCAG 2.2 AA**
([#654](https://github.com/sgomez/rfirma/issues/654)): los veredictos se
distinguen sin matiz (1.4.1); cada fila es un `role="status"` y el foco no se
mueve tras una acción (4.1.3); el pie no flota (2.4.11).

## Componentes y tokens

Primitivos `Button` (`secondary` y `ghost`) y `Select`; clases `.rf-title`,
`.rf-prose`, `.rf-body`; maquetación propia en `StatusView.css`, porque la tabla
no es un primitivo del sistema de diseño. Tokens: `--rf-bg`, `--rf-surface`,
`--rf-text`, `--rf-text-muted`, `--rf-border-subtle`, `--rf-border-strong`,
`--rf-space-xs|sm|md`; ni un color ni una sombra literales. Los iconos de
veredicto son macizos y en línea, de Heroicons (ver
[sistema de diseño](design-system.md)), y toman el color de su palabra.

## Decisiones

**Tabla, y no lista ni tarjetas.** La lista repite el nombre de la señal en cada
frase y deja el veredicto en sitios distintos; las tarjetas convierten cuatro
datos de una línea en cuatro bloques con aire. Con columnas fijas, el veredicto
de las cuatro cae en la misma vertical y se barre de un vistazo.

**No es un diálogo, y por eso cumple el 2.4.11 por construcción.** Un modal con
una tabla desplazable pide un pie flotante, y un pie flotante tapa lo enfocado.

**Cuatro señales, no cinco: «Canal de distribución» se cayó.** Una casilla que
siempre dice «Correcto» y no puede ponerse en rojo no es una señal: es
decoración que entrena a no mirar la columna.

**La firma en sedes lleva desplegable, no un botón que alterna.** Elegir qué
programa abren las sedes es una elección declarada, no una reparación
disfrazada: un botón que va y viene esconde cuántos candidatos hay.

**`afirma://` no se nombra.** Quien firma no sabe qué es un esquema de
protocolo, sabe qué programa firma.

**Sin número ni contador en ninguna parte**, ni en una franja ni en el botón de
aviso: contar obligaría a mantener la cuenta en dos sitios.

**El nombre de la ventana es «Estado de rFirma», no «Estado»**, que se leería
como el estado del documento abierto.

**La retirada se dispara aquí y no en Preferencias.** La acción cuelga de la
señal que informa de lo que se va a deshacer, igual que `Instalar`: retirar no es
un comportamiento que se ajuste, es deshacer lo que el
[primer arranque](primer-arranque.md) escribió.

Validado el 17/09/2026 en Claude Design; el botón de aviso sustituyó al
triángulo del menú el 30/09/2026. El dibujo ya no se guarda en el repositorio.
