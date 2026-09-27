# Sin barra de menús: una sola barra con la identidad, los documentos y el menú

La primera versión de la interfaz dibujaba una barra de menús clásica
(`Archivo · Ver · Ayuda`) dentro de la ventana. Se retira.

Ninguno de los escritorios objetivo la usa hoy: GNOME la retiró de su guía de
estilo en favor de una cabecera con botón de menú, y Windows 11 no la emplea en
sus aplicaciones nuevas.

`rfirma` tiene **una sola barra permanente**, de 44 px, que es a la vez cabecera
y tira de pestañas. De izquierda a derecha: la identidad (`rFirma`), el **botón
partido** de abrir, las **pestañas** de los documentos abiertos y, al extremo
derecho, el botón de menú.

- **El botón partido** reúne abrir y reabrir en un mismo control con dos
  gestos: el segmento principal, «Abrir PDF…», abre el diálogo del sistema
  directamente (Ctrl+O); la flecha despliega «Abiertos recientemente». Sin
  recientes —actividad apagada o lista vacía— la flecha desaparece y queda un
  botón simple.
- **Las pestañas que no caben** se recogen en un «+N ▾» al final, con un menú de
  las ocultas; la activa ocupa siempre el último hueco visible.

La barra no lleva el estado del documento ni el certificado: el documento
firmado lo marca su pestaña con ✓, y el certificado lo dice el selector del
panel de firma. Una insignia de estado se descartó por eso: repetía la pestaña y
confundía «Estado de rFirma» con el estado del documento.

Las vistas que no son de ningún documento —Preferencias, el panel de estado, el
primer arranque— ocupan el cuerpo de la ventana, y **la barra se queda solo con
`rFirma` y el menú**: sin botón de abrir y sin pestañas, porque las pestañas son
de documentos y esas vistas no son de ninguno.

Los menús que se han eliminado no se han movido a otro sitio: **no hacían
falta**. Abrir un documento ya tiene el botón partido y la zona de soltar;
guardar tiene la caja «Guardar en» del pie del panel de firma; y paginación y
zoom viven en la barra flotante del visor, que es exactamente lo que un menú
*Ver* habría contenido.

**Criterio de pertenencia al menú:** el menú contiene lo que habla **de la
aplicación**; nunca lo que habla **del documento**. Abrir, guardar, paginar y
ampliar son del documento y ya tienen su sitio en el recorrido; estado, ajustes,
ayuda e identidad son de la aplicación y no tienen otro. Un tope numérico
inventado se incumple el día que estorba, y el criterio ya excluye lo que el
tope querría excluir. Cuando el grupo de aplicación crezca de verdad, la
respuesta no es una barra de menús sino una página más dentro de Preferencias o
de Estado.

## Considered Options

- **Cabecera de 52 px y, debajo, una tira de pestañas de 40 px.** Fue el diseño
  hasta el 27/09/2026. Dos franjas para una sola cosa —la parte de arriba de la
  ventana— se comían 92 px de alto; con la barra única son 44, y esos 48 px
  vuelven al visor, que es la región principal.
- **Un «+» al final de la tira que desplegaba abrir y los recientes juntos.**
  Abrir costaba dos clics —desplegar y elegir «Abrir un PDF…»— cuando es el
  gesto más frecuente, y no tenía atajo propio. El botón partido deja abrir a un
  clic o a Ctrl+O y aparta reabrir a la flecha, dentro del mismo control.
- **Flechas ‹ › para desplazar la tira cuando las pestañas no caben.**
  Desplazar obliga a buscar pasando de una en una; el menú «+N» enseña todas las
  ocultas de golpe y la activa no se pierde nunca.

## Consequences

- Sin barra de menús se pierden los aceleradores de teclado visibles. No hay
  todavía una lista de atajos decidida; cuando la haya, hará falta un sitio
  donde consultarlos. Ctrl+O es el primero, y lo anuncia el `title` de «Abrir
  PDF…».
- Las pestañas comparten fila con la identidad, el botón partido y el menú, así
  que caben menos que en una tira propia: el desborde a «+N» llega antes.
