# El motor de filtros de certificado en el puente

Sondeo del [#314](https://github.com/sgomez/rfirma/issues/314), parte del mapa
[#308](https://github.com/sgomez/rfirma/issues/308).

**La pregunta**: si traemos `afirma-keystores-filters` al puente para resolver
los filtros que manda la sede, ¿vuelve `libawt.so` a la imagen nativa —lo que el
ADR-0012 echó del proyecto—, cuánto engorda la librería y aguanta la frontera FFI
sin estado?

**La respuesta corta**: **no vuelve nada**, porque **`libawt.so` nunca se había
ido**; el motor de filtros no añade ni un método de AWT. La librería pasa de
**27 723 872 a 28 379 232 bytes**, **+640 KiB (+2,4 %)**. **No hace falta ninguna
exclusión en el `pom.xml`**: se midieron las dos variantes y producen una imagen
del mismo tamaño exacto. Y la frontera se sostiene sin estado, con dos avisos que
sí importan: la lista de filtros es **disyuntiva**, y un filtro con un nombre
desconocido **se ignora en silencio y deja pasar todo**.

Rama del experimento: `research/keystores-filters-awt-probe`. Es desechable; no
hay PR.

---

## Cómo se midió

| | |
|---|---|
| GraalVM | CE `25.3.4+1.r25-graalce` (ADR-0004), vía `GRAALVM_HOME` explícito |
| Módulo | `es.gob.afirma:afirma-keystores-filters:1.9.1` desde `~/.m2` (ADR-0002) |
| Orden | `mvn -B package -DskipTests` y luego `native-image --shared -cp <jar>:<cp.txt>`, exactamente la línea de `just native` |
| Certificados | `testdata/fnmt/active-rsa.p12` (vigente) y `expired-rsa.p12` (caducado en 2020), en DER |

Tres imágenes construidas en directorios limpios y separados
(`target/native-baseline`, `target/native-filters`, `target/native-excl`), no una
sobre otra. Es importante: `just native` **no vacía** su directorio de
construcción, así que medir en el mismo sitio dos veces mezcla artefactos.

Añadir la dependencia al `pom.xml` **no mide nada por sí solo** —`native-image`
compila lo alcanzable, no lo que hay en el *classpath*—, así que el experimento
incluye un punto de entrada real, `autofirma_filter_certificates`, que instancia
`CertFilterManager` y aplica sus filtros. Sin él, las tres imágenes serían
idénticas y la conclusión, falsa.

---

## 1. ¿Aparece `libawt.so`?

No aparece: **ya estaba**. La construcción de `main`, sin tocar nada, emite hoy
los mismos cinco auxiliares:

```
libawt.so         924 120
libawt_xawt.so    520 424
liblcms.so        541 200
libawt_headless.so 35 968
libjava.so / libjvm.so  9 232 cada uno
```

Los ficheros son **idénticos byte a byte** antes y después de traer los filtros
(`md5sum` coincide para `libawt.so` y `liblcms.so`). Lo que hace que no lleguen
al paquete no es que no se generen: es que `just native` copia **un solo
fichero** y comprueba que no sobre nada en el directorio de distribución. La
invariante del ADR-0012 la sostiene esa receta, no la ausencia de los `.so`.

> **Corrección al `CLAUDE.md`.** Dice «los cinco auxiliares de AWT desaparecieron
> al excluir `afirma-ui-utils`». No desaparecieron: `native-image` los sigue
> produciendo en `target/native/`. Lo que la exclusión consiguió —y esto sí se
> confirma— es que **`javax.imageio` deje de ser alcanzable**: cero métodos en el
> árbol de llamadas. La frase debería decir «dejan de instalarse», no «dejan de
> generarse».

### Por qué siguen ahí, con filtros o sin ellos

El árbol de llamadas (`-H:+PrintAnalysisCallTree -H:PrintAnalysisCallTreeType=CSV`)
deja **47 métodos** de AWT alcanzables en la imagen **con** filtros, y son de dos
clases y nada más:

* `java.awt.Color`
* `java.awt.color.ICC_Profile`

Y sus llamantes, todos sin excepción, son iText:

```
com.aowagie.text.Font
com.aowagie.text.Jpeg
com.aowagie.text.pdf.PdfContentByte
com.aowagie.text.pdf.PdfICCBased
com.aowagie.text.pdf.PdfOutline
```

Es la maquinaria PAdES que ya estaba. `ICC_Profile` arrastra el módulo de gestión
de color del JDK, y de ahí `liblcms.so` y `libawt.so`. **Ni un solo llamante sale
de `es.gob.afirma.keystores`.**

## 2. ¿Cuánto engorda?

| Imagen | `librfirma_crypto.so` | Unidades de compilación |
|---|---:|---:|
| `main`, sin filtros | 27 723 872 B | 18 201 |
| Con `afirma-keystores-filters` y su punto de entrada | 28 379 232 B | 18 339 |
| **Diferencia** | **+655 360 B (+640 KiB, +2,4 %)** | **+138** |

El montón de la imagen sube de 12,94 a 13,44 MiB. Los 640 KiB salen sobre todo de
dos sitios: las dieciséis clases de filtro con su `CertFilterManager`, y **95
métodos de `javax.naming`** —el intérprete RFC 2254 (`SearchFilter`) usa
`javax.naming.directory` para trocear los DN.

## 3. ¿Hace falta una exclusión, como con `afirma-ui-utils`?

**No.** Y no es una opinión: se construyó la imagen **con** y **sin** las
exclusiones defensivas, y pesan lo mismo hasta el byte (28 379 232 en las dos).

El árbol de dependencias asusta. `afirma-keystores-filters` arrastra
`afirma-core-keystores`, y con él:

```
afirma-core-prefs
afirma-keystores-jmulticard-ui       <- un módulo de interfaz
jmulticard + jmulticard-jse          <- toda la fontanería de tarjeta y DNIe
  org.bouncycastle:bcpkix/bcprov/bcutil-jdk18on:1.84   <- un SEGUNDO BouncyCastle
```

Y `afirma-core-keystores` tiene, en efecto, un fichero que toca AWT —el único de
todo el módulo—: `AOKeyStoreManagerFactory`. Era la sospecha del ticket.

Ninguno de ellos entra en la imagen. Recuento en el árbol de llamadas:

| Paquete | Métodos alcanzables |
|---|---:|
| `es.gob.afirma.jmulticard` | 0 |
| `org.bouncycastle` (el `jdk18on`) | 0 |
| `es.gob.afirma.core.prefs` | 0 |
| `es.gob.afirma.ui` | 0 |
| `javax.imageio` | 0 |
| `AOKeyStoreManagerFactory` | 0 |

La razón es simple y se ve en el código fuente del original: **`src/main` de
`afirma-keystores-filters` no importa `AOKeyStoreManagerFactory` en ningún
sitio**. Quien lo importa son dos de sus *tests*
(`TestRFC2254CertificateFilter`, `TestPseudonymFilter`), que abren un almacén
Windows o Mozilla para tener certificados con los que probar. De
`afirma-core-keystores` el código de producción usa exactamente dos tipos, y los
dos son inocentes: `CertificateFilter` (abstracta, `X509Certificate` y
`ArrayList`) y `MultipleCertificateFilter`.

El caso es **distinto** al de `afirma-ui-utils` del ADR-0012, y conviene no
confundirlos. Allí la exclusión era necesaria porque `PdfPreProcessor` llamaba a
la clase **por reflexión**, y la reflexión el analizador no la ve: había que
quitar el `.jar` para que el `catch (Throwable)` degradase. Aquí no hay
reflexión; hay un grafo de llamadas estático que el analizador recorre entero y
poda solo.

**Recomendación**: traer la dependencia **sin exclusiones**. Añadirlas no gana un
byte y sí añade cuatro bloques de `pom.xml` que hay que mantener y explicar. Lo
que sí conviene es una prueba que fije la invariante —que
`AOKeyStoreManagerFactory` no sea alcanzable—, porque el día que alguien
instancie un `AOKeyStoreManager` «para reutilizar código», AWT entra de verdad y
nada lo avisa.

## 4. La forma de la frontera FFI

Se sostiene sin estado, igual que la prefirma. Entran N certificados en DER
(Base64, separados por `;`) más la cadena `filters=` tal cual la manda la sede, y
sale qué índices pasan:

```
autofirma_filter_certificates(thread, certChainB64, filterParams) -> char*
  {"ok":true,"selected":[0,2]}
  {"ok":false,"error":"<clase>: <mensaje>"}
```

Medido contra la imagen nativa por `dlopen` desde un arnés en C, con el
certificado vigente en el índice 0 y el caducado en el 1:

| `filterParams` | `selected` |
|---|---|
| *(vacío)* | `[0]` |
| `filters=nonexpired:` | `[0]` |
| `filters=subject.contains:PRUEBAS` | `[0,1]` |
| `filters=subject.contains:NOEXISTE` | `[]` |
| `filters=nonexpired:;subject.contains:PRUEBAS` | `[0]` |
| `filters.1=subject.contains:NOEXISTE`<br>`filters.2=nonexpired:` | `[0]` |
| `filters=issuer.rfc2254:(cn=AC FNMT Usuarios)` | `[0,1]` |
| `filters=thumbprint:SHA1:0011` | `[]` |
| `filters=basura:` | `[0,1]` |

`CertFilterManager` no guarda nada entre llamadas: se construye desde un
`Properties`, expone `getFilters()` y cada filtro decide con
`matches(X509Certificate)`. No hay `AOKeyStoreManager`, no hay diálogo, no hay
sesión. Encaja con el sello del ADR-0016 sin inventar nada: **este comando no
necesita sello**, porque no abre ni continúa ninguna operación.

Tres cosas que la primera versión del sondeo hizo mal y conviene no repetir:

1. **La lista es disyuntiva, no conjuntiva.** Pasa el certificado que satisfaga
   **cualquiera** de los elementos de `getFilters()`; lo conjuntivo vive
   **dentro** de cada elemento, porque un `filters=a:x;b:y` se compila en un
   `MultipleCertificateFilter`. Así lo aplica el original en
   `KeyStoreUtilities.getAliasesByFriendlyName`, uniendo en una tabla los alias
   que devuelve cada filtro. Con un AND ingenuo, la fila de `filters.1`/`filters.2`
   de arriba da `[]` en vez de `[0]`: una sede que ofrezca dos alternativas se
   queda sin certificados.
2. **Sin filtros no significa «pasa todo».** Si la sede no manda nada,
   `CertFilterManager` añade por su cuenta un `ExpiredCertificateFilter(false)`,
   citando la ETSI TS 119 102-1: nadie debería firmar con un certificado
   caducado. De ahí que la fila vacía dé `[0]` y no `[0,1]`. Es un
   comportamiento **deseable** que conviene heredar tal cual, no un efecto
   colateral que corregir.
3. **Un filtro desconocido se ignora en silencio y deja pasar todo.** La última
   fila es la peligrosa: `filters=basura:` devuelve `[0,1]` y solo deja un
   `WARNING` en el registro (*«Se omitirá el filtro 'basura:' por no estar
   reconocido»*). Es *fail-open*. Si la sede manda un filtro que esta versión de
   la librería no conoce, la restricción **desaparece** y la persona firma con un
   certificado que la sede pensaba haber excluido. Quien implemente esto tiene
   que decidir a propósito qué hacer —lo razonable es contar los filtros
   reconocidos frente a los declarados y avisar, o negarse— y no heredar el
   silencio.

---

## Qué queda medido, y qué no

Medido: que la imagen crece 640 KiB, que AWT no se mueve, que las exclusiones no
sirven de nada, y que las nueve combinaciones de filtro de la tabla dan lo que el
original daría.

**No medido**: los filtros que necesitan más que el certificado. `qualified:` y
`pseudonym:` consultan políticas y `QCStatements` y solo se pueden validar contra
certificados cualificados de verdad; `ssl:` y `dnie:` presuponen un almacén con
esa pinta. La tabla de arriba los deja fuera a propósito: con dos certificados de
usuario de la FNMT no se distingue «funciona» de «no encuentra nada».

Tampoco se ha medido el arranque: 640 KiB más de imagen no deberían tocarlo, pero
el sondeo no lo comprobó.

## Reproducirlo

```sh
git checkout research/keystores-filters-awt-probe
cd rfirma-native-bridge
mvn -B package -DskipTests
mkdir -p target/native-filters && cd target/native-filters
"$GRAALVM_HOME/bin/native-image" --shared \
    -cp "../rfirma-native-bridge-0.1.0.jar:$(cat ../cp.txt)"
```

Para el árbol de llamadas, los dos indicadores extra:

```sh
-H:+UnlockExperimentalVMOptions -H:PrintAnalysisCallTreeType=CSV -H:+PrintAnalysisCallTree
```

deja `reports/call_tree_methods.csv` y `reports/call_tree_invokes.csv`, que es de
donde salen los recuentos de este informe.
