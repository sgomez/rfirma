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
   ser una decisión del usuario. Apagado significa **no guardarla**: el
   recuadro arranca en el valor por omisión en cada documento.
2. **Recordar mi actividad** (interruptor, activo por omisión), con un botón
   **«Vaciar la lista»** al lado. Cubre los documentos recientes y el
   certificado usado la última vez: es la misma promesa a quien firma en un
   ordenador compartido. Apagarlo **borra** lo ya guardado, previa
   confirmación; vaciar sin apagar es «hoy no, mañana sí».
3. **Dónde se guarda el documento firmado** (desplegable). Por omisión, junto
   al documento original; la alternativa es una carpeta fija. La carpeta fija
   se comprueba **antes de firmar**: si no existe o no es escribible, se avisa
   en el panel y esa firma se guarda junto al original, sin cambiar la
   preferencia.
4. **Idioma** (desplegable). Español, català, euskara, galego, valencià e
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
`.rf-hint`, `.rf-divider`, `.rf-btn--primary`. El interruptor se maqueta con
tokens; no está en el sistema de diseño.

## Decisiones

**Qué se recuerda entre sesiones y dónde vive** está fijado en el
[ADR-0010](../adr/0010-memoria-entre-sesiones.md): los dos interruptores de
arriba, el borrado que provoca apagar el segundo, y la comprobación previa de
la carpeta fija salen de ahí.

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
