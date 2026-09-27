# Windows como segunda plataforma: dependencias por target y adaptadores no disponibles

rFirma compila también en Windows (`x86_64-pc-windows-msvc`). Linux sigue siendo la plataforma de
referencia: en Linux no cambian ni el código activo, ni el comportamiento, ni lo que se compila.
Windows entra por fases, y hasta que una fase lo implementa, cada pieza que solo existe en un
escritorio Linux tiene en Windows un **adaptador pendiente**.

## Las dependencias de escritorio Linux van por target

`gtk`, `glib`, `gio` y `oo7` se declaran en `[target.'cfg(target_os = "linux")'.dependencies]`:
sin GTK ni D-Bus no compilan en Windows, y en Linux la resolución es la misma de antes. `libc`
se queda común porque compila en las dos, aunque en Windows no se use.

## OpenSSL: compilado desde las fuentes, y solo en Windows

El crate `openssl` es común (la CA local y la lectura de certificados lo usan en `domain/`), y
Windows no trae OpenSSL. En `[target.'cfg(windows)'.dependencies]` se le añade la
característica `vendored`: `openssl-src` compila OpenSSL y lo enlaza estático, así que el
binario no arrastra ninguna DLL más. En Linux esa característica no se activa y se sigue usando
el OpenSSL del sistema.

Compilarlo exige un **Perl nativo de Windows** (Strawberry Perl, que traen los runners de Windows
de GitHub) y las Build Tools de Visual Studio. El Perl de Git for Windows **no sirve**: es de
Cygwin, le faltan módulos y el `Configure` de OpenSSL rechaza sus rutas para `VC-WIN64A`.

El `Cargo.lock` es el mismo para todas las plataformas, así que gana la entrada de
`openssl-src`, y `packaging/flatpak/cargo-sources.json` con ella: el flatpak la descarga para
resolver el grafo, pero no la compila.

## Los condicionales de sistema viven en una lista cerrada

El ADR-0010 dejaba todo el conocimiento del sistema en `paths.rs`. Con Windows ya no cabe en un
fichero, pero sigue sin repartirse: la guarda `tests/single_cfg_os_site.rs` enumera en
`AUTHORISED_SITES` los únicos ficheros con un `cfg` de sistema operativo, y exige que cada uno
lo lleve de verdad. Un fichero nuevo en la lista se justifica en la PR que lo añade.

Donde la diferencia es solo de tipos, no se pone condicional: el parámetro de PSS es un
`c_ulong`, que en Linux es el mismo `u64` de siempre.

## Los adaptadores pendientes dicen su nombre

Un adaptador pendiente respeta el puerto y falla como fallaría Linux sin el servicio, y su nombre
dice qué falta:

| Pieza en Linux | Pendiente en Windows | Qué hace mientras |
| --- | --- | --- |
| Diálogo GTK del PIN | `PendingWindowsPinDialog` | Falla con un mensaje en castellano. |
| Llavero `oo7` (ADR-0034) | `PendingWindowsCredentialManager` | `NoKeyring`: no hay instalación. |
| Manejadores de `afirma://` por GIO | `handlers_in_the_windows_registry_pending` | Ninguno registrado. |
| Diálogo GTK de fallo de arranque | `show_windows_dialog_pending` | Solo `stderr`. |
| `mlock` del secreto | `virtual_lock_on_windows_pending` | Sin bloqueo en RAM. |
| Cerrojo de `.parentlock` de Firefox | `firefox_parent_lock_on_windows_pending` | Firefox nunca «abierto». |

El cerrojo de la carpeta de paso (ADR-0024) no queda pendiente: en Windows es abrir el fichero
sin compartirlo, que cumple lo mismo que `flock`. Los adaptadores de NSS y de p11-kit compilan
sin cambios. En Windows no encuentran sus bibliotecas y fallan en tiempo de ejecución.

Las pruebas intrínsecamente de Unix (enlaces simbólicos, `fork`, `flock`, `OsString` no UTF-8)
se marcan con `cfg(unix)`, no se borran.

## Las recetas corren en Git Bash

