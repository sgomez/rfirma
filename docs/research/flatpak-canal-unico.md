# Flatpak como único canal: medición

Medición para el issue [#22](https://github.com/sgomez/rfirma/issues/22). El
[#17](https://github.com/sgomez/rfirma/issues/17) eligió flatpak sobre indicios, sin construir
nada; el [#23](https://github.com/sgomez/rfirma/issues/23) desactivó el riesgo de la glibc. Aquí se
construye el flatpak de verdad y se comprueba de punta a punta.

Entorno: Ubuntu 26.04 (glibc 2.43), flatpak 1.16.6, flatpak-builder 1.4.8, `org.gnome.Sdk` 49 y 50,
`org.freedesktop.Sdk.Extension.rust-stable//25.08`, sesión GNOME 50 sobre **Wayland** con NVIDIA
580. Imagen nativa `ce25-awt` del #23 (GraalVM CE 25, 35.326.048 bytes). Token SoftHSM `rfirma-test`
del [#5](https://github.com/sgomez/rfirma/issues/5).

Lo construido vive en `packaging/flatpak/`: el manifiesto `me.sgomez.rfirma.yml`, una
**sonda** Tauri desechable (`probe/`) que hace de aplicación mientras rfirma no existe, y
`verifica.sh`, que reconstruye y vuelve a comprobar todo de una pasada.

## Veredicto

**Flatpak sirve, y el ciclo trifásico completo con rúbrica de imagen firma y valida dentro del
arenero.** `pdfsig` dice *Signature is Valid* sobre un PDF de 179.813 bytes firmado con el
certificado del token, con la rúbrica dibujada. Es además la **primera vez** que la fase 2 la hace
PKCS#11 de verdad desde Rust (`cryptoki`, `CKM_SHA256_RSA_PKCS`) y no el `openssl` del banco de
pruebas.

Tres hallazgos cambian decisiones:

1. **El módulo PKCS#11 lo aporta el flatpak.** Los del anfitrión no cargan dentro y `p11-kit` solo
   proyecta el almacén de confianza, nunca un token de firma.
2. **«Guardar junto al documento original» no se puede implementar** bajo el arenero: la aplicación
   no puede saber dónde estaba el original, y el portal lo prohíbe explícitamente. Sustituido por
   `--filesystem=xdg-documents` en el
   [ADR-0011](../adr/0011-destino-del-documento-firmado.md).
3. **La ventana muere al primer fotograma** en Wayland con el GTK3 del runtime, en 49 y en 50, y se
   arregla con una variable de entorno que el manifiesto declara.

## 1. Los seis `.so` y `$ORIGIN`

Instalados con `install -Dm644` en `/app/lib/rfirma/`. La sonda los busca **relativa al
ejecutable** —`/app/bin/rfirma-probe` → `../lib/rfirma`— y no por una constante absoluta, que es la
restricción 1 del issue:

```
ejecutable      : /app/bin/rfirma-probe
dir libreria    : /app/lib/rfirma
los seis .so    : los seis presentes
dlopen          : OK (/app/lib/rfirma/librfirma_crypto.so)
```

Los cinco auxiliares se resuelven solos por `$ORIGIN`, igual que fuera del arenero: **ni
`LD_LIBRARY_PATH` ni `RPATH` ni CWD**. El razonamiento del ADR-0004 —los seis conviven en un
directorio que gestiona un empaquetador— se cumple igual con flatpak que con el `.deb`.

`RFIRMA_LIB_DIR` sigue sobrescribiendo la ruta para desarrollar contra `target/`.

## 2. La glibc

Confirmado dentro del arenero: `gnu_get_libc_version` devuelve **2.42**, contra el suelo de
**2.34** que midió el #23. La glibc del anfitrión de quien instala deja de intervenir. Nada nuevo;
solo la comprobación de que el número que se ejecuta es el del runtime.

## 3. El módulo PKCS#11: lo aporta el flatpak

Esta es la decisión que el `.deb` no tenía. Se probaron los tres caminos:

| Camino | Resultado |
|---|---|
| `p11-kit-client.so` del runtime | **No sirve.** Solo ve `System Trust` |
| Módulo del anfitrión vía `--filesystem=host-os:ro` | **Frágil.** `opensc-pkcs11.so` no carga |
| **Empaquetar `opensc` en el flatpak** | **Elegido.** Carga e inicializa dentro |

### p11-kit solo proyecta la confianza

Flatpak arranca por su cuenta un `p11-kit server` para cada arenero y lo publica en
`/run/user/1000/p11-kit/pkcs11`. Aparece sin declarar ningún permiso, lo que invita a pensar que es
la puerta a los módulos del usuario. No lo es: el proceso del anfitrión es

```
p11-kit server --sh -n /run/user/1000/.flatpak-helper/pkcs11-flatpak-… \
               --provider p11-kit-trust.so pkcs11:model=p11-kit-trust?write-protected=yes
```

o sea **un solo proveedor, el almacén de CA**. `p11-kit list-modules` dentro del arenero lista un
único módulo, `System Trust`, de solo lectura. Los `opensc-pkcs11` y `softhsm2` que el anfitrión sí
tiene registrados en `/usr/share/p11-kit/modules/` no cruzan.

### Los módulos del anfitrión no cargan

Con `--filesystem=host-os:ro` los ficheros se ven en `/run/host/usr/lib/…`, pero cargarlos es otra
cosa. El enlazador los resuelve contra el **runtime**, no contra el anfitrión:

| Módulo del anfitrión | `NEEDED` sin resolver dentro |
|---|---|
| `libsofthsm2.so` | ninguno (`libcrypto.so.3` y `libstdc++.so.6` los tiene el runtime) |
| `opensc-pkcs11.so` | **`libopensc.so.13`, `libeac.so.3`** |

`libsofthsm2.so` carga y firma —así se ejecuta el banco de pruebas—, pero es suerte: sus
dependencias coinciden con las del runtime. `opensc-pkcs11.so`, que es el módulo real de un DNIe,
**no carga**. Apuntar `LD_LIBRARY_PATH` a `/run/host/usr/lib/x86_64-linux-gnu` lo resuelve todo,
pero al precio de meter en el proceso librerías enlazadas contra la glibc 2.43 del anfitrión dentro
de un runtime de 2.42. Eso es exactamente la clase de rotura que el arenero existe para evitar.

### Lo que se empaqueta

Dos módulos nuevos en el manifiesto:

- **pcsc-lite 2.5.1** (meson), solo la librería cliente: el demonio `pcscd` sigue en el anfitrión.
  El runtime no la trae. `-Dipcdir=/run/pcscd`, que es donde `--socket=pcsc` monta el socket.
- **OpenSC 0.27.1**, que aporta `/app/lib/pkcs11/opensc-pkcs11.so`.

Dentro del arenero, sin `--filesystem` de ninguna clase, `opensc-pkcs11.so` **carga, inicializa y
enumera ranuras**; devuelve cero porque esta máquina no tiene lector. `--socket=pcsc` deja el
socket del anfitrión a la vista:

```
srw-rw-rw- 1 nfsnobody nfsnobody 0 /run/pcscd/pcscd.comm
```

Dos avisos honestos. **No hay lector ni tarjeta**, así que el camino
`opensc → libpcsclite → pcscd` no se ha ejercitado: con y sin `--socket=pcsc` la respuesta es la
misma, «No smart card readers found», igual que en el anfitrión. Y quien tenga un módulo propio del
fabricante instalado en el anfitrión necesitará un `flatpak override --filesystem=…`: es la vía de
escape, no el camino por omisión.

## 4. Ficheros y portales

Los portales responden desde dentro sin declarar nada:

```
portal FileChooser: (<uint32 4>,)
portal Documents  : (b'/run/user/1000/doc',)
```

Lo interesante es qué forma tiene lo que devuelven. Exportando un fichero como haría el diálogo de
abrir (`flatpak document-export --app=me.sgomez.rfirma --allow-write …/docs/original.pdf`),
la aplicación lo ve así:

```
ruta dentro del arenero: /run/user/1000/doc/1e8b83b9/original.pdf
directorio padre       : /run/user/1000/doc/1e8b83b9
contenido del padre    : original.pdf
```

**El directorio padre contiene un solo fichero**, y no es el del usuario. De ahí salen dos hechos:

- Escribir un hermano (`original-firmado.pdf`) **parece funcionar** y deja en el directorio real
  del usuario un fichero oculto `.xdp-original-firmado.pdf-5OUkyi` que nunca se renombra. La
  operación no da error y el resultado no aparece por ninguna parte: el fallo silencioso otra vez.
- La aplicación **no puede averiguar la ruta original**. `org.freedesktop.portal.Documents.Info` y
  `.Lookup` contestan `org.freedesktop.portal.Error.NotAllowed: Not allowed in sandbox`.

Consecuencia directa: **«junto al documento original», que es el valor por omisión de
[`docs/design/preferencias.md`](../design/preferencias.md), no es implementable** bajo el arenero.
Ni escribiendo al lado, ni preseleccionando esa carpeta en el diálogo de guardar, porque la
aplicación no sabe cuál es. Lo mismo alcanza a la degradación del
[ADR-0010](../adr/0010-memoria-entre-sesiones.md), que usa «junto al original» como recurso cuando
la carpeta fija falla.

**Resuelto en el [#27](https://github.com/sgomez/rfirma/issues/27)**, que lo sustituye por
`--filesystem=xdg-documents` y deja el recorrido sin diálogo por firma: el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md). Ahí se mide además que
`--filesystem=home` **tampoco** habría devuelto la ruta real —el portal solo la da a un llamante
`is_host`— y que escribir en una carpeta declarada que **no existe en el anfitrión** contesta OK y
no deja nada, la misma trampa de arriba con otro disfraz.

El **diálogo de guardar** sí resuelve el caso general: el usuario elige destino y el portal concede
el permiso justo para ese fichero. La sonda copia el PDF firmado a la ruta devuelta y funciona.

Nota aparte: `/etc/localtime` **sí** apunta dentro del arenero a `Europe/Madrid`. La trampa de la
zona horaria que el #23 encontró en Docker no se da aquí.

## 5. WebKitGTK: renderiza, pero hay que sujetarla

La sonda avisa desde el propio *webview*, así que la traza demuestra que WebKitGTK cargó, pintó y
ejecutó JavaScript dentro del arenero:

```
WEBVIEW OK  900x593  userAgent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 … Version/60.5
  cargada: /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so.0.21.9
  cargada: /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.1.so.0.10.13
```

Es la del runtime, la 4.1 contra la que enlaza Tauri v2, y no arrastra nada del anfitrión.

**Pero el proceso moría a los pocos segundos.** Con `WAYLAND_DEBUG=1` la causa es exacta:

```
-> wp_linux_drm_syncobj_manager_v1#48.get_surface(…, wl_surface#36)
-> wl_surface#36.attach(wl_buffer#51, 0, 0)
-> wl_surface#36.commit()
<- wl_display#1.error(wp_linux_drm_syncobj_surface_v1#53, 2,
                      "Explicit Sync only supported on dmabuf buffers")
Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.
```

GTK3 negocia sincronización explícita y luego *commitea* un búfer que no es dmabuf; Mutter corta la
conexión. Medido en tres configuraciones:

| Configuración | Resultado |
|---|---|
| Wayland, `--device=dri` | **muere** (Error 71) |
| Wayland, `--device=dri`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` | **vive** |
| Wayland, sin `--device=dri` | vive, con `libEGL warning: failed to create dri2 screen` |
| XWayland (`--socket=x11`, `GDK_BACKEND=x11`) | vive, con avisos `Failed to create GBM buffer` |

**`org.gnome.Platform` 49 y 50 se comportan idénticamente**: misma `libwebkit2gtk-4.1.so.0.21.9`,
mismo fallo, mismo arreglo. El manifiesto declara
`--env=WEBKIT_DISABLE_COMPOSITING_MODE=1`, que conserva la aceleración del resto y desactiva solo la
ruta que rompe. Es un paliativo sobre esta máquina (NVIDIA + Mutter 50): conviene reevaluarlo
cuando el runtime traiga GTK/WebKit más nuevos, y la sonda vale para eso.

Aviso: `--socket=fallback-x11` **no** da X11 mientras haya Wayland, así que forzar `GDK_BACKEND=x11`
sin `--socket=x11` hace fallar la inicialización de GTK con «Failed to initialize GTK».

### 5.1. El proxy que no se puede preguntar (v0.1)

La v0.1 se entregó con la ventana abriendo **con un error de DBus pintado a pantalla completa**:

```
GDBus.Error:org.freedesktop.portal.Error.NotAllowed: This call is not available inside the sandbox
```

El método rechazado se caza con `flatpak run --log-session-bus`:

```
C22: -> :1.66 call org.freedesktop.portal.ProxyResolver.Lookup at /org/freedesktop/portal/desktop
B5215: <- :1.66 return error org.freedesktop.portal.Error.NotAllowed from C22
```

Dos rechazos por arranque, uno del proceso de red de WebKit y otro del proceso web. **El portal solo
concede `ProxyResolver` a una aplicación con red**, y rfirma no la declara ni debe declararla. GLib
no tiene plan B: `GProxyResolverPortal` es el único resolvedor dentro del arenero, así que la carga
no llega a empezar y WebKit pinta el error del resolvedor como página.

Se corrige declarando el resolvedor nulo en `finish-args`:

```yaml
  - --env=GIO_USE_PROXY_RESOLVER=dummy
```

Con él, las dos llamadas desaparecen del bus (338 → 324 líneas de traza, ninguna `NotAllowed`). La
alternativa —`--share=network`— **no se toma**: rfirma firma sin salir a la red, y abrir el permiso
para callar un aviso de proxy es pagar el canal entero por un `Lookup`.

**Por qué la sonda del #22 no lo vio, teniendo los mismos permisos.** La sonda cargaba su HTML
directamente en el *webview* y por eso imprimió `WEBVIEW OK`; la aplicación de verdad sirve el
frontal por el protocolo propio de Tauri, y **ese** sí pasa por el resolvedor de proxy. Es el mismo
patrón que ya costó tres hallazgos en este proyecto (ADR-0013): la sonda y lo que se distribuye no
ejercitan el mismo camino.

**Y por qué `verifica.sh` decía OK.** Su paso 4 solo miraba que el proceso siguiera vivo a los diez
segundos, y lo estaba: una ventana viva no es una ventana que se vea. Desde este hallazgo el paso 4
corre con `--log-session-bus` y falla si el arenero rechaza cualquier llamada al portal.

### 5.2. El binario apuntaba al servidor de vite (v0.1)

Detrás del proxy había un segundo fallo, y este era el gordo: quitada la página de error del
resolvedor, la ventana abría con

```
Could not connect to localhost: Connection refused
```

El binario empaquetado **no llevaba el frontal dentro**. Comprobado sobre el `.flatpak` instalado:

```
$ strings -a …/files/bin/rfirma | grep -c "index-A5JvJEiO"   # el bundle de vite
0
$ strings -a …/files/bin/rfirma | grep "http://localhost:1420"
http://localhost:1420/
```

La causa está en el `build.rs` del crate `tauri` (2.11.5), y es de una literalidad incómoda:

```rust
let custom_protocol = has_feature("custom-protocol");
let dev = !custom_protocol;
```

**El modo dev de Tauri no se deduce del perfil de cargo: es la ausencia de una bandera.** Un
`cargo build --release` sin `--features custom-protocol` compila sin una queja y produce un binario
que sirve `devUrl` en vez del frontal empotrado. Quien normalmente activa la bandera es
`cargo tauri build`, y este repositorio **no lo usa a propósito** (ID-05: `bundle.active` es false y
el binario lo instala el manifiesto). Al saltarse el CLI se perdió lo único que el CLI aportaba.

Medido en el anfitrión, con el mismo `dist`:

| Construcción | Tamaño | `/assets/index-*.js` dentro |
|---|---|---|
| `cargo build --release` | 7,56 MB | no |
| `cargo build --release --features custom-protocol` | 8,09 MB | sí |

Media MB de diferencia: es el frontal. La bandera se declara en `src-tauri/Cargo.toml` **sin ser
`default`** —si lo fuera, `tauri dev` serviría los assets empotrados y moriría el recargado en
caliente— y la pasan a mano los dos sitios que compilan sin el CLI: la receta `build-rust` del
`justfile` y el módulo `rfirma-app` del manifiesto.

**La guarda.** El manifiesto saca del `dist/index.html` el nombre del bundle de vite y comprueba con
un `grep` que está dentro del binario recién compilado. Falla al construir, que es donde sale
barato, en vez de producir un bundle que abre contra un puerto que en el arenero no escucha nadie.

## 6. La prueba final

El listón del [#14](https://github.com/sgomez/rfirma/issues/14), ahora dentro del arenero y con
firma de token:

```
certificado     : cert-fnmt.b64
PDF de entrada  : /app/share/rfirma-probe/test.pdf (134813 bytes)
prefirma        : OK (1007 bytes de TriphaseData)
firma PKCS#11   : OK (PRE 513 bytes DER -> PK1 256 bytes)
postfirma       : OK (179813 bytes)

pdfsig:  Signer Certificate Common Name: EIDAS CERTIFICADO PRUEBAS - 99999999R
         Signature Validation: Signature is Valid.
rasterizado (pdftoppm -r 50): 62872 bytes
```

Los 62.872 bytes de la página rasterizada son **los mismos** que midió el #23 en el anfitrión y en
los dos runtimes: la misma página con la misma rúbrica dibujada.

La primera pasada dio *Signature is Invalid*, y por una razón que conviene recordar: se firmaba con
la clave del token y se incrustaba el certificado de prueba del banco. **El certificado tiene que
ser el del token**; obvio dicho así, silencioso al medirlo.

## 7. El manifiesto

| Decisión | Valor | Por qué |
|---|---|---|
| `app-id` | `me.sgomez.rfirma` | DNS inverso de `rfirma.sgomez.me`, dominio propio. Ver abajo |
| Runtime | `org.gnome.Platform//50` | 49 y 50 miden igual; se elige la nueva |
| Rama | `stable` | La versión va en `<releases>` del metainfo, no en la rama |
| Ventana | `--socket=wayland --socket=fallback-x11 --share=ipc --device=dri` | |
| Compositing | `--env=WEBKIT_DISABLE_COMPOSITING_MODE=1` | Sección 5 |
| Tarjetas | `--socket=pcsc` | El `pcscd` del anfitrión |
| Ficheros | `xdg-documents`, `~/.mozilla/firefox:ro`, `~/.pki/nssdb:ro` | Documentos por portal; los dos `:ro` son la excepción del #95/#101 para NSS. Ni `home` ni `host` |
| Módulo PKCS#11 | empaquetado (OpenSC 0.27.1 + pcsc-lite 2.5.1) | Sección 3 |
| Arquitectura | `x86_64` sola en v0.1 | No hay imagen nativa de aarch64 y no se ha medido |
| Distribución | Flathub como destino; `flatpak build-bundle` para probar | |

Tamaños medidos: **57 MB** en `/app` (35 MB los seis `.so`, 7,6 MB el binario de la sonda, 2,4 MB
OpenSC y pcsc-lite; el manifiesto descarta veinte de las veintidós herramientas de línea de órdenes
de OpenSC y conserva `opensc-tool` y `pkcs11-tool` para diagnosticar), **47,5 MB** ya instalado, y
**13.449.368 bytes** el bundle de un fichero. El bundle **no lleva el runtime**: quien lo instale se
descarga `org.gnome.Platform//50` aparte, unos 830 MB la primera vez.

El `app-id` sale del **dominio propio** y no de `io.github.sgomez`, que era la otra opción. El id es
lo más caro de cambiar después —es la carpeta de datos `~/.var/app/<id>/`, el nombre D-Bus, los
permisos de portales ya concedidos y la entrada de menú, y renombrar obliga a un
`end-of-life-rebase` con usuarios de por medio— así que se ata a lo que menos probable es que
cambie. Un id `io.github.*` ata la aplicación a la forja y miente el día que el repositorio se
mueva. Flathub verifica las dos formas: la del dominio, con un TXT en DNS o un fichero en
`.well-known`.

Se elige **Flathub** y no un repositorio propio: el motivo del #17 para abandonar el `.deb` era que
lo usara gente de otras distribuciones, y un repositorio propio devuelve la fricción que flatpak
venía a quitar. El bundle queda como vehículo de pruebas previas, no como canal.

## 8. Lo que no se ha medido

- **Un lector y una tarjeta reales.** Sin hardware, `opensc → pcscd` queda cargado pero no
  ejercitado.
- **Construir sin red**, que es regla de Flathub. El módulo `sonda` declara hoy
  `--share=network` para bajar las dependencias de cargo. Publicar en Flathub obliga a vendorizarlas
  (`flatpak-cargo-generator.py` → `cargo-sources.json`). No es un riesgo, es trabajo del spec.
- **aarch64.**
- **El diálogo del portal completado a mano.** Se comprueba que la petición se lanza y que los
  portales responden; empujar el botón «Abrir» de un diálogo del anfitrión no es automatizable en
  esta sesión de Wayland. El camino de los ficheros se midió por el portal de documentos, que es lo
  que el diálogo acaba entregando.

## 9. La entrada de documentos, dentro del arenero (#89)

El apartado 4 dejaba pendiente comprobar, ya con la aplicación de verdad y no la sonda, que los
bytes que el portal concede llegan a lo que la Orden 8 (`read_document`, `commands/mod.rs`) lee del
disco, y si el permiso que apunta contra la ruta del anfitrión sobrevive a cerrar y reabrir la
aplicación (ID-72). El paso 6 de `verifica.sh` mide las dos cosas.

**Los bytes llegan intactos.** `flatpak document-export --app=me.sgomez.rfirma <fichero>` es la
misma vía por la que el diálogo de la Orden 7 (`open_document`) concede el permiso: contra la ruta
del anfitrión, no el inodo, y devuelve la ruta montada dentro del arenero. Un `flatpak run
--command=sha256sum` sobre esa ruta, dentro del arenero, da el mismo hash que el fichero original en
el anfitrión.

**El identificador sobrevive a cerrar y reabrir la aplicación.** Se mató el proceso (`flatpak kill`)
y se volvió a pedir el permiso para el **mismo** fichero del anfitrión, como haría quien elige otra
vez el mismo PDF en el diálogo tras reabrir rfirma: el portal devuelve la **misma** ruta montada
(mismo identificador de documento), y sus bytes se siguen leyendo igual. El permiso vive en el
almacén del propio portal de documentos, no en el proceso de la aplicación, así que no depende de
que rfirma siga viva.

**Consecuencia para los recientes.** Hoy `RecentDocument` identifica una fila por la ruta canónica
del anfitrión (`memory/recents.rs`), que bajo el arenero la aplicación no puede leer del documento
del portal (`PortalDocument` no expone ninguna, apartado 4 de arriba). El día que la bandeja
persista entre sesiones bajo flatpak tendrá que identificar la fila por la ruta montada del portal
en vez de por la ruta real —esta medición dice que hacerlo funciona: volver a abrir el mismo host
path en una sesión posterior devuelve el mismo identificador y las mismas lecturas—, no es trabajo
de este ticket.

Ningún permiso nuevo hace falta para esta medición: `flatpak document-export` y la lectura dentro
del arenero solo usan el portal de documentos y el `--filesystem="$LAB"` que el banco de pruebas ya
declaraba para el resto de pasos.

## Reproducir

```bash
export GRAALVM_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce
rfirma-native-bridge/testbench/build-native-awt.sh ce25-awt awt-config
packaging/flatpak/verifica.sh
```

`verifica.sh` construye e instala el flatpak, imprime el informe del arenero, ejecuta el ciclo
trifásico completo firmando con el token de pruebas, valida con `pdfsig`, arranca la ventana y la
deja diez segundos, comprueba que un documento entrado por el portal llega con sus bytes intactos
dentro del arenero, y empaqueta el bundle.
