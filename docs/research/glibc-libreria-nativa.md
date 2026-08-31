# Contra qué glibc se puede ejecutar la librería nativa

Medición para el issue [#23](https://github.com/sgomez/rfirma/issues/23). El
[#17](https://github.com/sgomez/rfirma/issues/17) eligió flatpak como único canal y dejó la glibc
como **el riesgo vivo**, con la frase «encaja hoy por coincidencia»; el
[#22](https://github.com/sgomez/rfirma/issues/22) la heredó sin medirla. Aquí se mide. **Registra
hechos, no decide** el manifiesto: eso es el #22.

Entorno: GraalVM CE 25.3.4.1, Maven 3.9.12, anfitrión Ubuntu con **glibc 2.43**
(`ldd (Ubuntu GLIBC 2.43-2ubuntu2.3) 2.43`), binutils, Docker 29.7.2, flatpak 1.16.6 con
`org.gnome.Platform` 49 y 50. Banco de pruebas: `rfirma-native-bridge/`, imagen `ce25-awt`
reconstruida desde cero para esta medición (35.326.048 bytes, idéntica en tamaño a la del #14), y
un guion nuevo, `testbench/run-cross-glibc.sh`.

## Veredicto

**La glibc mínima soportada es 2.34, y el riesgo que el #17 dejó abierto queda desmentido.** El
suelo lo pone `librfirma_crypto.so`; los otros ocho ficheros no pasan de `GLIBC_2.4`. El ciclo
trifásico completo con rúbrica de imagen se ejecuta y produce firma válida en las cuatro glibc
probadas, incluida una **cuatro versiones por debajo** de la del anfitrión, y el PDF sale
**idéntico bit a bit**. **No hace falta construir dentro de `org.gnome.Sdk`**: construir en el
anfitrión vale.

Con flatpak, además, la pregunta se desactiva casi entera: la aplicación no se ejecuta contra la
glibc de la distribución del usuario sino contra la del runtime, que es una sola y la elegimos
nosotros. La glibc del anfitrión **de quien instala** deja de importar.

## 1. El suelo por símbolos

`objdump -T` sobre los nueve `.so` de la imagen recién construida, quedándose con el `GLIBC_*` más
alto que exige cada uno:

| Fichero | `GLIBC_*` máximo | Símbolo(s) que lo imponen |
|---|---|---|
| `librfirma_crypto.so` | **2.34** | `dladdr` `dlopen` `dlsym` `pthread_create` `pthread_join` `pthread_kill` `pthread_key_create` `pthread_setname_np` `pthread_getspecific` `pthread_setspecific` `pthread_mutex_trylock` `pthread_attr_getstack` `pthread_attr_getguardsize` `pthread_attr_setstacksize` `pthread_condattr_setclock` |
| `libawt.so`, `libawt_xawt.so`, `libjavajpeg.so`, `libfontmanager.so`, `liblcms.so` | 2.4 | `__stack_chk_fail` |
| `libawt_headless.so` | 2.2.5 | `malloc`, `dlopen`, `strlen`… |
| `libjava.so`, `libjvm.so` | ninguno versionado | — |

La tabla preliminar del issue queda **confirmada sobre una imagen reconstruida**, con una
corrección: `libfontmanager.so` y `liblcms.so` también piden 2.4, y el issue no los listaba.

### El 2.34 no es una API que podamos evitar

Los quince símbolos del máximo son **todos** de `pthread` y `dl`. Eso no es casualidad: glibc 2.34
absorbió `libpthread` y `libdl` dentro de `libc` y, al hacerlo, **re-versionó toda esa superficie**
a `GLIBC_2.34`. Los símbolos existían desde mucho antes; lo que cambió fue la etiqueta de versión.

De ahí se siguen dos cosas:

- **Bajar el número no es cuestión de dejar de usar algo.** No hay «una API que no usamos» que
  quitar, como el issue planteaba como posibilidad: es la superficie de hilos entera, que un
  runtime de Java necesita por definición.
- **El número lo decide el anfitrión donde se construye**, no el código. Cualquier binario enlazado
  contra una glibc ≥ 2.34 pide esos símbolos con esa versión. Construir sobre una glibc anterior
  bajaría el suelo solo, sin tocar una línea.

### Dependencias externas

`NEEDED` de los ficheros que se cargan en el flujo con rúbrica de imagen:

```
librfirma_crypto.so  libz.so.1 libm.so.6 libc.so.6
libawt.so            libjava.so libjvm.so libm.so.6 libdl.so.2 libc.so.6
libawt_headless.so   libawt.so libjava.so libdl.so.2 libm.so.6 libc.so.6
libjavajpeg.so       libjava.so libc.so.6
libjava.so           librfirma_crypto.so
libjvm.so            librfirma_crypto.so
```

Fuera del sistema, **una sola: `libz.so.1`**. (`libfontmanager.so` y `libawt_xawt.so` sí arrastran
`libfreetype`, `libX11`, `libXext`, `libXi`, `libXrender` y `libXtst`, pero **ninguno de los dos se
carga** en nuestro flujo: no están entre los seis ficheros del #6 y el trazado de abajo lo
confirma.)

## 2. Lo que `objdump` no ve

Los símbolos versionados dan una cota, no la historia entera: quedan fuera los módulos que glibc
abre con `dlopen` en tiempo de ejecución (NSS, `gconv`, locales), la resolución de nombres y el
propio intérprete. Trazando con `LD_DEBUG=libs` la postfirma completa dentro de un Ubuntu 24.04
pelado y bajo `env -i`, la lista de bibliotecas inicializadas es **exactamente**:

```
ld-linux-x86-64.so.2  libc.so.6  libm.so.6  libdl.so.2  libz.so.1
librfirma_crypto.so   libawt.so  libawt_headless.so  libjava.so  libjavajpeg.so  libjvm.so
```

Y filtrando por `nss`, `gconv`, `libnsl` y `resolv`: **cero coincidencias**. No hay carga
perezosa de módulos de glibc, ni resolución de nombres, ni conversión de codificaciones por
`gconv`. La cota por símbolos es, en este caso, la respuesta completa.

## 3. La ejecución

`testbench/run-cross-glibc.sh` ejecuta el ciclo trifásico completo —prefirma nativa, firma PK1
fuera, postfirma nativa— con **rúbrica de imagen** (el caso de seis ficheros del #6) en cada
entorno, bajo `env -i` y sin Java instalado. La firma PK1 y la validación (`pdfsig`, `pdftoppm`) se
hacen en el anfitrión, para no exigir python3 ni poppler dentro de cada contenedor.

| Entorno | glibc | Flujo completo | `pdfsig` | Rúbrica al rasterizar |
|---|---|---|---|---|
| Anfitrión (referencia) | **2.43** | OK, 179.789 B | *Signature is Valid* | Sí |
| Docker `ubuntu:24.04` | **2.39** | OK | *Signature is Valid* | Sí |
| `org.gnome.Platform//49` | **2.42** | OK, 179.789 B | *Signature is Valid* | Sí |
| `org.gnome.Platform//50` | **2.42** | OK, 179.789 B | *Signature is Valid* | Sí |

Un arranque limpio no habría bastado: el flujo elegido es el de rúbrica de imagen justamente
porque es el único que **carga `libawt.so` y atraviesa su `JNI_OnLoad`**, que es donde CE 21 moría
(#12, #13). El rasterizado con `pdftoppm` da 62.872 bytes en los cuatro, o sea la misma página con
la misma rúbrica dibujada.

### Equivalencia bit a bit

Reutilizando **la misma sesión trifásica** del anfitrión (mismo `signed.xml`, mismos
`extraParams`, mismo `TIME`, que es la condición dura del #13) y postfirmando en cada entorno:

| Entorno | vs. anfitrión | `pdfsig` |
|---|---|---|
| Anfitrión | IDÉNTICO | *Signature is Valid* |
| `org.gnome.Platform//49` | **IDÉNTICO** | *Signature is Valid* |
| `org.gnome.Platform//50` | **IDÉNTICO** | *Signature is Valid* |
| `ubuntu:24.04` con `TZ` | **IDÉNTICO** | *Signature is Valid* |

La glibc **no influye en la salida**: el mismo `.so` sobre 2.39, 2.42 y 2.43 escribe los mismos
179.789 bytes.

## 4. Corrección: qué glibc traen los runtimes

Comprobado **dentro** del runtime, no deducido de la versión de GNOME:

| | glibc |
|---|---|
| `org.gnome.Platform//49` | **2.42** |
| `org.gnome.Platform//50` | **2.42** |
| Anfitrión de desarrollo | **2.43** |

Dos correcciones al mapa: el anfitrión **no es 2.42 sino 2.43**, y la 49 trae **la misma glibc que
la 50**, así que elegir entre los dos runtimes no es una decisión de glibc.

Margen resultante: **2.34 de suelo contra 2.42 de runtime, ocho versiones**. No es una
coincidencia frágil.

## 5. Una trampa nueva, y no es de glibc

En la primera pasada, Ubuntu 24.04 producía un PDF **25 bytes más corto** que el anfitrión y
`pdfsig` decía **`Digest Mismatch`**. La causa no es la glibc: es la **zona horaria**.

```
ANFITRIÓN  /M(D:20260831091541+02'00')   ByteRange [0 1194 55196 124593]
CONTENEDOR /M(D:20260831071541Z)         ByteRange [0 1188 55190 124574]
```

El contenedor no lleva base de zonas horarias, así que Java escribe `Z` (17 caracteres) donde el
anfitrión escribe el desfase (25). La fecha vive en el diccionario `/Sig`, dentro del rango
firmado, y ocho caracteres menos corren el `ByteRange` entero. Montándole `/usr/share/zoneinfo` y
pasándole `TZ=Europe/Madrid`, el PDF sale **idéntico bit a bit** y `pdfsig` lo valida.

Esto **añade un tercer elemento** a la restricción que #13 y #14 midieron. La postfirma regenera el
PDF entero y exige de la prefirma no dos cosas sino tres: **los mismos `extraParams`, el mismo
`TIME` y la misma zona horaria**. Y falla igual de callada: firma que se completa sin error y
`pdfsig` que dice `Digest Mismatch` después.

En rfirma las dos fases corren en el mismo proceso, así que no es un riesgo real hoy —pero sí lo
sería en la arquitectura de servidor trifásico del cliente oficial, y conviene que el spec lo diga.
Ojo también con el caso opuesto: dentro de un flatpak sin `--filesystem` sobre la zona horaria, el
comportamiento sería el del contenedor. Aquí no se dio porque `org.gnome.Platform` sí la expone.

## 6. Construir dentro de `org.gnome.Sdk`

**Descartado con datos.** Era la alternativa cara —meter GraalVM y Maven dentro del SDK— y el issue
la condicionaba a que la medición anterior fallara. No falló: el suelo es 2.34, el runtime da 2.42
y la ejecución dentro de los dos runtimes produce el PDF correcto bit a bit. El manifiesto del #22
puede **construir la librería en el anfitrión** y limitarse a fijar el runtime.

## 7. Efecto sobre la niebla

La niebla dice que reabrir **«Instaladores nativos por distribución»** «devuelve la pregunta de
contra qué glibc se construye la librería». Ya está contestada: **2.34**, que es Ubuntu 22.04,
Debian 12 y RHEL 9. Todo lo que sigue con mantenimiento está por encima; lo que queda por debajo
—Ubuntu 20.04 (2.31), Debian 11 (2.31)— ya salió del soporte estándar. La frase puede quitarse.

## Reproducir

```bash
export GRAALVM_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce
rfirma-native-bridge/testbench/build-native-awt.sh ce25-awt awt-config
rfirma-native-bridge/testbench/run-cross-glibc.sh ce25-awt
```

Sin medir, por acotación explícita del esfuerzo: **dónde está el muro exacto**. Las cuatro glibc
probadas están por encima de 2.34, así que confirman el margen pero no localizan el punto de
ruptura. Un `docker run ubuntu:20.04` (2.31) con el mismo guion lo localizaría, si alguna vez hace
falta.
