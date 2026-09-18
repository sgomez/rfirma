# Sin barra de menús: una cabecera única

La primera versión de la interfaz dibujaba una barra de menús clásica
(`Archivo · Ver · Ayuda`) dentro de la ventana. Se retira.

Ninguno de los escritorios objetivo la usa hoy: GNOME la retiró de su guía de
estilo en favor de una cabecera con botón de menú, y Windows 11 no la emplea en
sus aplicaciones nuevas.

`rfirma` tiene **una sola cabecera permanente**: identidad a la izquierda,
estado del documento y un botón de menú a la derecha. Las vistas son del cuerpo
de la ventana.

Los menús que se han eliminado no se han movido a otro sitio: **no hacían
falta**. Abrir un documento ya tiene la zona de soltar de la bandeja; guardar
tiene la fila «Se guardará en» del panel de firma; y paginación y zoom viven en
la barra flotante del visor, que es exactamente lo que un menú *Ver* habría
contenido.

**Criterio de pertenencia al menú:** el menú contiene lo que habla **de la
aplicación**; nunca lo que habla **del documento**. Abrir, guardar, paginar y
ampliar son del documento y ya tienen su sitio en el recorrido; estado, ajustes,
ayuda e identidad son de la aplicación y no tienen otro. Un tope numérico
inventado se incumple el día que estorba, y el criterio ya excluye lo que el
tope querría excluir. Cuando el grupo de aplicación crezca de verdad, la
respuesta no es una barra de menús sino una página más dentro de Preferencias o
de Estado.

## Consequences

- Sin barra de menús se pierden los aceleradores de teclado visibles. No hay
  todavía una lista de atajos decidida; cuando la haya, hará falta un sitio
  donde consultarlos.
