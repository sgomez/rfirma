# El almacén que nombra la sede se lee, y el que rFirma no abre se rechaza

Una sede puede acotar de qué almacén sale el certificado: manda `keystore` con
el nombre en claro, o `ksb64` con ese mismo nombre en Base64, y detrás del
primer `:` la ruta de la biblioteca PKCS#11 que lo sirve. AutoFirma 1.9.2 lo
obedece (`UrlParameters.getKeyStoreName` y `getDefaultKeyStoreLib`, y los
cuatro lanzadores que construyen el almacén con lo que sale de ahí).

rFirma **no tiene más almacén que el suyo**: el listado de certificados sale de
los almacenes NSS del equipo y de los módulos PKCS#11 instalados —los que conoce
de antemano y los que la instalación registra en p11-kit—
(`identity/adapters/pkcs11/stores.rs` y `p11kit.rs`), y no hay forma de pedirle
que abra otro.
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
3. **Sin biblioteca, se obedece un solo almacén: el de la familia NSS** (`SHARED_NSS` y
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
   existen en Linux; las tarjetas por su nombre del original (`DNIEJAVA`,
   `CERES`…), que quedan fuera de rFirma por desviación declarada; y el
   `PKCS11` sin biblioteca, que no dice a qué módulo acotar.
5. **El código nativo que carga rFirma lo decide la instalación, no la sede.**
   rFirma nunca carga un módulo por el hecho de que lo nombre la sede: los
   suyos son los que descubre —los candidatos fijos y los que registra p11-kit
   en `/usr/share/p11-kit/modules`, `/etc/pkcs11/modules` y
   `~/.config/pkcs11/modules`, sin los de `trust-policy: yes` ni los que
   `enable-in` o `disable-in` dejan fuera del programa `rfirma`—. Por eso
   `PKCS11:<ruta>` (o su nombre visible, `PKCS#11`) se obedece solo como
   **acotación**: si la ruta, canonizada, es la de un módulo PKCS#11 ya
   descubierto, el listado se reduce a los certificados de ese módulo; si no lo
   es, o no existe, sale con `SAF_08` como en la regla 4. Cualquier otro nombre
   con biblioteca —también el de la familia NSS— sigue siendo rechazo.
6. **Un nombre que el original no reconoce se ignora**, como allí, donde acaba
   en el almacén por omisión del sistema. Rechazarlo endurecería una negativa
   que el original no hace, y la sede no habría acotado nada de todos modos.
7. **Solo se mira donde hay certificado que elegir.** `save` y `load` no lo
   leen, igual que en el original.

El dominio conserva la ruta de la biblioteca tal y como vino, sin comillas, y
la lleva en el filtro de la sede; el adaptador de identidad la canoniza y la
compara con los módulos descubiertos, porque canonizar es tocar el disco y eso
no cabe en una regla pura.

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
- La sede que acota a un módulo que la instalación ya ofrece —el SoftHSM de
  pruebas, el `opensc-pkcs11.so` de las tarjetas— ve solo sus certificados,
  como en el original. La que nombra otra ruta recibe `SAF_08` y la ventana
  lo cuenta como `unsupportedKeyStore`: rFirma no ha cargado nada.
- Una ruta con otra forma que llega al mismo fichero —sin el directorio
  multiarch, por un enlace— acota igual, porque se compara canonizada.
- Instalar un módulo PKCS#11 con su fichero `.module` basta para que rFirma lo
  liste, sin tocar rFirma. El almacén de CA de p11-kit no entra nunca: con él,
  el listado ofrecería las CA del sistema como certificados de firma.
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

**Rechazar siempre `PKCS11:<ruta>`, como cualquier biblioteca.** Era la regla
anterior, y era segura: rFirma no cargaba nada que nombrara la sede. Pero
rechazaba también la sede que acota al mismo módulo que rFirma ya tenía
abierto —la prueba de conformidad del token de pruebas pide justo eso— y le
daba un `SAF_08` donde el original le ofrece los certificados. Lo que la hacía
segura no era rechazar la ruta, sino no cargarla; la acotación conserva eso y
deja de negar lo que sí se puede cumplir. Descartada.

**Cargar el módulo que nombra la sede, como el original.** Es la compatibilidad
completa, y pone en manos de una página web qué biblioteca nativa entra en el
proceso de rFirma. Descartada.

**Acotar cualquier almacén nombrado, no solo el PKCS#11.** El listado solo
distingue certificados por el módulo que los sirve, y el resto de nombres del
catálogo no dice ninguno que rFirma tenga. Descartada: el rechazo de la regla 4
dice la verdad y no finge una restricción que no se aplica.

**Preguntar a `pkg-config` por el directorio de módulos de p11-kit.** Da la
ruta exacta del sistema en el que se compiló, pero `pkg-config` es una
herramienta de compilación que el equipo de la persona no tiene por qué
tener. Descartada: la ruta relativa de un `.module` se busca en los directorios
de módulos de Debian (multiarch), Fedora (`lib64`) y Arch (`lib`).
