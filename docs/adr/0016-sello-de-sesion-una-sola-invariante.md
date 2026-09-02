# El sello de sesión: una sola invariante entre prefirma y postfirma

La postfirma PAdES **regenera el PDF entero**, así que exige recibir exactamente lo mismo que
la prefirma o el documento sale con `Digest Mismatch` **sin lanzar ni un error**. El
[#7](https://github.com/sgomez/rfirma/issues/7) descubrió la restricción sobre los
`extraParams`, el [#14](https://github.com/sgomez/rfirma/issues/14) la midió sobre el `TIME`, y
el [#23](https://github.com/sgomez/rfirma/issues/23) añadió un tercer elemento que nadie
esperaba: la **zona horaria**, porque el desfase entra dentro del rango firmado. Los tres
fallan igual de callados.

AutoFirma lleva estos datos **en campos separados** del `TriphaseData` —`TIME` por un lado, los
`extraParams` por otro, la zona horaria por ninguno— y esa forma es justo la que dejó que se
colara el tercero durante años. rFirma los junta en un **sello de sesión**: un bloque único que
la prefirma devuelve, que Rust conserva sin interpretar y que la postfirma compara **antes de
firmar**, abortando si difiere. La comprobación pasa a ser una comparación de bytes, y no hay
manera de olvidarse de un campo porque no hay campos que recordar.

El sello lleva los `extraParams` **efectivos**, no los que envió el llamante: `PdfSessionManager`
muta el `Properties` que recibe —reescribe `signatureSubFilter` en cuanto hay política o perfil
baseline— y `PAdESTriPhaseSigner:174` **no lo clona**, así que el puente puede releer el objeto
justo después de `preSign` y serializar lo que de verdad se usó. Guardar lo enviado en vez de lo
efectivo reintroduciría el fallo por otra puerta.

Dentro van también el algoritmo de firma y el `TIME`. Fuera quedan `PRE` y `PID`, que son
salida de la prefirma y no configuración.

Y dentro van, además, el **SHA-256 del PDF** y el **SHA-256 de la cadena de certificados** (sus
DER concatenados en orden). No son configuración, pero son lo que sigue **viajando aparte**
hasta la postfirma: postfirmar un PDF que no es el prefirmado da `Digest Mismatch`, y hacerlo
con otro certificado da un documento que dice estar firmado por quien no lo firmó. Las dos cosas
completan sin error, que es exactamente el fallo que este ADR existe para cerrar, así que el
sello las ata igual que ata el `TIME`.

## Consecuencias

La zona horaria deja de heredarse del entorno: la prefirma captura la del sistema y la
postfirma la impone. Se descarta fijarla a UTC, que sería más simple pero mentiría en la fecha
que el usuario ve estampada en el recuadro.

El sello es opaco para Rust **por diseño**. Si algún día hace falta leer un valor de dentro, la
respuesta es que la prefirma lo devuelva aparte, no abrir el sello: en cuanto Rust lo
interpreta, puede reconstruirlo, y un sello reconstruible no protege de nada.

## Enmienda: el conjunto de páginas viaja dentro del sello, y por eso no hay obra

Añadido con el hito v0.3 ([#148](https://github.com/sgomez/rfirma/issues/148)).
No cambia el mecanismo: lo registra, porque la pregunta se abrió y la respuesta
está medida ([#150](https://github.com/sgomez/rfirma/issues/150)).

El multipágina no añade un campo al sello. **`signaturePages` es un
`extraParam`**, cruza el puente con toda su gramática sin una línea de Java
nueva —`SessionStamp.parseParams` es un `Properties.load` en crudo— y **no sale
mutado de la prefirma**, así que los `extraParams` **efectivos** que este ADR ya
serializa lo llevan dentro por existir. La invariante sigue siendo una sola
comparación de bytes, y el conjunto de páginas queda atado igual que el `TIME`.

Lo que **no** entra en el sello, y hay que decirlo para que no se le atribuya:
`imagePage`, que es el sello sin firmar y está fuera de la lista cerrada de
ajustes; y la **validación del destino**, que ocurre **antes** de llamar al
puente y es de Rust. `PdfUtil.getPages` no lanza nunca —recorta, avisa por
`WARNING` y cae en la última página—, y la respuesta de la prefirma no dice
dónde acabó el widget: no hay nada que el sello pueda proteger ahí, porque el
fallo ya ocurrió y se llama éxito.
