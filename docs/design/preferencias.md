# Preferencias

Los ajustes de la aplicación. Se abre desde el menú de la
[cabecera](cabecera.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido, en cualquier momento.

## Estructura

`.rf-dialog` de 480 px sobre `.rf-scrim`, con los ajustes separados por
`.rf-divider` y un único botón «Cerrar» abajo a la derecha. Los cambios se
aplican al hacerlos: no hay «Guardar» ni «Cancelar».

## Los ajustes

1. **Recordar la última configuración de firma visible** (interruptor, activo
   por omisión). La página, la posición y el contenido del recuadro se
   reutilizan en el siguiente documento. Es una de las dos mejoras que
   justificaron el prototipo, y aquí deja de ser un comportamiento fijo para
   ser una decisión del usuario.
2. **Dónde se guarda el documento firmado** (desplegable). Por omisión, junto
   al documento original; la alternativa es una carpeta fija.
3. **Idioma** (desplegable). Español, català, euskara, galego, valencià e
   inglés: la misma lista que el cliente oficial. El cambio se aplica en
   caliente, como el resto del diálogo. Un idioma solo aparece aquí si tiene
   **todas** las cadenas traducidas.

Los valores posibles viven **dentro** de los desplegables. Nada de textos
debajo enumerando lo que el propio control ya muestra al abrirse.

## Estados

Uno. Los ajustes tienen siempre valor.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-field`, `.rf-label`, `.rf-input`,
`.rf-hint`, `.rf-divider`, `.rf-btn--primary`. El interruptor se maqueta con
tokens; no está en el sistema de diseño.

## Decisiones

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
