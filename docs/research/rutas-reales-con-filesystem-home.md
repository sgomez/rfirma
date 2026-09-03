# ¿Puede el diálogo de Tauri devolver rutas reales con `--filesystem=home`?: medición

Sondeo del issue [#240](https://github.com/sgomez/rfirma/issues/240), del que
depende entero el [#221](https://github.com/sgomez/rfirma/issues/221). La
pregunta no es si `--filesystem=home` da acceso al disco —eso nunca estuvo en
duda— sino si **el diálogo de ficheros que rfirma usa de verdad** devuelve la
ruta del anfitrión cuando ese permiso está declarado. Si la devolviera, flatpak
y `.deb` se comportarían igual y el aparato que el #221 estaba diseñando
sobraría.

Entorno: Ubuntu 26.04, sesión GNOME sobre Wayland, `xdg-desktop-portal` del
anfitrión, `org.gnome.Platform//50` con **GTK 3.24.52**. El bundle medido es el
`me.sgomez.rfirma` instalado (commit `a4e013ef`, rama `stable`), con sus
permisos reales; donde hizo falta `--filesystem=home` se añadió en la línea de
`flatpak run`, que es exactamente lo que haría el manifiesto. Todas las
mediciones se hicieron con `flatpak run --command=python3 me.sgomez.rfirma -`
según describe el `CLAUDE.md`. Las rutas del anfitrión aparecen redactadas como
`/home/<usuario>/`.

## Veredicto

**No.** Y no por una limitación del permiso, sino por una decisión que está
tres capas más abajo y que rfirma no puede tocar sin escribir su propio diálogo:

1. `tauri-plugin-dialog` va sobre `rfd`, y el backend `gtk3` de `rfd` —el que
   este proyecto tiene activo— no usa `GtkFileChooserDialog` sino
   **`GtkFileChooserNative`**.
2. `GtkFileChooserNative` en GTK 3.24 **enruta al portal siempre que existe
   `$XDG_RUNTIME_DIR/flatpak-info`**, es decir, siempre dentro de un flatpak, y
   **`GTK_USE_PORTAL=0` no lo anula** porque en GTK 3 esa variable ni se lee
   cuando el fichero existe. Medido: la llamada D-Bus a
   `org.freedesktop.portal.FileChooser.OpenFile` se emite igual con la variable
   sin poner, a `0` y a `1`.
3. La otra opción de `rfd`, la feature `xdg-portal`, es *más* portal todavía.
   No hay configuración de `tauri-plugin-dialog` que devuelva rutas reales
   dentro del arenero.

`--filesystem=home` sí cambia una cosa —el acceso directo con `std::fs` a
`/home/<usuario>/…` funciona, medido— pero **la aplicación nunca llega a saber
qué ruta real corresponde al documento que el usuario eligió**: el portal
contesta `NotAllowed` a `Documents.Info` y `Documents.Lookup` también con `home`
declarado. Y la trampa del `.xdp-…` huérfano del
[#22](https://github.com/sgomez/rfirma/issues/22) **se reproduce igual, con
`home` puesto**.

Conclusión para el #221: la vía de `--filesystem=home` **no lo disuelve**. El
ADR-0011 llega a la conclusión correcta; lo que hay que corregir es su
*argumento*, porque la frase «conservar junto al original exigiría saltarse el
portal» es cierta por un motivo distinto del que dice.

## 1. `rfd`: ¿portal obligatorio o diálogo GTK en proceso?

Versiones resueltas en `rfirma-app/src-tauri/Cargo.lock`: `tauri-plugin-dialog
2.7.3` y `rfd 0.16.0`. `rfirma-app/src-tauri/Cargo.toml` declara
`tauri-plugin-dialog = "2"` sin tocar features, así que valen las de omisión.

`tauri-plugin-dialog-2.7.3/Cargo.toml` (líneas 64-71 y 112-115):

```toml
[features]
default = ["gtk3"]
gtk3 = ["rfd/gtk3"]
xdg-portal = ["rfd/xdg-portal", "rfd/tokio", "rfd/wayland"]

[target.'cfg(...)'.dependencies.rfd]
version = "0.16"
features = ["common-controls-v6"]
default-features = false
```

Es decir: **rfirma está hoy en el backend `gtk3`**. Confirmado en el propio
`Cargo.lock`, donde las dependencias de `rfd` incluyen `gtk-sys`, `glib-sys` y
`gobject-sys`, y **no** incluyen `ashpd`, `pollster` ni `urlencoding`, que son
las tres que arrastra `xdg-portal`.

La elección es **en tiempo de compilación y solo por feature**. `rfd-0.16.0/src/backend.rs`
(líneas 8-47) no tiene más predicado que `feature = "gtk3"` frente a
`not(feature = "gtk3")`; no hay ninguna variable de entorno ni detección de
arenero en `rfd`. Y las dos features son **excluyentes de forma dura**:
`rfd-0.16.0/build.rs` (líneas 8-15)

```rust
if gtk && xdg {
    panic!("You can't enable both `gtk3` and `xdg-portal` features at once");
} else if !gtk && !xdg {
    panic!("You need to choose at least one backend: `gtk3` or `xdg-portal` features");
}
```

Consecuencia práctica que conviene dejar escrita, porque es contraintuitiva:
**no se puede «forzar GTK» añadiendo `rfd` con la feature `gtk3` como
dependencia directa**. Cargo une features y nunca las quita, así que la unión
tendría las dos y el `build.rs` aborta la compilación. La única palanca sin
bifurcar es la feature `xdg-portal` del propio plugin, documentada en su README
(líneas 34-46) y en `src/lib.rs` (líneas 9-10) — y es la palanca que va en la
dirección contraria a la que interesaba.

**Aquí está el hallazgo que decide el sondeo.** Que `rfd` esté en el backend
`gtk3` *no* significa diálogo en proceso, porque ese backend no abre un
`GtkFileChooserDialog`. `rfd-0.16.0/src/backend/gtk3/file_dialog/dialog_ffi.rs`
(líneas 3 y 29):

```rust
use gtk_sys::GtkFileChooserNative;
...
let dialog = gtk_sys::gtk_file_chooser_native_new(
```

Y `GtkFileChooserNative` es precisamente el widget que GTK redirige al portal
dentro de un flatpak.

## 2. GTK 3.24: el enrutado al portal es forzoso e inanulable

Fuente primaria, rama `gtk-3-24` de GNOME/GTK,
[`gtk/gtkprivate.c`](https://gitlab.gnome.org/GNOME/gtk/-/raw/gtk-3-24/gtk/gtkprivate.c)
(líneas 271-293):

```c
gboolean
gtk_should_use_portal (void)
{
  static const char *use_portal = NULL;

  if (G_UNLIKELY (use_portal == NULL))
    {
      char *path;

      path = g_build_filename (g_get_user_runtime_dir (), "flatpak-info", NULL);
      if (g_file_test (path, G_FILE_TEST_EXISTS))
        use_portal = "1";
      else
        {
          use_portal = g_getenv ("GTK_USE_PORTAL");
          if (!use_portal)
            use_portal = "";
        }
      g_free (path);
    }

  return use_portal[0] == '1';
}
```

El orden importa y es el que mata la vía: **si existe
`$XDG_RUNTIME_DIR/flatpak-info`, `use_portal` vale `"1"` y `GTK_USE_PORTAL` ni
siquiera se lee** — está en la rama `else`. (En GTK 4 esto se invirtió y la
variable sí puede anular; en GTK 3 no, y Tauri en Linux es GTK 3.)

El modo se elige en
[`gtk/gtkfilechoosernative.c`](https://gitlab.gnome.org/GNOME/gtk/-/raw/gtk-3-24/gtk/gtkfilechoosernative.c)
(líneas 749-777), con el respaldo en proceso al final:

```c
  if (self->mode == MODE_FALLBACK &&
      gtk_file_chooser_native_portal_show (self))
    self->mode = MODE_PORTAL;

  if (self->mode == MODE_FALLBACK)
    show_dialog (self);
```

y `gtk_file_chooser_native_portal_show` abre con `if (!gtk_should_use_portal ())
return FALSE;`
([`gtkfilechoosernativeportal.c`](https://gitlab.gnome.org/GNOME/gtk/-/raw/gtk-3-24/gtk/gtkfilechoosernativeportal.c),
líneas 455-466). Lo que devuelve cada modo está en `gtk_file_chooser_native_get_files()`
(líneas 731-747): en `MODE_PORTAL`, los URI que dio el portal; en
`MODE_FALLBACK`, los del diálogo en proceso.

**Medición**, con `--filesystem=home` declarado y `flatpak run --log-session-bus`,
mostrando un `Gtk.FileChooserNative`:

```
===== NATIVE  [] =====
C23: -> org.freedesktop.portal.Desktop call org.freedesktop.portal.FileChooser.OpenFile at /org/freedesktop/portal/desktop
PRE-show visibles: []
POST-show visibles: []
===== NATIVE  [--env=GTK_USE_PORTAL=0] =====
C23: -> org.freedesktop.portal.Desktop call org.freedesktop.portal.FileChooser.OpenFile at /org/freedesktop/portal/desktop
===== NATIVE  [--env=GTK_USE_PORTAL=1] =====
C23: -> org.freedesktop.portal.Desktop call org.freedesktop.portal.FileChooser.OpenFile at /org/freedesktop/portal/desktop
```

Las tres veces sale la llamada al portal, y ninguna ventana de elección de
fichero se hace visible dentro del proceso. `--filesystem=home` no cambia nada
de esto: GTK no consulta los permisos del flatpak para decidir.

> Aviso para quien repita la medición: contar los toplevels de
> `Gtk.Window.list_toplevels()` **no sirve** para distinguir los dos modos.
> `GtkFileChooserNative` construye su `GtkFileChooserDialog` de respaldo en el
> `init` (`gtkfilechoosernative.c:544`), así que el toplevel existe siempre,
> esté o no en modo portal. Hay que mirar la visibilidad, o el bus. Este sondeo
> se equivocó primero por ahí.

**Un `GtkFileChooserDialog` normal, en cambio, no tiene enrutado al portal
en ninguna parte**: un grep de `portal` sobre el `gtk/` de la rama solo
aparece en `gtkprivate.c`, `gtkprintoperation-unix.c`, `gtkcolorpickerportal.c`,
`gtkfilechoosernativeportal.c` y `gtkapplication-dbus.c`;
`gtkfilechooserdialog.c` y `gtkfilechooserwidget.c` no lo mencionan.
Comprobado en el arenero: el `Gtk.FileChooserDialog` sí se hace visible en
proceso y no emite ninguna llamada al portal.

```
===== DIALOG (en proceso) =====
POST-show visibles: ['FileChooserDialog']
```

Ese es el camino por el que GIMP o un editor de textos empaquetado obtienen
rutas reales. **Está disponible, pero cuesta un diálogo propio escrito contra
GTK a mano**, fuera de `tauri-plugin-dialog` y fuera de `rfd`, y eso ya no es
una concesión de permisos: es una pieza nueva de interfaz nativa que hay que
mantener en paralelo a la que se usa en `.deb`. El coste está del mismo lado
que el aparato que el #221 quería evitar.

## 3. Lo que `--filesystem=home` sí cambia, y lo que queda fuera

Con los permisos instalados (`xdg-documents`) y con `home` añadido, mismo
script, mismo fichero de prueba en `/home/<usuario>/probe-240/original.pdf`:

| Comprobación | `xdg-documents` (instalado) | `--filesystem=home` |
|---|---|---|
| `listdir /home/<usuario>` | OK, **5** entradas (solo lo montado) | OK, **194** entradas |
| leer `/home/<usuario>/probe-240/original.pdf` | `FileNotFoundError` | **OK, 35 bytes** |
| escribir un hermano en esa carpeta | `FileNotFoundError` | **OK, y aparece en el anfitrión** |
| `listdir /media` | no existe | **no existe** |
| `listdir /mnt` | no existe | **no existe** |
| `listdir /tmp` | el `/tmp` privado del arenero | el `/tmp` privado del arenero |

Así que el acceso directo por ruta real funciona **dentro de `home` y solo
dentro de `home`**. `/media` y `/mnt` existen en el anfitrión y **siguen sin
existir dentro del arenero** con `home` puesto; el `/tmp` que se ve es el
privado del flatpak, no el del anfitrión. Para un documento en un disco externo,
en `/tmp` del anfitrión o en cualquier punto de montaje, **el portal sigue
siendo la única entrada**, y la aplicación tendría que seguir tratando bien la
ruta `/run/user/<uid>/doc/…`. O sea: `home` no elimina el caso, **añade un
segundo caso**. Dos caminos en vez de uno es más aparato, no menos.

*No medido*: no había ningún disco externo ni punto de montaje disponible en
este equipo, así que el comportamiento con un `/run/media/<usuario>/<volumen>`
real no se ha comprobado; lo que sí está medido es que los directorios raíz
donde aparecería no se montan.

## 4. La trampa del `.xdp-…` huérfano sigue ahí

Es el punto que más importa, porque era el que el #221 esperaba que
desapareciera. **No desaparece.**

Primero, el dato que la sostiene: con `--filesystem=home` declarado, el portal
**sigue negándose a traducir** una ruta suya a la real.

```
Documents.Add OK, doc_id = 4a720af2
Documents.Info FALLO: org.freedesktop.portal.Error.NotAllowed: Not allowed in sandbox (36)
Documents.Lookup FALLO: org.freedesktop.portal.Error.NotAllowed: Not allowed in sandbox (36)
```

Así que aunque la aplicación *pueda* escribir en `/home/<usuario>/…`, **no sabe
en qué carpeta de `/home/<usuario>/…` está el documento que el usuario acaba de
elegir**. Poder escribir y saber dónde escribir son cosas distintas, y `home`
solo da la primera.

Y segundo, la trampa reproducida tal cual, con `home` puesto, exportando el
fichero por `Documents.AddFull` con permisos de escritura y escribiendo un
hermano en el directorio del portal:

```
AddFull -> ['363150c6']
contenido: ['original.pdf']
escritura de hermano: OK (sin error)
contenido tras escribir: ['original.pdf', 'original-firmado.pdf']
existe segun el sandbox: True
```

En el anfitrión, al mismo tiempo:

```
-rw-r--r--  1 …     7 …  .xdp-original-firmado.pdf-rsefgK
-rw-rw-r--  1 …    35 …  original.pdf
```

Idéntico a lo que midió el #22: la escritura contesta OK, el arenero ve el
fichero, y en la carpeta del usuario queda un oculto `.xdp-…` que no se renombra
nunca. **`--filesystem=home` no toca esto en absoluto**, porque el fallo está en
el FUSE del portal de documentos y la ruta que se le pasa sigue siendo suya.

(Con un `Documents.Add` sin permiso de escritura explícito, la escritura del
hermano falla limpiamente con `PermissionError` en vez de fallar en silencio.
Es la variante benigna del mismo caso, no una defensa.)

## 5. El coste de la concesión, en una línea

**Un atacante que ya ejecuta código en rfirma pasa de leer los documentos del
usuario y dos almacenes de certificados a leer y escribir el `$HOME` entero:
claves SSH y GPG, el llavero, credenciales de la nube y de git, y toda la
configuración.**

Medido, mismo script con y sin el permiso:

| Ruta | `xdg-documents` | `--filesystem=home` |
|---|---|---|
| `~/.ssh` | no existe | **9 entradas** |
| `~/.gnupg` | no existe | **18 entradas** |
| `~/.local/share/keyrings` | no existe | **2 entradas** |
| `~/.aws`, `~/.kube` | no existen | **4 y 2 entradas** |
| `~/.git-credentials`, `~/.netrc` | no existen | **69 y 186 bytes** |
| `~/.config` | no existe | **128 entradas** |
| `~/.bash_history` | no existe | **3304 bytes** |
| `~/.pki/nssdb`, `~/.mozilla/firefox` | legibles `:ro` | legibles y **escribibles** |

Y hay un detalle que no es menor: `home` es de lectura **y escritura**, así que
convierte las dos excepciones `:ro` del ADR-0004 —`~/.mozilla/firefox` y
`~/.pki/nssdb`, concedidas en el #95 y el #101 precisamente con el `:ro` como
«la mitad importante»— en escribibles. El argumento de que esas excepciones ya
cubren datos más sensibles que los documentos no traslada: se concedieron sin
escritura, y `home` la da.

## Qué hacer con esto

- **El #221 no se disuelve.** La respuesta a su pregunta de partida es «no»:
  dentro del flatpak el diálogo devuelve la ruta del portal aunque se declare
  `--filesystem=home`, y flatpak y `.deb` siguen comportándose distinto.
- **El ADR-0011 se queda, con el argumento arreglado.** La frase «conservar
  junto al original exigiría saltarse el portal, que es lo que prohíbe el
  ADR-0004» dice lo correcto por el motivo equivocado. El motivo medido es
  doble, y ninguno de los dos es una prohibición de estilo: (a) el diálogo del
  que rfirma dispone **no puede** devolver la ruta real dentro del arenero, sea
  cual sea el permiso, porque GTK 3 enruta `GtkFileChooserNative` al portal de
  forma inanulable; y (b) aun con `home`, la aplicación **no puede averiguar** la
  carpeta del original, porque `Documents.Info` y `.Lookup` contestan
  `NotAllowed`. Merece la pena reescribir el párrafo citando este informe.
- **La puerta que sí existe**, por si alguna vez se reabre: un
  `GtkFileChooserDialog` propio, en proceso, más `--filesystem=home`. Devuelve
  rutas reales dentro de `home`. Cuesta una pieza de interfaz nativa mantenida
  a mano, deja fuera todo lo que no esté en `home`, y su precio en permisos es
  la tabla del apartado 5. No parece un cambio bueno para ahorrarse el aparato
  del #221, pero queda medido por si el contexto cambia.
