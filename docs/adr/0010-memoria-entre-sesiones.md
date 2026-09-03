# Qué recuerda rFirma entre sesiones, y dónde lo guarda

El recorrido de firma validado en el
[ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md) da por hecho
que la aplicación recuerda cosas: la bandeja lista documentos recientes, y
Preferencias promete reutilizar la última configuración de firma visible. Nada
de eso estaba en el inventario cerrado de capacidades del prototipo, y AutoFirma
no sirve de oráculo porque **no tiene lista de recientes en absoluto** (sí
guarda la configuración de firma visible, en `java.util.prefs`, y la rúbrica
**como ruta**, que es justo lo que aquí se rechaza).

Se recuerdan **seis cosas**, partidas en dos grupos según quién las decide.

**Configuración** (la elige el usuario): el idioma, el tema, dónde se guarda el
documento firmado, los interruptores, y el fichero de la rúbrica.

**Estado** (lo acumula la aplicación sola): los documentos recientes, la última
configuración de firma visible y el certificado usado la última vez. La enmienda
del final le suma la última carpeta abierta.

La distinción no es cosmética: en Windows el estado no debe viajar en un perfil
móvil y la configuración sí, y en Linux `XDG_STATE_HOME` existe exactamente para
esto. Borrar el estado no reconfigura la aplicación; borrar la configuración no
pierde el trabajo.

## Reglas que se derivan

- **Los recientes cachean metadatos**, no una ruta: nombre, insignia, `mtime` y
  fecha de último uso. Sin caché habría que parsear diez PDFs antes de pintar
  la bandeja, porque la insignia `Firmado`/`Sin firmar` no se deduce del
  identificador. Se revalida solo el documento que se selecciona, comparando
  el `mtime`.
- **Diez entradas, desalojo por último uso.** La bandeja no tiene buscador; si
  algún día hace falta uno, el límite estaba mal.
