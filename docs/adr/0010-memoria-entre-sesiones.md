# Qué recuerda rFirma entre sesiones, y dónde lo guarda

El recorrido de firma validado en el
[ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md) da por hecho
que la aplicación recuerda cosas: la bandeja lista documentos recientes, y
Preferencias promete reutilizar la última configuración de firma visible. Nada
de eso estaba en el inventario cerrado de capacidades del prototipo, y AutoFirma
no sirve de oráculo porque **no tiene lista de recientes en absoluto** (sí
guarda la configuración de firma visible, en `java.util.prefs`, y la rúbrica
**como ruta**, que es justo lo que aquí se rechaza).

Se recuerdan **cinco cosas**, partidas en dos grupos según quién las decide.

**Configuración** (la elige el usuario): el idioma, dónde se guarda el documento
firmado, los interruptores, y el fichero de la rúbrica.

**Estado** (lo acumula la aplicación sola): los documentos recientes, la última
configuración de firma visible y el certificado usado la última vez.

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
  canónica bastaba; el arenero descrito en el
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
  bajo el arenero, y no se sustituye por otro. Firmar y luego no poder escribir
  obliga a explicar que el documento está firmado pero en ningún sitio.
- El formato de la rúbrica se valida al elegirla, no al firmar, porque los
  formatos admitidos se fijan en tiempo de construcción
  ([ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md)).
- El hito v0.1 es solo Linux; las columnas de macOS y Windows se registran ahora
  porque el momento de saberlo es antes de escribir `paths.rs`, no después.
