# El diario del trámite solo guarda campos cerrados

El diario del trámite acaba en un informe de problema, y ese informe se publica
en un issue de GitHub. Por eso el diario nunca guarda texto libre: cada evento
es un tipo con campos cerrados (enumeraciones, números, rangos de tamaño y el
host de la sede), y lo que no tiene campo no se graba.

## La regla

1. **Ni el mensaje de un error ni el `detail` de un `Failure`.** Del error
   solo se guardan su situación y los códigos que lo identifican.
2. **De una excepción de Java, la clase y la traza de pila**, con sus causas y
   sin sus mensajes: el código de AutoFirma es público, y sus mensajes llevan
   el DN, rutas y URLs.
3. **Del certificado, el tipo de almacén y la CA emisora.** Ni el titular, ni
   el número de serie, ni la huella.
4. **De una URL, solo el host.** La credencial de canal, las rutas y las
   queries no entran.
5. **El informe no redacta**: lo que sale es lo que el diario ya guardaba. La
   única excepción es la descripción que escribe la persona, que va aparte y
   se ve antes de enviarla.

## Consequences

- Un fallo cuya causa solo está en el mensaje de una excepción no se
  diagnostica con el informe; habrá que reproducirlo.
- Añadir un dato al diario obliga a darle un campo, y esa revisión es la que
  decide si es personal.
