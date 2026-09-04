# Empaquetado flatpak

`flatpak` es **uno de los tres canales de distribución** de rfirma, junto al
`.deb` y el `.rpm`
([ADR-0004](../../docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).
Los nativos no se empaquetan aquí: los produce el *bundler* de Tauri
([ADR-0013](../../docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)).

| Fichero | Qué es |
|---|---|
| `me.sgomez.rfirma.yml` | El manifiesto |
| `me.sgomez.rfirma.desktop` / `.metainfo.xml` | Entrada de menú y metadatos |
| `verifica.sh` | Verificación reproducible dentro del sandbox |
| [`../verifica-contenido.sh`](../verifica-contenido.sh) | La invariante del ADR-0012 (un solo `librfirma_crypto.so`, `libawt.so` en ninguna parte), independiente del formato |
| `cargo-sources.json` / `node-sources.json` | Dependencias vendorizadas, generadas |
| `sources.lock` | El sello que dice contra qué ficheros de bloqueo se generaron |
| `check-sources.sh` | Falla si esas fuentes se han quedado atrás |

## Instalar

El bundle **no trae el runtime**: sale del remoto de **Flathub**, que es
requisito de instalación.

```bash
flatpak remote-add --user --if-not-exists \
    flathub https://dl.flathub.org/repo/flathub.flatpakrepo
just flatpak
flatpak install --user packaging/flatpak/me.sgomez.rfirma.flatpak
```

`just flatpak` construye contra un repositorio ostree local (`repo/`, sin
versionar) y de ahí saca `me.sgomez.rfirma.flatpak`, que es **el entregable del
v0.1** (ID-42). No se publica en ningún sitio.

## La sonda ya no está

Hasta el [#56](https://github.com/sgomez/rfirma/issues/56) el manifiesto
empaquetaba `probe/`, una aplicación Tauri mínima escrita para *medir* el
sandbox en el [#22](https://github.com/sgomez/rfirma/issues/22). Ahora empaqueta
`rfirma-app`, y la sonda se ha borrado: contenía FFI, carga de la librería
nativa y PKCS#11, o sea una **segunda implementación de la frontera FFI**, que
es justo donde este proyecto lleva tres hallazgos de fallo silencioso
([ADR-0013](../../docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md)).
Lo que midió está escrito en
[`docs/research/flatpak-canal-unico.md`](../../docs/research/flatpak-canal-unico.md).

El resto del manifiesto —runtime, permisos, la librería en `/app/lib/rfirma`,
pcsc-lite y OpenSC— se quedó tal cual.

## Verificar

El frontend y la librería nativa se construyen **en el anfitrión** y entran ya
construidos, así que van primero:

```bash
export GRAALVM_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce
just native
just build-ts
just token       # el paso 4 firma con el token de la grada B
packaging/flatpak/verifica.sh
```

`verifica.sh` da siete pasos. Dentro del sandbox comprueba lo que solo el
sandbox puede romper: que el módulo PKCS#11 que empaqueta el propio flatpak
cargue, que la ventana arranque y siga viva, que un documento entrado por el
portal llegue con sus bytes intactos, y que el sandbox **rechace escribir** en
el perfil de Firefox y en `~/.pki/nssdb` — los dos únicos `--filesystem` que no
van por portal (#101, AC 3). La invariante del ADR-0012 —un solo
`librfirma_crypto.so`, `libawt.so` en ninguna parte— ya no vive aquí: la
comprueba [`../verifica-contenido.sh`](../verifica-contenido.sh), independiente
del formato, sobre el `.flatpak` construido (o el `.deb`/`.rpm`, cuando
existan).

El paso 4 corre el **ciclo trifásico completo con rúbrica de imagen** y lo valida
con `pdfsig`, contra la librería **instalada en el bundle** — los bytes que se
distribuyen, no los del árbol de construcción. Eso es lo que faltaba: la
verificación del [#22](https://github.com/sgomez/rfirma/issues/22) se corrió
contra la imagen de **seis** ficheros, y la rúbrica de imagen es justo el caso
cuyo comportamiento depende de qué `.so` haya al lado. Necesita el token de la
grada B (`just token`) y `poppler-utils`.

Ese paso se ejecuta en el anfitrión apuntando a la librería del bundle, y no
dentro del sandbox, por tres razones medidas: dentro **no hay token** (el bundle
empaqueta OpenSC para una tarjeta física, y montar el SoftHSM del anfitrión es
justo el `LD_LIBRARY_PATH` de otra glibc que prohíbe el ID-40), **no hay
poppler** (`pdfsig` no está ni en el bundle ni en `org.gnome.Platform//50`), y
**no hay por dónde entrar** (rfirma no tiene modo headless: el ciclo solo se
alcanza por los `#[tauri::command]` desde el WebView, y un binario de prueba del
anfitrión tampoco sirve de puente, porque aquí la glibc es 2.43 y la del runtime
2.42). Meter SoftHSM, poppler y un binario de prueba dentro sería distribuir el
banco de pruebas y romper «los permisos son los declarados y ninguno más»;
cerrarlo pide un manifiesto de banco aparte.

## Pendiente antes de publicar

El canal es propio: paquetes en GitHub Releases y **tres** repositorios en
`rfirma.sgomez.me` —ostree, apt y dnf—. Ver el
[ADR-0015](../../docs/adr/0015-canal-de-distribucion-propio.md).

- **Publicar el repositorio ostree.** `flatpak build-export` +
  `flatpak build-update-repo`, firmado con GPG, servido como ficheros estáticos,
  más el `.flatpakref` con la huella de la clave.

## Construir sin red

Ya está hecho: **no hay `--share=network` en el manifiesto**. Las dependencias
de cargo entran vendorizadas desde `cargo-sources.json`, y `cargo build` corre
con `--offline`.

Los dos generadores son de
[flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) y **no
se versionan aquí** (ID-04): se traen a mano la primera vez.
`just flatpak-sources` falla nombrando el que falte.

```bash
just flatpak-sources   # cuando cambie Cargo.lock o pnpm-lock.yaml
```

Esa receta regenera los dos JSON **y** reescribe `sources.lock` con el `sha256`
de cada fichero de bloqueo. El CI no los regenera: ejecuta
`just check-flatpak-sources` (dentro de `just lint`, y por tanto de `just
check`), que compara esos `sha256` y falla nombrando el fichero que se ha
movido. Un fichero generado dentro del CI es un fichero que nadie ha mirado
([ID-07](https://github.com/sgomez/rfirma/issues/46)).

`node-sources.json` se genera y se versiona, pero **el manifiesto todavía no lo
usa**: el frontend se construye en el anfitrión y entra hecho, porque
`org.gnome.Sdk//50` no trae `node`. Consumirlo pide añadir la extensión de SDK
`org.freedesktop.Sdk.Extension.node22`, que es otra decisión. Mientras tanto lo
cubre la misma puerta, para que no se pudra en silencio.
