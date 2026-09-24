# El almacén que nombra la sede se lee, y el que rFirma no abre se rechaza

Una sede puede acotar de qué almacén sale el certificado: manda `keystore` con
el nombre en claro, o `ksb64` con ese mismo nombre en Base64, y detrás del
primer `:` la ruta de la biblioteca PKCS#11 que lo sirve. AutoFirma 1.9.2 lo
obedece (`UrlParameters.getKeyStoreName` y `getDefaultKeyStoreLib`, y los
cuatro lanzadores que construyen el almacén con lo que sale de ahí).

rFirma **no tiene más almacén que el suyo**: el listado de certificados sale de
los almacenes NSS del equipo y de los módulos PKCS#11 instalados
(`identity/adapters/pkcs11/stores.rs`), y no hay forma de pedirle que abra otro.
Hasta este ADR el parámetro se ignoraba en silencio, que es lo peor de las tres
opciones: la sede creía haber acotado el origen del certificado y firmaba con
uno cualquiera.

## La regla

1. **Se lee como en el original.** El `keystore` heredado gana al `ksb64`
   cuando vienen los dos, porque es el orden del original. El `ksb64` que no es
   Base64 **se ignora** —también como allí— y no rechaza la operación. Un valor
   sin `:` es todo nombre; con `:`, lo de la izquierda es el nombre y lo de la
   derecha la biblioteca.
2. **El almacén se reconoce por sus dos nombres**, como el original: primero el
   visible (`"PKCS#12 / PFX"`, `"Llavero de Mac"`, `"Mozilla / Firefox
   (unificado)"`…), sin distinguir mayúsculas y recortado, y solo si no, el de
   la constante (`PKCS12`, `APPLE`, `MOZ_UNI`…). Son las dos puertas de
   `SimpleKeyStoreManager.getKeyStore` (1.9.2), y sin la primera la mitad del
   catálogo entraba por la puerta de atrás y se ignoraba en silencio. La
   comparación por el nombre de la constante tampoco distingue mayúsculas,
   donde el original usa un `AOKeyStore.valueOf` que sí lo hace: es más laxa a
   propósito, y en la dirección segura —rechaza de más, nunca de menos—.
3. **Se obedece un solo almacén: el de la familia NSS** (`SHARED_NSS` y
   `MOZ_UNI`). Es el almacén que rFirma ya abre, y es además el que el propio
   original elige en Linux cuando nadie nombra ninguno
   (`AOKeyStore.getDefaultKeyStoreTypeByOs`). Obedecerlo no cambia de dónde
   sale el certificado: solo confirma que la sede pidió lo que va a ocurrir.
4. **Cualquier otro nombre de `AOKeyStore` sale con `SAF_08`**
   (`ERROR_CANNOT_ACCESS_KEYSTORE`), nombrando el parámetro por el que vino, y
   la ventana lo cuenta como `unsupportedKeyStore`. Es el código con el que el
   original contesta cuando no puede abrir el almacén que se le pide —el de
   Windows en Linux, por ejemplo—, y por eso el que una sede sabe tratar. Ahí están el `PKCS12`, que necesitaría una contraseña
   que la orden de instalación no lleva; el `WINDOWS` y el `APPLE`, que no
   existen en Linux; y las tarjetas, que quedan fuera de rFirma por desviación
   declarada.
5. **Nombrar biblioteca es rechazo, aunque el almacén sea el de la familia
   NSS.** rFirma no carga un módulo PKCS#11 porque se lo diga una sede: los
   suyos los descubre él.
6. **Un nombre que el original no reconoce se ignora**, como allí, donde acaba
   en el almacén por omisión del sistema. Rechazarlo endurecería una negativa
   que el original no hace, y la sede no habría acotado nada de todos modos.
7. **Solo se mira donde hay certificado que elegir.** `save` y `load` no lo
   leen, igual que en el original.

La ruta de la biblioteca se queda como vino, sin comillas: rFirma no la abre, y
canonizarla sería tocar el disco desde una regla pura.

## Consequences

- La sede que acota el almacén recibe el mismo `SAF_08` que le daría el
  original al no poder abrirlo, y puede decidir. La que no lo acota no nota
  nada.
- El código no distingue el motivo del rechazo; la ventana sí, porque
  `unsupportedKeyStore` cuenta qué almacén o qué biblioteca se nombró.
- No hace falta el camino de contraseña de un `.p12` que la orden
  `site_install_certificate` no tiene: el `PKCS12` de una sede se rechaza antes
  de necesitarlo. El día que ese camino exista, este ADR se reescribe para
  obedecerlo.
- Si `AOKeyStore` gana nombres en una versión posterior del original, la tabla
  de `site/domain/protocol/key_store.rs` hay que volver a medirla contra ese
  tag.

## Considered Options

**Seguir ignorándolo.** Es lo que había, y es aceptar de más: la sede pide un
almacén concreto y se le firma con otro sin decírselo. Descartada.

**Rechazar con `SAF_07` (`ERROR_CANNOT_FIND_KEYSTORE`).** Su nombre parece
describir el caso, pero en 1.9.2 es un código huérfano: está en el catálogo y
el original no lo emite nunca. Una sede no ha visto jamás un `SAF_07` y no
tiene rama que lo trate; el `SAF_08` es lo que recibe del original cuando el
almacén que nombra no se abre. Descartada.

**Rechazar el parámetro entero, lo nombre lo que lo nombre.** Simple, y deja
fuera a las sedes que nombran el almacén NSS que rFirma ya usa —que es
justamente el caso en el que no hay nada que arreglar—. Descartada.

**Obedecer el `PKCS12` abriendo el fichero que nombra la sede.** Es la
compatibilidad completa, y exige una contraseña que hoy nadie pide y un camino
por el que una sede haría que rFirma leyera un fichero del equipo que la
persona no ha elegido. Descartada mientras no haya ADR que decida ese camino.

**Guardar el almacén nombrado y pasárselo al listado de certificados.** Sería
acotar de verdad el origen, y hoy no hay a quién pasárselo: el listado no
distingue por almacén. Descartada por ahora; el rechazo dice la verdad y no
finge una restricción que no se aplica.
