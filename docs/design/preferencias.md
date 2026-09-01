# Preferencias

Los ajustes de la aplicación. Se abre desde el menú de la
[cabecera](cabecera.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido, en cualquier momento.

## Estructura

`.rf-dialog` de 480 px sobre `.rf-scrim`, con los ajustes separados por
`.rf-divider` y un único botón «Cerrar» abajo a la derecha. Los cambios se
aplican al hacerlos: no hay «Guardar» ni «Cancelar».

### Geometría

- Diálogo de **480 px**.
- Cada interruptor es una fila con la pastilla de **40×24 px** (pomo de 16 px)
  **delante** del texto, 16 px de separación, y el texto en `.rf-prose` con su
  ayuda en `.rf-hint` 4 px debajo. La ayuda se sangra hasta la columna del
  texto: 40 px de pastilla más los 16 de separación.
  Esos 16 px son **de este diálogo**: el panel de firma usa el mismo
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
- «Vaciar la lista» cuelga del ajuste que lo explica y sigue esa misma sangría.
  Es un botón secundario de 32 px de alto, 8 px de relleno lateral y 12 px de
  cuerpo — el mismo tamaño menor que el `Cambiar` del pie del panel.
- Los tres desplegables son `.rf-field` con `.rf-label` y un cierre que
  reutiliza `.rf-input`, con el chevrón a la derecha y la lista flotando 4 px
  por debajo.

**El desplegable no es un `<select>` nativo.** Se intentó, y no vale: el cierre
se estila con CSS, pero la lista que se abre la pinta el sistema de ventanas
—GTK, bajo WebKitGTK— y no la hoja de estilos, así que las opciones salían con
los colores del escritorio en medio de un diálogo hecho con los tokens del
sistema de diseño. No es una limitación que se pueda rodear con más CSS: ese
trozo de interfaz no es nuestro. A cambio hay que reponer a mano lo que el
elemento nativo daba gratis —`combobox` + `listbox` con
`aria-activedescendant`, flechas, Inicio, Fin, Intro, Escape, cierre al pulsar
fuera y foco de vuelta—, y eso es lo que hace `Select`. Un `<div>` con un
`onClick` no es un desplegable, es un dibujo de uno.

### Las tres diferencias con el canvas

Decididas al transcribir, y no se reabren (ID-44):

1. **«Junto al documento original» se ha eliminado.** El artboard lo ofrece
   como destino del firmado, pero el
   [ADR-0011](../adr/0011-destino-del-documento-firmado.md) midió que bajo el
   arenero **no es implementable**: escribir un hermano del fichero que entrega
   el portal deja un `.xdp-…` huérfano y **no da error**, que es la peor de las
   formas de fallar —el usuario cree que ha guardado y no ha guardado—. El
   desplegable lista solo las carpetas que el arenero permite. El canvas es
   anterior a esa medición.
2. **«Recordar mi actividad» y «Vaciar la lista» se quedan**, aunque el
   artboard no los dibuje: los exige el ID-34, y sin ellos no hay forma de
   cumplir la promesa a quien firma en un ordenador compartido. Se maquetan con
   el patrón visual de los ajustes que el artboard sí dibuja —interruptor
   delante, texto y ayuda al lado—, con la geometría añadida que queda anotada
   arriba.
3. **El tema se queda**, y llegó después: los tokens del bundle ya traían los
   dos temas y el `data-theme` que fuerza cualquiera de ellos; lo que faltaba
   era dónde elegirlo. Se maqueta como los otros dos desplegables.

## Los ajustes

1. **Recordar la última configuración de firma visible** (interruptor, activo
   por omisión). La página, la posición y el contenido del recuadro se
   reutilizan en el siguiente documento. Es una de las dos mejoras que
   justificaron el prototipo, y aquí deja de ser un comportamiento fijo para
   ser una decisión del usuario. Apagado significa **no guardarla**: el
   recuadro arranca en el valor por omisión en cada documento.
2. **Recordar mi actividad** (interruptor, activo por omisión), con un botón
   **«Vaciar la lista»** al lado. Cubre los documentos recientes y el
   certificado usado la última vez: es la misma promesa a quien firma en un
   ordenador compartido. Apagarlo **borra** lo ya guardado, previa
   confirmación; vaciar sin apagar es «hoy no, mañana sí».
3. **Dónde se guarda el documento firmado**. En el flatpak es una **carpeta**,
   y el ajuste enseña su **nombre**, no su ruta: bajo el arenero la aplicación
   escribe en ella pero la única palabra que tiene de ella es su último
   segmento, y enseñar la ruta donde se puede y el nombre donde no sería la
   misma pantalla contando cosas distintas según el empaquetado. Por omisión,
   la carpeta de documentos del usuario. Se comprueba **antes de firmar**: si
   no está o no se puede escribir, se avisa en el pie del panel y ahí mismo se
   ofrece `Cambiar`; ni se degrada a otro sitio ni se apaga el botón de firmar.
   La carpeta **no se crea nunca** si no está.

   *Junto al documento original* **no aparece aquí en el flatpak**: bajo el
   arenero la aplicación no puede saber de qué carpeta salió el original. Es
   una capacidad que llegará con los instaladores nativos, no una opción
   atenuada que le cuente al usuario nuestros problemas de empaquetado.
   Razonamiento y alternativas descartadas en el
   [ADR-0011](../adr/0011-destino-del-documento-firmado.md).

4. **Tema** (desplegable): *El del sistema*, *Claro* u *Oscuro*. Por omisión,
   el del sistema, que **no es «claro»**: es no forzar nada y dejar que mande
   `prefers-color-scheme`. Los otros dos escriben `data-theme` en `<html>`, que
   es lo que los tokens de color del bundle leen para redefinir los roles. El
   cambio se aplica en caliente, como el resto del diálogo.
5. **Idioma** (desplegable). Español, català, euskara, galego, valencià e
   inglés: la misma lista que el cliente oficial. El cambio se aplica en
   caliente, como el resto del diálogo. Un idioma solo aparece aquí si tiene
   **todas** las cadenas traducidas. En la primera ejecución sale del locale
   del sistema cotejado contra esos seis, con español como recurso; no hay
   diálogo de bienvenida que pregunte lo que la aplicación ya sabe.

Los valores posibles viven **dentro** de los desplegables. Nada de textos
debajo enumerando lo que el propio control ya muestra al abrirse.

## Estados

Uno. Los ajustes tienen siempre valor.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-field`, `.rf-label`, `.rf-input`,
`.rf-hint`, `.rf-divider`, `.rf-card--elevated`, `.rf-btn--primary`. El
interruptor y el desplegable se maquetan con tokens; ninguno de los dos está en
el sistema de diseño.

## Decisiones

**Dónde cae el documento firmado** está fijado en el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md): la carpeta por
omisión, el nombre en vez de la ruta, la comprobación previa sin degradación y
el `Cambiar` que vale solo para una firma.

**Qué se recuerda entre sesiones y dónde vive** está fijado en el
[ADR-0010](../adr/0010-memoria-entre-sesiones.md): los dos interruptores de
arriba, el borrado que provoca apagar el segundo, y la comprobación previa de
la carpeta fija salen de ahí. El tema entró después, por la enmienda de ese
mismo ADR.

**Los ajustes se guardan al elegirlos, en el disco.** El diálogo llama a
`PreferencesStore`, y debajo son `read_configuration` y `write_configuration`,
que pasan por `memory::Memory::remember_configuration`: el único sitio donde el
borrado del estado al apagar «Recordar mi actividad» no se puede olvidar.

El **alcance de la traducción** está fijado en
[#16](https://github.com/sgomez/rfirma/issues/16): las seis lenguas, cadenas
propias escritas desde cero con el vocabulario de `CONTEXT.md`, mensajes de
error que traducen situaciones nuestras y no el texto de PKCS#11 ni el del
puente Java, y texto de la firma visible que sigue al idioma de la aplicación.

Preferencias existe desde el primer día en lugar de esperar a tener «algo que
configurar»: el propio recorrido ya generó dos ajustes reales, y un menú que
promete preferencias y abre un diálogo vacío es peor que no tenerlas.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Preferencias · desde el menú».