El `justfile` es el mismo en las dos plataformas. En Windows sus recetas corren en el `bash.exe`
de Git for Windows, fijado por ruta absoluta en `windows-shell`: buscado por el `PATH`, Windows
encontraría antes el `bash.exe` de WSL. Las recetas con shebang necesitan además `cygpath`, así
que `just` se lanza desde Git Bash, no desde PowerShell ni desde cmd.

La raíz del repositorio se toma con barras normales (`root`), porque `bash` se come las
invertidas de `justfile_directory()`. GraalVM se busca en `GRAALVM_HOME` y, en Windows, en
`JAVA_HOME`, que es donde lo deja su instalador; en Linux sigue la ruta de SDKMAN. `just tools`
comprueba en cada sistema lo suyo: en Windows, `cl.exe` por `vswhere`, el runtime de WebView2 y
un Perl nativo para OpenSSL, y no pide ni las bibliotecas del WebView de Linux, ni softhsm, ni
NSS; gettext pasa a aviso porque solo lo usan `just po` y el carril del CI.

## La biblioteca nativa se llama como manda la plataforma

`librfirma_crypto.so` en Linux y `rfirma_crypto.dll` en Windows. Rust compone el nombre con
`DLL_PREFIX` y `DLL_SUFFIX` de `std` (`library_file`), sin `cfg`; el `justfile` hace lo mismo
en `native_lib_name`. Se busca en los mismos sitios del ADR-0004: `RFIRMA_LIB_DIR` y
`../lib/rfirma` junto al ejecutable.

`native-image` sigue construyendo con las banderas de `native-image.properties`, que no cambian,
así que en Windows emite `librfirma_crypto.dll`: `just native` la instala renombrada. El nombre
del fichero no lo lee nadie dentro de la imagen, y los trece símbolos que resuelve Rust salen
exportados igual. Como en Linux, emite al lado los auxiliares de AWT (`awt.dll`, `java.dll`,
`jvm.dll`, `lcms.dll`…), la `.lib` de importación y las cabeceras, y no se copia ninguno: la
invariante de un solo fichero del ADR-0012 vale igual. La `.dll` solo importa bibliotecas del
sistema y el runtime de Visual C++.

## Considered Options

**OpenSSL del sistema en Windows** (`OPENSSL_DIR`, vcpkg): deja el `Cargo.lock` intacto, pero
obliga a cada equipo y al CI a instalarlo, y a distribuir sus DLL junto al binario. Se descartó
porque la construcción deja de ser reproducible con solo `cargo`.

**Sustituir `openssl` por crates de Rust puro**: reescribe código que en Linux funciona. Queda
fuera de una fase que solo busca compilar.

**Pasar `-o rfirma_crypto` a `native-image` en Windows**: evita renombrar, pero devuelve banderas
sueltas al `justfile`, y `native-image.properties` es el único sitio de las banderas de la imagen.

**Recetas duplicadas con `[windows]` y `[linux]`**, o PowerShell como shell de Windows: dos
versiones de cada receta que envejecen por separado. Git Bash ya está en cualquier equipo que
tenga Git, y los scripts de `scripts/` son de `bash`.

**Un alias de `cfg` desde `build.rs`** (`cfg(gtk_desktop)`) para no tocar la guarda: esconde el
sistema operativo tras un nombre que la guarda no ve. Se descartó por eso mismo.

## Consequences

- `tauri-build` exige en Windows `icons/icon.ico`; se empaqueta con los PNG que ya existían.
- `tauri-build` solo pone el manifiesto de Common Controls v6 al binario, y los ejecutables de
  prueba que enlazan Tauri mueren al arrancar con `STATUS_ENTRYPOINT_NOT_FOUND`. En Windows,
  `build.rs` enlaza `windows-app-manifest.xml` en todo lo que se enlaza, y por eso está en
  `AUTHORISED_SITES`.
- `just tools`, `just bootstrap`, `just native`, `just dev` y `just fmt` funcionan en Windows;
  el resto de recetas (`check`, `flatpak`, `bundle`, `certs`…) sigue siendo de Linux.
- En Windows la aplicación busca la `.dll` en `../lib/rfirma` junto al ejecutable, igual que en
  Linux; dónde la deje un instalador de Windows se decide con el instalador.
- `cargo test` en Windows no corre las pruebas de grada B y C que necesitan softhsm o NSS.
