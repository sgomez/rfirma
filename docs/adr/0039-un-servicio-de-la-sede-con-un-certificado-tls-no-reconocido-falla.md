# Un servicio de la sede con un certificado TLS no reconocido falla

Cuando AutoFirma 1.9.2 llama por HTTPS a un servicio de la sede (servlets del
lote, servidor intermedio, servidor trifásico o descarga del `dat`) y el
saludo TLS falla con `SSLHandshakeException`, `SSLErrorProcessor`
(`SSLErrorProcessor.java:23-60`) muestra un diálogo que pregunta si se confía
en el certificado del servidor, si hay entorno gráfico y no es `headless`. Si
la persona acepta, lo añade a un almacén de confianza temporal y reintenta.
rFirma valida con el almacén de confianza del sistema, sin excepciones.

## La regla

1. **rFirma no ofrece confiar en un certificado que el sistema no reconoce.**
   La petición falla, con el código de protocolo que cada cliente ya da a un
   servicio al que no se llega.
2. **La ventana de sede dice que el certificado del servidor no es de
   confianza**, y nombra el servidor, en vez de decir solo que no se llega a
   él. Lo que la persona tiene que resolver es de la sede o de su sistema,
   no de rFirma.
3. **Es una desviación deliberada.** La mide la comprobación de conformidad
   `the_site_services_accept_a_certificate_the_person_trusts`, con un
   servidor falso de la sede que presenta un certificado autofirmado. AutoFirma
   la cumple si se acepta el diálogo, y rFirma no la cumple a propósito: lleva la
   etiqueta `rfirma:adr-0039`.

Es la salida de riesgo grave de ADR-0023. La persona no sabe comparar una
huella de certificado, y un diálogo de «¿confías?» que aparece a mitad de un
trámite se acepta casi siempre. Aceptarlo entrega a quien intercepta la
conexión el documento, la firma o el lote, y en el servidor trifásico, lo que
se firma.

## Consequences

- Una sede cuyo servicio usa un certificado autofirmado, o de una CA interna
  que el sistema no tiene, funciona con AutoFirma y falla con rFirma. No se
  conoce ninguna. La FNMT, que emite los certificados de la mayoría de sedes
  públicas, está en los almacenes de confianza de las distribuciones.
- La solución para una CA interna es instalarla en el sistema, que es lo que
  haría falta para cualquier otro programa.

## Considered Options

**Reproducir el diálogo, solo para el trámite en curso.** Es la conformidad
literal, y acotarla al trámite limita cuánto dura el daño, pero no evita que
se acepte el certificado de quien intercepta la conexión. Descartada por
ADR-0023.

**Reproducirlo con la huella del certificado a la vista.** Solo sirve si la
persona tiene la huella correcta por otro canal, y ninguna sede la publica.
Descartada.
