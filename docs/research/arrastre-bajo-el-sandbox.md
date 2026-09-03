# Qué llega al soltar un fichero en la ventana, bajo el sandbox: medición

Medición del **ID-69** para el issue
[#83](https://github.com/sgomez/rfirma/issues/83). El spec
[#81](https://github.com/sgomez/rfirma/issues/81) prometió un aviso que dijera
**qué hacer** cuando un fichero soltado no se pueda leer (ID-68), y puso como
condición que antes se midiera si eso pasa de verdad y con qué rutas. Esto es
esa medición: **no es una suposición sobre lo que el sandbox deja pasar**.

Entorno: Ubuntu 26.04, flatpak 1.16.6, `org.gnome.Platform//50` (GTK **3.24.52**,
WebKitGTK 4.1), sesión GNOME sobre Wayland, `xdg-desktop-portal` del anfitrión.
El bundle medido es el `me.sgomez.rfirma` instalado, con los permisos del
manifiesto tal cual: `--filesystem=xdg-documents` y ninguno más.

## Veredicto

**Sí llega algo utilizable, y desde cualquier carpeta.** La promesa de arrastrar
se queda: no hay que subir nada a producto.

Pero llega **por dos caminos distintos**, y solo uno de los dos funciona desde
fuera de la carpeta de documentos:

| Origen del arrastre | Lo que llega | ¿Se puede leer? |
|---|---|---|
| Habla el portal `FileTransfer` (Nautilus y cualquier GTK moderno), fichero **fuera** de `~/Documents` | `/run/user/1000/doc/1e20dd88/fichero.pdf` | **Sí** |
| Habla el portal, fichero **dentro** de `~/Documents` | la ruta del anfitrión tal cual | **Sí** |
| **No** habla el portal, fichero **dentro** de `~/Documents` | la ruta del anfitrión tal cual | **Sí** |
| **No** habla el portal, fichero **fuera** de `~/Documents` | la ruta del anfitrión tal cual | **No**: `ENOENT` |

O sea: **la última fila es el aviso del ID-68**, y existe. No es un caso
teórico, y tampoco es el caso corriente: arrastrar desde el explorador de
archivos del escritorio cae en las dos primeras filas.

Por eso el mensaje no puede decir «el fichero no existe» —existe, y la persona
lo está viendo en su pantalla— sino **qué hacer**: abrirlo con el botón de
abrir, que sí pasa por el portal de ficheros y concede la lectura.

## 1. Dentro del sandbox solo está la carpeta de documentos

Lo primero, porque es la mitad de la respuesta:

```
$ flatpak run --command=sh me.sgomez.rfirma -c 'ls -A $HOME; ls -A /run/user/1000'
Documents
.local
.var

app  bus  doc  .flatpak  flatpak-info  p11-kit  wayland-0
```

`$HOME` tiene **una** carpeta del usuario, que es lo que concede
`--filesystem=xdg-documents`. Cualquier otra ruta del anfitrión —`~/Downloads`,
`~/Desktop`, un pendrive— **no existe** dentro: no es que dé permiso denegado,
es que da `ENOENT`.

Y está montado `/run/user/1000/doc`, el FUSE del portal de documentos, que es
por donde entra todo lo demás.

## 2. El arrastre pasa por el portal `FileTransfer`, y eso no lo hacemos nosotros

La cadena, comprobada eslabón a eslabón:

1. **El origen** (Nautilus, GTK4) publica los ficheros con
   `org.freedesktop.portal.FileTransfer.StartTransfer` + `AddFiles` y ofrece en
   el arrastre el tipo `application/vnd.portal.filetransfer`, cuyo contenido es
   solo una clave.
2. **WebKitGTK** registra los tipos de destino del WebView con
   `gtk_target_list_add_uri_targets()` — el símbolo está en
   `libwebkit2gtk-4.1.so.0`, comprobado—, y esa función de GTK añade
   `application/vnd.portal.filetransfer` **además** de `text/uri-list` cuando el
   portal responde (`gtkselection.c`, desde 3.24.37).
3. **wry** —`wry-0.55.1/src/webkitgtk/drag_drop.rs`— lee las rutas con
   `data.uris()`, o sea `gtk_selection_data_get_uris()`.
4. **GTK** ve que el tipo de la selección es el del portal, llama a
   `file_transfer_portal_retrieve_files_sync()` y devuelve las rutas ya
   exportadas. Todo esto ocurre **dentro de la llamada**: ni wry ni rfirma se
   enteran.

El resultado es que a Tauri le llegan `PathBuf` normales y corrientes, y en el
caso interesante son rutas de `/run/user/1000/doc/`.

La cadena está medida de punta a punta y sin GUI: el anfitrión hace de origen
(`StartTransfer` + `AddFiles`, que es lo que hace Nautilus) y dentro del sandbox
se hace de destino (`RetrieveFiles`, que es lo que hace GTK):

```
clave de la transferencia: 1752544301385907672514389025651046275789
LLEGA  /run/user/1000/doc/1e20dd88/rfirma-id69-fuera.pdf
  LEIBLE si  -> b'%PDF-1.4 medicion-id69-fuera\n'
LLEGA  /home/sergio/Documents/rfirma-id69-dentro.pdf
  LEIBLE si  -> b'%PDF-1.4 medicion-id69-dentro\n'
CRUDA  /home/sergio/Downloads/rfirma-id69-fuera.pdf
  EXISTE False  LEIBLE no  -> [Errno 2] No such file or directory
CRUDA  /home/sergio/Documents/rfirma-id69-dentro.pdf
  EXISTE True  LEIBLE si  -> b'%PDF-1.4 medicion-id69-dentro\n'
```

Las dos primeras filas son lo que llega **por el portal**; las dos últimas, las
rutas del anfitrión tal cual, que es lo que llegaría si el origen no hablara el
portal.

Dos detalles que importan al código:

- **El nombre del fichero sobrevive** dentro de la ruta exportada
  (`…/1e20dd88/rfirma-id69-fuera.pdf`), así que `PortalDocument::name()` sigue
  sacando el nombre del último segmento, igual que con el diálogo.
- **Un fichero que ya se ve dentro del sandbox no se exporta**: el portal
  devuelve su ruta del anfitrión sin más. No hay nada que hacer al respecto; las
  dos formas se leen igual.

## 3. Fuera del sandbox (`just dev`) pasa lo mismo

`file_transfer_portal_supported()` no comprueba si hay sandbox: comprueba si
`org.freedesktop.portal.Documents` contesta en el bus. En un escritorio GNOME
contesta siempre, así que **`just dev` toma el mismo camino**, solo que para un
llamante sin sandbox el portal devuelve las rutas reales:

```
LLEGA  /home/sergio/Downloads/rfirma-id69-fuera.pdf
  LEIBLE si  -> b'%PDF-1.4 medicion-id69-fuera\n'
LLEGA  /home/sergio/Documents/rfirma-id69-dentro.pdf
  LEIBLE si  -> b'%PDF-1.4 medicion-id69-dentro\n'
```

Es una buena noticia y una trampa a la vez: **probar el arrastre en `just dev`
no prueba el caso del ID-68**, porque ahí fuera todo se lee. El único sitio
donde se ve el fallo es dentro del bundle, arrastrando desde un origen que no
hable el portal.

## 4. Qué se hace con esto

- El mensaje del ID-68 se redacta **para la cuarta fila de la tabla** y dice qué
  hacer: usar el botón de abrir. Es
  `errors.situations.droppedFileUnreadable` en los seis catálogos.
- La comprobación en Rust es un intento de **abrir** el fichero soltado, no un
  `exists()`: la diferencia entre `ENOENT` y un permiso denegado no le importa a
  quien firma, y las dos acaban en el mismo aviso con su detalle técnico crudo
  al lado (ID-29).
- Nada de esto se puede reproducir en el CI: no hay portal, no hay escritorio y
  no hay quien arrastre. Lo que sí corre en grada A es la decisión
  —`dropped::first_pdf`, con rutas de mentira— y el comportamiento de la ventana
  contra el doble del puerto.

## Reproducirlo

Los tres guiones de la medición no se versionan: son veinte líneas de
`Gio.DBusConnection` cada uno y están descritos arriba lo bastante para
reescribirlos. Lo que hay que montar es:

1. Un fichero en `~/Documents` y otro fuera, por ejemplo en `~/Downloads`.
2. Desde el anfitrión, `StartTransfer` con `autostop: false` y `AddFiles` con
   los descriptores de los dos, **sin que el proceso termine**: la transferencia
   muere con la conexión de quien la abrió.
3. Con esa clave, dentro del sandbox
   (`flatpak run --command=python3 me.sgomez.rfirma`), `RetrieveFiles` y un
   `open()` sobre cada ruta que devuelva; y, para el contraste, un `open()`
   sobre las dos rutas del anfitrión tal cual.