- **Un reciente se identifica por un identificador opaco, no por su ruta**
  (ID-62, [#82](https://github.com/sgomez/rfirma/pull/92)). Cuando se escribió
  este ADR la aplicación aún hablaba con el disco directamente y una ruta
  canónica bastaba; el sandbox descrito en el
  [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md) cambió esa
  premisa — bajo el portal de documentos la aplicación **nunca** conoce la ruta
  original, solo un identificador que acuña el backend al abrir el documento
  y que guarda el registro en memoria (`memory::opened`). Guardar una ruta
  habría sido guardar una mentira, y además es justo la fuga que cierra el
  [ADR-0011](0011-destino-del-documento-firmado.md): un identificador no se
  puede recorrer por fuerza bruta como una ruta.
- **Al firmar entran dos filas**, el original y el firmado, y el firmado pasa a
  ser el documento activo. Fusionarlos en una fila que «evoluciona» esconde que
  hay dos ficheros en el disco, que es lo que el usuario necesita saber para no
  mandar el equivocado.
- **Un documento que ya no responde no se purga en silencio**: la fila se
  atenúa con la insignia `No disponible` y ofrece quitarla. Un PDF en un USB
  desmontado, o cuyo permiso del portal ya no vale, no está borrado.
- **La rúbrica se copia**, no se referencia. Es una sola, y se sustituye al
  elegir otra. Guardar la ruta —lo que hace AutoFirma— pierde la rúbrica en
  silencio en cuanto el usuario mueve el PNG.
- **Del certificado se guarda cómo volver a encontrarlo, no quién es**: el
  módulo PKCS#11 y la etiqueta o ID en el token, nunca el titular ni el DNI. Se
  relee del token al arrancar; si no está, el panel vuelve a «Sin certificado»
  sin ruido.
- **Apagar «Recordar mi actividad» vacía el estado**, con confirmación. Conservar
  el fichero mientras la preferencia dice que no se recuerda nada incumple lo
  que promete el rótulo. «Vaciar la lista» sigue existiendo aparte: es «hoy no,
  mañana sí».
- **Apagar «Recordar la configuración de firma visible» significa no guardarla**,
  no guardarla y no aplicarla. Estado invisible que reaparece meses después al
  reencender el interruptor es peor que no tenerlo.
- **Escritura atómica** (temporal + `rename`) y `"version": 1` en ambos ficheros.
  Si no parsea o la versión es desconocida, se renombra a `.bak` y se arranca con
  los valores por omisión, avisando una vez. Una preferencia corrupta no puede
  impedir firmar.
- **El idioma sale del locale del sistema** cotejado contra los seis admitidos,
  con español como recurso. Sin diálogo de bienvenida que pregunte lo que la
  aplicación ya sabe.

## Las rutas son la implementación en Linux de tres nombres

Un único módulo, `paths.rs`, es el **único sitio del código con un `cfg!` de
sistema operativo**. Expone `config_file()`, `state_file()` y `rubric_path()`;
el resto de la aplicación no sabe qué sistema hay debajo, y añadir macOS o
Windows toca un fichero. Se apoya en el resolutor de rutas de Tauri v2, que ya
está en el proyecto: ni `directories` ni Tauri ofrecen un directorio de estado
fuera de Linux, así que esa parte la pone el módulo.

|         | configuración                          | estado                     | rúbrica                |
| ------- | -------------------------------------- | -------------------------- | ---------------------- |
| Linux   | `$XDG_CONFIG_HOME/rfirma/`             | `$XDG_STATE_HOME/rfirma/`  | `$XDG_DATA_HOME/rfirma/` |
| Windows | `%APPDATA%\rfirma\`                    | `%LOCALAPPDATA%\rfirma\`   | `%APPDATA%\rfirma\`    |
| macOS   | `~/Library/Application Support/rfirma/` | el mismo, otro fichero     | el mismo               |

Formato JSON en los dos ficheros, porque los escribe la aplicación y nadie los
edita a mano. macOS no distingue configuración de estado y ahí la separación se
colapsa a dos ficheros en el mismo directorio; Windows encaja mejor que Linux,
porque `%LOCALAPPDATA%` impide solo que una lista de rutas locales viaje por la
red.

## Consequences

- La bandeja gana un tercer valor de insignia, `No disponible`, que el
  vocabulario de dos valores de su ficha no contemplaba.
- Preferencias gana un interruptor, «Recordar mi actividad», y un botón «Vaciar
  la lista». El interruptor cubre recientes **y** certificado: son la misma
  promesa al usuario del ordenador compartido.
- La carpeta de destino se comprueba **antes de firmar**, no al guardar. Qué
  pasa cuando no está lo fija el
  [ADR-0011](0011-destino-del-documento-firmado.md), que retira la degradación
  «junto al documento original» que aquí se describía: ese destino no existe
  bajo el sandbox, y no se sustituye por otro. Firmar y luego no poder escribir
  obliga a explicar que el documento está firmado pero en ningún sitio.
- El formato de la rúbrica se valida al elegirla, no al firmar, porque los
  formatos admitidos se fijan en tiempo de construcción
  ([ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md)).
- El hito v0.1 es solo Linux; las columnas de macOS y Windows se registran ahora
  porque el momento de saberlo es antes de escribir `paths.rs`, no después.

## Enmienda: el tema

La configuración gana una sexta memoria, **el tema** (`system`, `light`,
`dark`), que no estaba cuando se escribió este ADR. Es del grupo de la
configuración por la misma razón que el idioma: lo elige el usuario y la
aplicación obedece. Por omisión vale `system`, que **no es «claro»** sino no
forzar nada y dejar que mande `prefers-color-scheme`: es lo que hacía la ventana
antes de que el ajuste existiera, y abrir en claro dentro de un escritorio
oscuro parece un fallo.

## Enmienda: la última carpeta abierta

El estado gana una memoria más, **la última carpeta de la que se abrió un
documento**, para que el diálogo de abrir vuelva a aparecer ahí. La decide la
enmienda del [ADR-0011](0011-destino-del-documento-firmado.md); lo que le toca a
este es dónde vive y qué se la lleva.

Es **estado y no configuración**: nadie la elige, la acumula la aplicación sola
al abrir documentos. Así que va al fichero de `XDG_STATE_HOME` y **«Recordar mi
actividad» se la lleva** igual que a los recientes y al certificado —es la misma
promesa a quien firma en un ordenador compartido, y una carpeta del anfitrión
que sobreviviera a «Vaciar la lista» diría por dónde anduvo el anterior—.

En el flatpak vale `None` siempre: el portal no dice de qué carpeta salió el
documento, así que no hay nada que apuntar. No es un campo que se apague por
prudencia, es uno que allí nunca llega a tener valor.

## Enmienda: la ventana, y la posición del recuadro por documento

Añadido con el hito v0.2 ([#123](https://github.com/sgomez/rfirma/issues/123)).
Son dos cosas: una memoria nueva y un reparto distinto de una que ya estaba.

### El tamaño de la ventana es una memoria más

El estado gana **el tamaño de la ventana y si estaba maximizada**. Va en el
mismo `state.json` que el resto del estado, escrito por la aplicación con el
mismo `paths.rs`, y **no** con `tauri-plugin-window-state`: ese plugin trae su
propio fichero, su propio formato y su propio momento de escritura, que es una
cuarta ruta de persistencia al lado de las tres que este ADR se ha molestado en
nombrar.

**La posición no se repone en Wayland**, y por tanto no se repone en ningún
canal. El protocolo no deja a una aplicación colocarse: `xdg_toplevel` no tiene
ninguna petición de posición, y el compositor decide. Guardar unas coordenadas
que en el escritorio objetivo no se pueden aplicar es guardar un campo que
miente en el sitio donde más se usa; que en X11 funcionara no basta para que la
misma memoria signifique dos cosas distintas según la sesión.

**«Recordar mi actividad» no se lleva el tamaño de la ventana**, y es la única
excepción del grupo de estado. Lo demás que hay ahí —recientes, certificado,
última carpeta— dice **qué** hizo el anterior; el tamaño de una ventana no dice
nada de nadie. Y quien vacía su lista de recientes para no dejar rastro no
espera que además la ventana se le encoja al tamaño de fábrica, que se lee como
un fallo y no como una promesa cumplida. Es un juicio y no una medición: si
alguien encuentra un caso en el que el tamaño delate algo, esta viñeta es la que
hay que tirar.

### La posición del recuadro se recuerda por documento, no en Preferencias

Este ADR guardaba «la última configuración de firma visible» como una sola cosa
del grupo de estado. Se parte en dos:

- **Global**, y gobernado por el interruptor de Preferencias: el propio
  interruptor de firma visible, las cinco casillas, el motivo y el tamaño del
  recuadro. Eso es lo que se reutiliza en el siguiente documento.
- **Por documento**: la **página y la posición** del recuadro, guardadas en la
  fila de recientes de ese documento.

Reponer sobre un documento nuevo una posición elegida para otro es lo que
rechaza el ID-22: el recuadro acaba fuera de página, o encima del texto, y el
usuario tiene que arreglarlo cada vez. Un recuadro colocado a mano en la
página 3 de un contrato no dice nada de dónde va en un modelo 036.

Cuesta acoplar la ficha 3 a la 2 —sin fila de recientes no hay dónde guardar la
posición—, y se acepta: es el mismo sitio donde ya viven los metadatos de ese
documento, y desaparece con él cuando se olvida o se vacía la lista, sin ninguna
regla de caducidad nueva.
