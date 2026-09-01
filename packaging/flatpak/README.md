# Empaquetado flatpak

`flatpak` es el **único canal de distribución** de rfirma
([ADR-0004](../../docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).

| Fichero | Qué es |
|---|---|
| `me.sgomez.rfirma.yml` | El manifiesto |
| `me.sgomez.rfirma.desktop` / `.metainfo.xml` | Entrada de menú y metadatos |
| `verifica.sh` | Verificación reproducible dentro del arenero |
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
arenero en el [#22](https://github.com/sgomez/rfirma/issues/22). Ahora empaqueta
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
packaging/flatpak/verifica.sh
```

`verifica.sh` comprueba lo que solo el arenero puede romper: que la librería
nativa esté —y **sola**, sin auxiliares de AWT—, que el módulo PKCS#11 que
empaqueta el propio flatpak esté ahí, y que la ventana arranque y siga viva. La
firma de punta a punta se mide fuera, con `just test-native`; recuperar ese paso
aquí dentro espera a que rfirma orqueste las tres fases.

## Pendiente antes de publicar

El canal es propio: bundle en GitHub Releases y repositorio ostree en
`rfirma.sgomez.me`. Ver el
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
