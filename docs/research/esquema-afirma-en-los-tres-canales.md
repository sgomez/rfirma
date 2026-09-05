# Cómo llega la URL `afirma://` hasta rfirma en los tres canales

Sondeo del [#313](https://github.com/sgomez/rfirma/issues/313), parte del mapa
[#308](https://github.com/sgomez/rfirma/issues/308). Contesta las cinco preguntas del ticket para
**flatpak**, **`.deb`** y **`.rpm`**.

Todo lo que aquí se afirma está o **medido en esta máquina** (Ubuntu con `dpkg`, Flatpak 1.16.6,
AutoFirma 1.9 instalada por `.deb`) o **leído en la fuente que lo decide** (el código de
`tauri-bundler`, el de los complementos de Tauri en la versión exacta que fija el `Cargo.lock`, el
instalador de `clienteafirma`, y las especificaciones de freedesktop). Lo que no se ha podido
comprobar va marcado.

---

## Resumen: las cinco respuestas en una línea cada una

1. **Qué declarar**: `MimeType=x-scheme-handler/afirma;` **y `%u` en el `Exec=`**. Las dos cosas, en
   los tres canales, **a mano**: el flatpak porque su `.desktop` está escrito a mano, y el `.deb` y
   el `.rpm` porque `desktopTemplate` sustituye la plantilla del *bundler* y con ella el `%u` que
   el *bundler* tampoco pone.
2. **Cómo llega**: como **argumento de línea de órdenes**, en los tres. Nadie es `DBusActivatable`
   hoy y no conviene serlo.
3. **Instancia única**: sí, la URL entera llega al *callback* de `lib.rs:87` en `command_line[1]`,
   también dentro del flatpak —medido— y **sin `finish-args` nuevos**. Pero hoy `invoked_pdf` la
   trataría como una ruta y abriría la ventana de «esto no es un PDF».
4. **Con AutoFirma instalada**: es determinista y **hoy gana AutoFirma en el `.deb`/`.rpm`**
   (orden alfabético de `mimeinfo.cache`: `afirma.desktop` antes que `rfirma.desktop`) y **gana
   rfirma en el flatpak** (los *exports* de flatpak van antes que `/usr/share` en `XDG_DATA_DIRS`).
   En Fedora y openSUSE gana AutoFirma pase lo que pase, porque su `%post` se declara
   predeterminada a la brava en `mimeapps.list`. La persona puede elegir siempre.
5. **Reversibilidad**: sí en los tres, porque el registro vive en el `.desktop` y el `.desktop` lo
   borra el gestor de paquetes. El residuo posible es la elección explícita del usuario en
   `~/.config/mimeapps.list`, que no la limpia nadie —y AutoFirma **sí** deja residuo en Fedora y
   openSUSE si se actualiza en vez de desinstalarse.

**Lo que este sondeo cambia respecto de lo que creíamos**: el ADR-0005 dice que «el flatpak sí puede
registrar `x-scheme-handler/afirma` exportando su `.desktop`; el nuestro simplemente no lo declara
todavía». Es cierto pero se queda corto: **declarar el `MimeType` no basta en ninguno de los tres
canales**. Sin `%u` el escritorio lanza el proceso *sin* la URL, y la conversación con la sede no
llega a empezar. Ese `%u` es la pieza que no está en ninguna parte del repositorio y que no aparece
sola en ninguno de los tres canales.

---

## 0. De dónde sale la URL y qué se espera de ella

Del contrato ya medido en
[`contrato-protocolo-afirma.md`](contrato-protocolo-afirma.md): la única URL de esquema `afirma://`
que llega al sistema operativo es la de arranque,

```
afirma://websocket?ports=<p1,p2,p3>&v=4&jvc=4&idsession=<20 chars>[&dlgload=false]
```

y su única función es **lanzar el proceso**. Es un argumento de arranque, no una operación. A
partir de ahí todo viaja por el `wss://` local. Eso fija el listón de este sondeo: basta con que la
cadena llegue **entera y una sola vez** al proceso que ya corre o al que se acaba de abrir.

---

## 1. Qué hay que declarar, y quién lo genera en cada canal

### Lo normativo

La *Desktop Entry Specification* asocia tipos a una aplicación con la clave `MimeType`
(«The MIME type(s) supported by this application»), y avisa en su sección de registro de tipos de
que ahí **no hay prioridad ninguna**: «Priority for applications is handled external to the .desktop
files». El `.desktop` declara **capacidad**, nunca preferencia; `mimeapps.list` es lo que declara
preferencia (punto 4).

Conviene saber una cosa que sorprende: **`x-scheme-handler/<esquema>` no aparece en ninguna
especificación de freedesktop.** No está en la *Shared MIME-info Database*, ni en la *Desktop Entry
Spec*, ni en mime-apps-spec. Es una **convención de GIO**, y su definición real es una línea de
`gio/gdesktopappinfo.c`:

```c
scheme_down = g_ascii_strdown (uri_scheme, -1);
content_type = g_strdup_printf ("x-scheme-handler/%s", scheme_down);
```

(en `g_app_info_get_default_for_uri_scheme_impl`). De ahí en adelante es un tipo MIME como
cualquier otro. Dos consecuencias: **el esquema se pasa a minúsculas** —`afirma` ya lo es— y todo el
algoritmo de mime-apps-spec le aplica entero. Que sea convención y no norma no lo hace frágil —
`xdg-mime`, GNOME y KDE la comparten, y AutoFirma la usa desde hace años— pero sí explica por qué no
hay un documento al que apelar.

La segunda mitad es el **código de campo** del `Exec=`. La especificación es explícita: el lanzador
sustituye `%u` por una URI (`%U` por varias), sólo puede haber **como mucho uno** por línea de
órdenes, y **si `Exec=` no lleva ninguno, no se pasa nada**. Un `.desktop` con
`MimeType=x-scheme-handler/afirma;` y `Exec=rfirma` registra el esquema correctamente y luego
arranca rfirma **sin decirle a qué**.

Y tiene que ser `%u`, no `%f`. La especificación define `%f` como «used for programs that do not
understand the URL syntax» y obliga al lanzador a **descargar** lo que no esté en el sistema de
ficheros local a un temporal. Con un esquema propio eso no significa nada.

La tercera pieza es `mimeinfo.cache`, que escribe `update-desktop-database` sobre un directorio de
`applications/`. No está en ninguna especificación —su propia página de manual dice que «the order
of the desktop files found for a MIME type is not significant»—, pero en la práctica **es
obligatoria**: GIO, y por tanto GTK, GNOME y el `xdg-open` de la mayoría de escritorios, resuelve
por ella. Medido sobre un directorio limpio con un solo `.desktop`:

```console
$ gio mime x-scheme-handler/afirma          # sin mimeinfo.cache
No default applications for “x-scheme-handler/afirma”
$ update-desktop-database …/applications
$ gio mime x-scheme-handler/afirma          # con mimeinfo.cache
Default application for “x-scheme-handler/afirma”: probe.desktop
```

**Sin la caché, el `MimeType` del `.desktop` no lo ve nadie.**

### Flatpak: a mano, en `packaging/flatpak/me.sgomez.rfirma.desktop`

Hoy el fichero es, entero:

```ini
[Desktop Entry]
Type=Application
Name=rFirma
Exec=rfirma
Icon=me.sgomez.rfirma
Terminal=false
Categories=Utility;
```

Faltan las dos líneas. Lo que hay que dejar es `Exec=rfirma %u` y
`MimeType=x-scheme-handler/afirma;`.

**Por qué el `%u` importa aquí más que en ningún sitio**: flatpak **reescribe el `Exec=` al
exportar**, y decide *en función del código de campo del original* si añade el reenvío de ficheros.
Medido sobre los flatpaks instalados en esta máquina:

| Exec exportado | Original |
|---|---|
| `flatpak run … --command=obs com.obsproject.Studio` | sin código de campo |
| `flatpak run … --command=stremio --file-forwarding com.stremio.Stremio @@u %u @@` | con `%u` |
| `flatpak run … --command=com.discordapp.Discord --file-forwarding com.discordapp.Discord @@u %U @@` | con `%U` |

Eso es `export_desktop_file()` en `common/flatpak-dir.c`: recorre los argumentos del `Exec`
original y, si encuentra `%f` o `%u`, añade `--file-forwarding` y envuelve el código de campo en
`@@ … @@` o `@@u … @@`. La comparación es `strcasecmp`, así que **`%U` y `%F` entran por la misma
rama**. Sin código de campo no hay ni reenvío ni argumento.

Es decir: **con `Exec=rfirma` a secas, el lanzador exportado tampoco tendría dónde poner la URL**,
y el fallo sería exactamente el de arriba —la aplicación abre y no sabe a qué—. No hay nada que
tocar en el manifiesto: `me.sgomez.rfirma.yml` ya instala el `.desktop` en
`/app/share/applications/me.sgomez.rfirma.desktop`, que es a la vez el directorio que flatpak
exporta —la lista de `exported_subdirs` está **codificada en el fuente**, no es configurable— y el
nombre que exige, porque un fichero exportado sin el prefijo del *app id* **no falla: se borra**
(`g_warning ("Non-prefixed filename %s in app %s, removing.")`). Todo el arreglo cabe en el
`.desktop`.

Y flatpak mantiene la caché de sus *exports*: `/var/lib/flatpak/exports/share/applications/`
tiene su propio `mimeinfo.cache`, con las cuatro asociaciones de esquema de los flatpaks instalados
aquí. No hay que correr nada.

### `.deb` y `.rpm`: el *bundler* de Tauri **y** nuestro `desktopTemplate`

El *bundler* sí sabe hacerlo, pero **a medias**, y nosotros hemos desactivado la mitad que sí hacía.

- **La configuración no está en `bundle`**, contra lo que sugiere el nombre. La única fuente es el
  nodo del complemento:

  ```json
  { "plugins": { "deep-link": { "desktop": { "schemes": ["afirma"] } } } }
  ```

  La CLI lo lee de ahí y lo pasa al *bundler* como `deep_link_protocols`
  (`crates/tauri-cli/src/interface/rust.rs`). El esquema va **sin `://`**.

- **El *bundler* emite el `MimeType`**: en `generate_desktop_file()`
  (`crates/tauri-bundler/src/bundle/linux/freedesktop/mod.rs`) mapea cada esquema a
  `x-scheme-handler/<esquema>` y lo une con `;` a los de `fileAssociations`. Lo usan igual el `.deb`
  (`debian.rs`) y el `.rpm` (`rpm.rs`); el lanzador se llama `<productName>.desktop`, o sea
  `rfirma.desktop`.

- **El *bundler* NO emite el `%u`.** Su plantilla es `Exec={{exec}}`, y `exec` es sólo el nombre del
  binario. Es una incoherencia del propio Tauri: la plantilla que el complemento `deep-link` escribe
  *en tiempo de ejecución* sí pone `format!("\"{}\" %u", exec)`
  (`plugins/deep-link/src/lib.rs`), pero la del paquete no. *No se ha localizado un issue de Tauri
  que lo reconozca; es lectura directa del código, no cita de la documentación.*

- **Y encima nosotros sustituimos la plantilla.** `tauri.conf.json` declara
  `desktopTemplate: "../../packaging/rfirma.desktop.hbs"` para `deb` y para `rpm`, y ese fichero no
  tiene `MimeType` ni `%u`:

  ```ini
  [Desktop Entry]
  Type=Application
  Name=rFirma
  Exec={{exec}}
  Icon={{icon}}
  Terminal=false
  Categories={{categories}}
  ```

  Así que **poner `plugins.deep-link.desktop.schemes` no serviría de nada por sí solo**: el
  `MimeType` que el *bundler* calcularía se descarta con la plantilla. La variable `{{mime_type}}`
  **sí** está disponible en `DesktopTemplateParams` aunque el *docstring* de `desktopTemplate` no la
  liste, así que el arreglo es una plantilla con `Exec={{exec}} %u` y un bloque
  `{{#if mime_type}}MimeType={{mime_type}}{{/if}}`.

  El ADR-0013 explica por qué la plantilla es nuestra —«un `desktopTemplate` compartido por deb y
  rpm con el mismo contenido que el del flatpak, para que sólo diverja el nombre del fichero»—, y
  esa razón sigue en pie: **las dos líneas nuevas hay que ponerlas en los dos ficheros a la vez**,
  el `.hbs` y el `.desktop` del flatpak, o divergen.

- **Nadie corre `update-desktop-database` en la instalación.** El *bundler* no genera ningún script
  de mantenimiento: `debian.rs` copia `preinst`/`postinst`/`prerm`/`postrm` **sólo** si se declaran
  en la configuración, y `rpm.rs` igual con `preInstallScript`/`postInstallScript`. Quien lo corre
  es el **disparador de la distribución**: en esta máquina,
  `/var/lib/dpkg/info/desktop-file-utils.triggers` contiene `interest-noawait
  /usr/share/applications`, así que `dpkg` lo dispara solo con cualquier paquete que deje un
  `.desktop` ahí. Fedora hace lo propio con disparadores de fichero en el `.spec` de su
  `desktop-file-utils`: `%transfiletriggerin -- %{_datadir}/applications` y
  `%transfiletriggerpostun` con el mismo directorio, los dos corriendo `update-desktop-database`.
  **Así que el `.deb` y el `.rpm` no necesitan ningún script de mantenimiento propio** —lo que es
  una suerte, porque el *bundler* no los genera—. Lo que sí conviene es **declarar
  `desktop-file-utils` como dependencia**: sin ese paquete no hay disparador, no hay
  `mimeinfo.cache`, y —medido arriba— el registro no existe para GIO.

- **No hace falta `xdg-mime default` en un `postinst`.** AutoFirma lo tenía y lo **comentó** en su
  `.deb` con la razón correcta —«no es necesario porque se define en el fichero de control»
  (`instalador_deb/src/DEBIAN/postinst:47`)—. Ver el punto 4 para lo que sí conviene no imitar.

### Lo que **no** hay que declarar

`rfirma` no declara `application/pdf` en ningún lanzador —ID-155, ADR-0018: no es el programa de los
PDF, es lo que se hace con uno—. Un `x-scheme-handler` no toca esa decisión: no compite por abrir
ficheros, sólo por un esquema que hoy no tiene más dueño que AutoFirma. El `MimeType` del lanzador
principal pasa de vacío a llevar **exactamente una** entrada, y sigue sin llevar `application/pdf`.

---

## 2. Cómo llega la URL al proceso: argumento, no D-Bus

**En los tres canales, argumento de línea de órdenes.** No hay activación D-Bus en ninguna parte y
no conviene añadirla.

### Por qué no D-Bus

`DBusActivatable=true` cambia el camino entero. La Desktop Entry Spec lo dice sin rodeos: «If the
value is true then implementations should **ignore the Exec key** and send a D-Bus message to launch
the application». El nombre de bus es el del `.desktop` sin la extensión —`me.sgomez.rfirma`— y el
objeto se deriva de él cambiando puntos por barras con una barra delante,
`/me/sgomez/rfirma`; ahí hay que servir `org.freedesktop.Application`, cuyo método `Open` recibe
`as` de URI. Eso es lo que hace GIO: si el `.desktop` es activable y hay bus de sesión, llama a
`Open`; si no, cae a `Exec=` con la URI sustituida en `%u`. *La especificación habla de «files» en
la prosa de `Open` aunque el tipo sea una lista de URI; que un esquema propio viaje por ahí es
inferencia de la firma, no cita.*

Es un camino real y flatpak lo soporta —medido: Stremio,
que declara `DBusActivatable=true`, se exporta con un
`/var/lib/flatpak/exports/share/dbus-1/services/com.stremio.Stremio.service` cuyo `Exec` es
`flatpak run … --gapplication-service`—. Pero:

- El *bundler* de Tauri **no** emite `DBusActivatable` ni ningún `.service`; habría que fabricar los
  dos a mano y en los tres canales. En el flatpak, además, el `.service` tiene que llamarse
  exactamente igual que su clave `Name` o **el export falla con error**, no con aviso.
- Tauri no implementa `org.freedesktop.Application`. Con GApplication sale gratis; aquí habría que
  escribirlo entero.
- Y no compra nada: el complemento de instancia única **ya** monta un servicio de sesión que hace
  justo esto (punto 3), y el `Exec=… %u` ya entrega la URL. Serían dos mecanismos para lo mismo, que
  es el patrón que este repositorio lleva tres hallazgos de fallo silencioso evitando (ADR-0013).

Obsérvese además que Stremio declara `DBusActivatable=true` **y** conserva `%u` en su `Exec` —el
export de flatpak no toca la clave `DBusActivatable`, sólo el `Exec`—. La especificación pide
justamente eso: «Applications should still include Exec= lines in their desktop files for
compatibility with implementations that do not understand the DBusActivatable key». El camino de
argumento es el respaldo obligatorio, no una alternativa: **añadir D-Bus no permite quitar el
`%u`**.

### Qué llega exactamente, en el flatpak

El `Exec` exportado no es el nuestro. Con `%u` en el original, flatpak exporta
`flatpak run … --file-forwarding me.sgomez.rfirma @@u %u @@`. El bloque `@@u … @@` es reenvío de
URI: los `file:` se convierten en documentos del portal, y **el resto pasa tal cual**. Medido
directamente:

```console
$ flatpak run --file-forwarding --command=echo com.discordapp.Discord \
      @@u 'afirma://websocket?ports=1,2,3&v=4&idsession=ABC' /etc/hostname @@
afirma://websocket?ports=1,2,3&v=4&idsession=ABC file:///run/user/1000/doc/fa9787f/hostname
```

La URL de esquema propio llega **literal, sin tocar**, al proceso de dentro del sandbox; el fichero
de al lado sí se convierte en un enlace del portal. Es exactamente lo que hace falta: el esquema
`afirma://` no nombra un fichero, así que el portal no tiene nada que hacer con él y no lo estropea.

Y no es casualidad ni suerte: `add_args_and_forward_files()` (`common/flatpak-run.c`) sólo construye
un `GFile` cuando el argumento empieza por `file:` o por una barra; cualquier otra cosa cae en el
`else` y se pasa al sandbox tal cual. El manual lo dice igual de claro: «Arguments between "@@u" and
"@@" are considered URIs, and **any "file:" URIs are exported**». **Ningún `--filesystem` hace falta
para que llegue la URL** —los `--filesystem` sólo deciden si un `file:` se sirve directo o por el
portal—, así que el manifiesto no se toca por esto.

En el `.deb` y el `.rpm` no hay intermediario: el argumento es la URL y punto.

---

## 3. El cruce con la instancia única

### El mecanismo, leído en la versión que fija el `Cargo.lock`

`tauri-plugin-single-instance` **2.4.4** (`platform_impl/linux.rs`) usa el **bus de sesión**, no
sockets ni ficheros de bloqueo:

- La primera instancia reclama el nombre `<identifier>.SingleInstance` —para nosotros
  **`me.sgomez.rfirma.SingleInstance`**—, sirve el objeto en `/me/sgomez/rfirma/SingleInstance` con
  la interfaz `org.SingleInstance.DBus`, y **no permite que se lo quiten**
  (`replace_existing_names(false)`, `allow_name_replacements(false)`).
- La segunda recibe `NameTaken`, llama a `ExecuteCallback(argv, cwd)` sobre ese nombre y hace
  `std::process::exit(0)`.

Así que **sí: la URL entera llega**, en `command_line[1]` del `Invocation` que construye
`lib.rs:87`, porque lo que la segunda instancia manda es su `std::env::args()` completo.

Dos cosas conviene apuntar, porque no son obvias:

- **El nombre de bus cambió en la 2.4.0** (PR #3194): antes era
  `org.<identifier_con_guiones_bajos>.SingleInstance`. **La documentación oficial de Tauri sigue
  describiendo el nombre viejo** y pidiendo, para flatpak, un par de `--own-name`/`--talk-name` sobre
  él. Copiar eso hoy autorizaría un nombre que la aplicación ya no usa. Con la 2.4.4 no hace falta
  (ver abajo).
- **La segunda instancia usa `std::env::args()`, que entra en pánico con un argumento que no sea
  UTF-8 válido.** `Invocation::of_this_process()` usa `args_os()` a propósito y documenta por qué
  —«morir en el arranque es lo contrario del ID-158»—; **el camino de la segunda invocación no tiene
  esa protección y no la podemos poner, está dentro del complemento**. No afecta a una URL
  `afirma://` (es ASCII), sí a un segundo `rfirma /ruta/con/bytes/ilegibles.pdf`. Queda anotado: es
  una diferencia real entre las dos puertas de entrada, y no la vigila ninguna prueba.

### Dentro del flatpak: funciona, y sin permisos nuevos

La política por defecto del bus de sesión en flatpak deja a la aplicación poseer su propio
`$FLATPAK_ID` **y los subnombres de él**. En el fuente es literalmente
`flatpak_bwrap_add_arg_printf (bwrap, "--own=%s.*", app_id)`
(`flatpak_context_add_bus_filters`, `common/flatpak-context.c`), y el manual de `xdg-dbus-proxy`
define el comodín así: «A name of "org.foo.\*" matches "org.foo", "org.foo.bar", and
"org.foo.bar.gazonk", but not "org.foobar"». `me.sgomez.rfirma.SingleInstance` es un subnombre de
`me.sgomez.rfirma`: entra en la política por defecto. Medido con dos instancias simultáneas del
mismo id (usando `org.gnome.Platform` como cobaya, con `gdbus`/PyGObject del propio runtime):

```console
# instancia 1
RequestName org.gnome.Platform.SingleInstance -> 1   # PRIMARY_OWNER
# instancia 2, a la vez
RequestName org.gnome.Platform.SingleInstance -> 2   # IN_QUEUE  →  NameTaken
```

Y la llamada de vuelta **atraviesa el proxy**: desde la segunda instancia, invocar
`org.SingleInstance.DBus.ExecuteCallback` sobre ese nombre devuelve

```
GDBus.Error:org.freedesktop.DBus.Error.UnknownMethod: Object does not exist at path “/x”
```

—un error **del dueño**, no un `AccessDenied` del `xdg-dbus-proxy`—. Es decir: el mensaje llegó. La
segunda invocación entrega la URL a la ventana que corre, dentro del sandbox, con los `finish-args`
que ya tiene el manifiesto y **sin añadir ni `--own-name` ni `--talk-name`**.

Un matiz: cada `flatpak run` crea una **instancia nueva del sandbox**, no reutiliza la anterior
—`flatpak_builtin_run` monta siempre un `bwrap` nuevo; la unicidad la pone la aplicación, no
flatpak—. Lo
que se comparte es el bus de sesión, que es justo lo que el complemento necesita. El sandbox nuevo
arranca, descubre el nombre tomado, manda el `argv` y se muere; el coste es un arranque de sandbox
por clic, que es lo mismo que ya pasa hoy al abrir un PDF desde el gestor de ficheros.

### El agujero real: hoy la URL se interpretaría como una ruta

Esto es lo que hay que arreglar en el código, y no lo arregla ningún `.desktop`. `invoked_document`
delega en `dropped::invoked_pdf`, que hace literalmente:

```rust
let paths: Vec<PathBuf> = command_line.iter().skip(1)
    .filter(|argument| !argument.starts_with('-'))
    .map(|argument| from.join(argument))
    .collect();
first_pdf(&paths)
```

Un `afirma://websocket?…` no empieza por `-`, así que se une a la carpeta de trabajo, no tiene
extensión `.pdf`, y acaba en la ventana normal diciendo que eso no es un PDF (ID-158). **Se traga la
URL en silencio y con una explicación equivocada.** Vale para las dos puertas: la primera invocación
(`of_this_process`) y la segunda (el *callback* de `lib.rs:87`).

Hay dos maneras de cerrarlo, y la decisión no es de este sondeo:

- **Con el complemento `deep-link`**: `tauri-plugin-single-instance` tiene una *feature* `deep-link`
  que, antes de llamar a nuestro *callback*, pasa el `argv` a
  `tauri_plugin_deep_link::DeepLink::handle_cli_arguments`, que emite `deep-link://new-url`. Trae
  dos ataduras leídas en el código: sólo acepta **exactamente un** argumento tras el nombre del
  binario —un segundo argumento y la URL se descarta **en silencio**— y sólo esquemas declarados
  estáticamente en `plugins.deep-link.desktop.schemes`.
- **A mano**, discriminando el argumento en `invocation.rs` antes de tratarlo como ruta. Es una
  condición sobre un prefijo, y deja la regla donde ya está el resto de la decisión de qué abre una
  invocación.

Lo que **no** cambia en ninguno de los dos casos: el orden de registro. El complemento de instancia
única debe seguir siendo **el primero**, como ya documenta `lib.rs:79-83`, porque hace
`std::process::exit(0)` dentro de su `setup`.

Y una advertencia sobre el ID-160: la segunda invocación **sustituye sin preguntar**, salvo con una
firma viva. Una URL `afirma://` no es un documento, así que la regla de sustitución tal y como está
escrita hoy no le aplica —`second_invocation` devolvería `None` por no haber PDF—, pero la excepción
de la firma viva sí debería aplicarle, y con más razón: una sede que pide firmar mientras hay un PIN
a medias es exactamente el caso que el ID-160 protege. Es una decisión de spec, no de este sondeo.

---

## 4. Qué pasa si AutoFirma también está instalada

Es **determinista**, y el resultado **depende del canal**. Medido en esta máquina, que tiene
AutoFirma 1.9 instalada por su `.deb`.

### Lo que declara AutoFirma

- **`.deb`**: `/usr/share/applications/afirma.desktop`, con `MimeType=x-scheme-handler/afirma;` y
  `Exec=/usr/bin/autofirma %u`. Nada más: la línea `xdg-mime default` de su `postinst` está
  **comentada**.
- **`.rpm` (Fedora y openSUSE)**: además del `.desktop`, el `%post` **se declara predeterminada a
  mano**, añadiendo `x-scheme-handler/afirma=autofirma.desktop` a
  `/usr/share/applications/mimeapps.list` (y en openSUSE también a `gnome-mimeapps.list`,
  `/usr/local/share/applications/mimeapps.list` y su `gnome-mimeapps.list`). Lo hace con un `echo
  >>` a pelo, sin comprobar si el fichero ya tiene sección `[Default Applications]` ni si la línea
  está repetida.

### Quién gana

El algoritmo lo fija mime-apps-spec: se recorren los `mimeapps.list` en orden —primero
`$XDG_CONFIG_HOME`, luego `$XDG_CONFIG_DIRS`, luego los `applications/mimeapps.list` de
`$XDG_DATA_HOME` y `$XDG_DATA_DIRS`— buscando una entrada en `[Default Applications]` que además
**esté realmente asociada** al tipo; y si ninguno la da, el paso final es «select the **most-preferred
application (according to associations)** that supports the type». Ese último paso es el que
gobierna nuestro caso, porque hoy nadie escribe un predeterminado explícito en Debian ni en Ubuntu.
Y lo decide el orden de `XDG_DATA_DIRS` y, dentro de un directorio, el orden de `mimeinfo.cache`.
Ambas cosas medidas:

- **El orden dentro de la caché es alfabético por *desktop id*, no de instalación.** Creando primero
  `zzz.desktop` y después `aaa.desktop`, `update-desktop-database` escribe
  `x-scheme-handler/afirma=aaa.desktop;zzz.desktop;`.
- **Gana el primero de la caché** cuando no hay preferencia explícita. Con `afirma.desktop` y
  `rfirma.desktop` en el mismo directorio:
  `Default application for “x-scheme-handler/afirma”: afirma.desktop`.
- **Un directorio de más prioridad gana entero.** Añadiendo un `.desktop` con ese `MimeType` en
  `~/.local/share/applications` —y **sin tocar `mimeapps.list`**—, la predeterminada pasó
  inmediatamente de `afirma.desktop` a la nueva. En cuanto se retiró, volvió a `afirma.desktop`.

Aplicado a los tres canales, con el `XDG_DATA_DIRS` real de esta máquina
(`~/.local/share/flatpak/exports/share : /var/lib/flatpak/exports/share : /usr/local/share : /usr/share : …`):

| Canal de rfirma | AutoFirma instalada por | Quién gana | Por qué |
|---|---|---|---|
| flatpak | `.deb` | **rfirma** | los *exports* de flatpak van antes que `/usr/share` |
| `.deb` | `.deb` | **AutoFirma** | mismo directorio, y `afirma` < `rfirma` alfabéticamente |
| `.rpm` (Fedora/openSUSE) | `.rpm` | **AutoFirma** | su `%post` se pone predeterminada en `mimeapps.list`, que manda sobre la caché |

Los dos resultados son malos por motivos opuestos. En el `.deb` y el `.rpm` **rfirma no se
enteraría nunca** de que la sede llamó. En el flatpak **rfirma le roba el esquema a AutoFirma sin
avisar a nadie**, que es precisamente lo que hace AutoFirma en Fedora y lo que aquí se ha llamado
siempre «éxito parcial silencioso».

### Sí, la persona puede elegir, y esa es la única salida honrada

`xdg-mime default rfirma.desktop x-scheme-handler/afirma` —o el selector de aplicaciones
predeterminadas de GNOME o de KDE— escribe la preferencia en `~/.config/mimeapps.list`, bajo
`[Default Applications]`, y esa preferencia **gana a todo lo anterior** en los tres canales, porque
la configuración del usuario va antes que la del sistema. Es reversible y es visible.

Lo que **no** conviene hacer es imitar a AutoFirma: escribir en el `mimeapps.list` **del sistema**
desde un `postinst`/`%post` es apropiarse de una decisión del usuario, deja residuo (punto 5) y en
el `.deb`/`.rpm` de rfirma exigiría inventar el script de mantenimiento que hoy no existe. Si se
quiere que rfirma pueda ser la predeterminada, lo correcto es **ofrecerlo desde la aplicación** —el
complemento `deep-link` trae `register()`/`set_as_default()`, que corren `update-desktop-database` y
`xdg-mime default` sobre `$XDG_DATA_HOME/applications`, es decir sobre el usuario y no sobre el
sistema— y que sea un gesto, no un efecto de instalar.

*No verificado*: qué hace exactamente el diálogo de Firefox/Chrome cuando hay dos candidatos. Lo que
sí consta es que AutoFirma se salta esa pregunta en Firefox instalando
`network.protocol-handler.warn-external.afirma = false` en un `.js` de preferencias del sistema
(`/usr/lib/firefox/defaults/pref/`), que es un truco que rfirma no debería copiar sin decidirlo
aparte.

---

## 5. Reversibilidad al desinstalar

El registro vive en el `.desktop`, y el `.desktop` lo instala y lo borra el gestor de paquetes. Por
tanto **el registro es reversible en los tres canales**, con matices distintos:

- **Flatpak.** Los *exports* son **enlaces simbólicos** al árbol desplegado, y
  `flatpak_dir_uninstall()` termina llamando a `flatpak_dir_update_exports()`, que barre los enlaces
  colgantes (`flatpak_remove_dangling_symlinks`). La caché la regenera el disparador propio de
  flatpak, `triggers/desktop-database.trigger`, que corre `update-desktop-database` sobre
  `exports/share/applications` después de cada operación. No queda nada nuestro. *No medido por
  desinstalación real* —no se ha instalado el flatpak de rfirma en esta máquina—, pero es la misma
  mecánica por la que los cuatro esquemas de
  `/var/lib/flatpak/exports/share/applications/mimeinfo.cache` corresponden exactamente a los
  flatpaks que hay instalados.
- **`.deb`.** `dpkg` borra `/usr/share/applications/rfirma.desktop` y el disparador
  `interest-noawait /usr/share/applications` de `desktop-file-utils` regenera `mimeinfo.cache`. No
  hace falta `postrm`; el de AutoFirma, de hecho, no hace nada más que imprimir un mensaje. Sin
  `desktop-file-utils` instalado no hay disparador, pero tampoco había caché, así que tampoco hay
  residuo.
- **`.rpm`.** Igual, y con el detalle de que el `.spec` de `desktop-file-utils` de Fedora declara
  **las dos direcciones**: `%transfiletriggerin` y `%transfiletriggerpostun` sobre
  `%{_datadir}/applications`. La desinstalación regenera la caché sola. *No medido aquí: no hay
  `rpm` en esta máquina.*

**Los dos residuos que sí quedan**, y conviene tenerlos escritos:

1. **La elección explícita del usuario.** Si alguien puso rfirma como predeterminada,
   `~/.config/mimeapps.list` conserva `x-scheme-handler/afirma=rfirma.desktop` después de
   desinstalar. Ningún gestor de paquetes toca el `$HOME`, y así debe ser. El efecto es benigno y la
   propia especificación lo prevé —«If the application is no longer installed, the next application
   in the list is attempted»—, pero es la razón de más para no escribirla desde la instalación: lo
   que el paquete escribe, el paquete debería poder borrarlo, y en el `$HOME` no puede.
2. **El de AutoFirma, que no es nuestro pero nos toca.** Su `%postun` de Fedora borra su línea de
   `/usr/share/applications/mimeapps.list` **sólo en desinstalación (`$1 -eq 0`), no al
   actualizar**, y su `%post` la vuelve a añadir con un `echo >>` cada vez. Es decir: **en Fedora y
   openSUSE, actualizar AutoFirma duplica la línea y la reafirma como predeterminada**, pisando
   cualquier elección que el usuario haya hecho en el `mimeapps.list` del sistema. Su elección en
   `~/.config/mimeapps.list` sí sobrevive, porque va antes. Es un argumento más para que la respuesta
   de rfirma a «quiero que me llamen a mí» sea un gesto en la aplicación, del usuario y en el
   `$HOME`, y no una carrera de scripts de instalación en `/usr`.

---

## Lo que esto deja para el spec de la v0.5

1. Añadir `MimeType=x-scheme-handler/afirma;` **y `%u` en el `Exec=`** a los **dos** ficheros a la
   vez: `packaging/flatpak/me.sgomez.rfirma.desktop` y `packaging/rfirma.desktop.hbs`. En el `.hbs`,
   con `{{mime_type}}` y `plugins.deep-link.desktop.schemes = ["afirma"]` en `tauri.conf.json`, o
   con la cadena literal; lo que no vale es poner la configuración y confiar en que la plantilla
   propia la recoja sola.
2. Declarar `desktop-file-utils` en las dependencias del `.deb` y del `.rpm`: sin él no hay
   `mimeinfo.cache` y el registro no existe para GIO.
3. Una **puerta en `packaging/verifica-contenido.sh`** que afirme, sobre cualquiera de los tres
   artefactos, que el lanzador lleva las dos líneas. Es el mismo patrón que la invariante del `.so`:
   un `.desktop` sin `%u` produce un paquete que instala bien, arranca bien y no recibe nunca la
   llamada de la sede. Es justo el fallo silencioso caro que este repositorio ya sabe cazar barato.
4. Enseñar a `invocation.rs` a distinguir una URL de una ruta, **en las dos puertas** —primera y
   segunda invocación—, y decidir si eso llega por el complemento `deep-link` (con su límite de un
   solo argumento y su lista estática de esquemas) o a mano.
5. Decidir si rfirma **ofrece** ponerse como predeterminada desde la aplicación, en el `$HOME`; y
   dejar escrito que **no** se escribe en el `mimeapps.list` del sistema desde ningún script de
   instalación.
6. No hace falta ningún `finish-args` nuevo en el manifiesto del flatpak. En particular, **no** los
   `--own-name`/`--talk-name` que la documentación de Tauri sigue pidiendo: describen el nombre de
   bus anterior a `tauri-plugin-single-instance` 2.4.0.

## Lo que NO se ha podido comprobar

1. La desinstalación real del flatpak de rfirma (no está instalado aquí); la mecánica de enlaces
   colgantes sí está leída en `flatpak-dir.c`.
2. Los disparadores de `rpm`: no hay `rpm` en esta máquina. El `.spec` de Fedora sí está leído.
3. Que `org.freedesktop.Application.Open` admita esquemas no-`file`: se deduce de la firma `as` /
   «array of URIs»; la prosa de la especificación dice «files».
4. El comportamiento del selector de aplicaciones predeterminadas de KDE Plasma, que tiene su propia
   caché (`ksycoca`). Lo medido aquí es GIO.
5. Qué pinta exactamente el diálogo de Firefox o de Chrome cuando hay **dos** candidatos para
   `afirma://`.
6. Si Tauri tiene un *issue* abierto reconociendo que su plantilla de `.desktop` emite el `MimeType`
   pero no el `%u`. Lo afirmado es lectura directa del código, no cita de su documentación.

## Fuentes

**Medido en esta máquina** (Ubuntu con `dpkg`, Flatpak 1.16.6, `desktop-file-utils` 0.28, AutoFirma
1.9 por `.deb`): `gio mime` con y sin `mimeinfo.cache`; orden de `mimeinfo.cache` frente a orden de
creación; precedencia de `~/.local/share/applications`; `flatpak run --file-forwarding` con una URL
de esquema propio y un fichero; posesión del subnombre `<app-id>.SingleInstance` desde dos
instancias simultáneas y llamada de una a la otra a través del proxy; `Exec=` exportado de cinco
flatpaks instalados; disparador de `desktop-file-utils`.

**Código leído en la versión que fija `rfirma-app/src-tauri/Cargo.lock`**:
`tauri-plugin-single-instance` 2.4.4 (`src/platform_impl/linux.rs`, `src/lib.rs`, `Cargo.toml`),
`tauri-utils` 2.9.3 (`src/config.rs`).

**Código y documentación de terceros**: `tauri-bundler`
(`crates/tauri-bundler/src/bundle/linux/freedesktop/{main.desktop,mod.rs}`, `debian.rs`, `rpm.rs`),
`tauri-cli` (`src/interface/rust.rs`), `tauri-plugin-deep-link` (`src/lib.rs`,
`src/template.desktop`, `src/config.rs`) y la documentación de Tauri v2 de *deep linking* y
*single instance*; flatpak (`common/flatpak-dir.c`, `common/flatpak-run.c`,
`common/flatpak-context.c`, `profile/flatpak.sh`, `triggers/desktop-database.trigger`,
`doc/flatpak-run.xml`), `xdg-dbus-proxy`, GLib (`gio/gdesktopappinfo.c`), las especificaciones
*Desktop Entry* y *mime-apps* de freedesktop, la página de manual de `update-desktop-database`, y
los empaquetados de `desktop-file-utils` de Debian y de Fedora.

**Instalador de AutoFirma**, en el clon de `clienteafirma`:
`afirma-simple-installer/linux/instalador_deb/src/usr/share/applications/afirma.desktop`,
`…/instalador_deb/src/DEBIAN/{control,postinst,postrm}`,
`…/instalador_rpm_fedora/rpmbuild/SPECS/autofirma.spec` y su equivalente de openSUSE.

## Discoveries

- El ADR-0005 dice que «el flatpak sí puede registrar `x-scheme-handler/afirma` exportando su
  `.desktop`». Es cierto pero incompleto y **induce al error caro**: sin `%u` el registro funciona y
  la URL no llega. Conviene enmendar esa frase cuando se implemente el esquema.
- `docs/AGENTS.md` se amplía en esta misma rama con este informe.
