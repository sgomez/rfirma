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
| `verifica.sh` | Prueba de humo manual (`just flatpak-smoke`): ventana, portal, almacenes NSS y ciclo trifásico |
| [`../verifica-contenido.sh`](../verifica-contenido.sh) | La invariante del ADR-0012 (un solo `librfirma_crypto.so`, `libawt.so` en ninguna parte), independiente del formato |
| `cargo-sources.json` | Dependencias de cargo vendorizadas, generadas |
| `sources.lock` | El sello del `Cargo.lock` contra el que se generó |
| `check-sources.sh` | Falla si esas fuentes se han quedado atrás |

OpenSC y `pcsc-lite` llevan `x-checker-data` en el manifiesto, y el workflow
`flatpak-versions.yml` (lunes y a mano) sale en rojo cuando se publica una
versión nueva de cualquiera de los dos. La subida es a mano, a la siguiente
versión publicada y nunca a un commit de `master`.

## Instalar

El bundle **no trae el runtime**, pero lleva dentro la dirección de **Flathub**
(`--runtime-repo`): al instalarlo, flatpak ofrece añadir ese remoto y descarga
`org.gnome.Platform` sin configurar nada antes. En el equipo solo hacen falta
`flatpak` y `xdg-desktop-portal`.

```bash
just flatpak
flatpak install --user packaging/flatpak/me.sgomez.rfirma.flatpak
```

`just flatpak` construye contra un repositorio ostree local (`repo/`, sin
versionar) y de ahí saca `me.sgomez.rfirma.flatpak`, que es **el entregable del
v0.1** (ID-42). No se publica en ningún sitio.

## Ficheros fuera de `~/Documents`

El manifiesto no abre ninguna otra carpeta de documentos que `xdg-documents`. Para
la línea de órdenes con un fichero de otro sitio, `--file-forwarding` y `@@`
(que delimitan cada ruta) lo exponen a través del portal de documentos:

```bash
flatpak run --file-forwarding me.sgomez.rfirma sign -i @@ ~/Descargas/contrato.pdf @@ -o ~/Documents/firmado.pdf
```

El fichero de salida de este ejemplo cae en `~/Documents`, que sí está abierto.

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

El resto del manifiesto —runtime, permisos, la librería en `/app/lib/rfirma`—
se quedó tal cual. Las tarjetas y el DNIe ([ADR-0049](../../docs/adr/0049-el-flatpak-trae-su-opensc-y-usa-el-pcscd-del-anfitrion.md))
usan el `pcscd` del anfitrión por `--socket=pcsc` y el OpenSC que lleva dentro
el flatpak. El anfitrión necesita `pcscd` y el driver de su lector, `libccid` en
la mayoría de los casos.

## Verificar

El frontend y la librería nativa se construyen en el anfitrión y entran ya
construidos, así que van primero:

```bash
export GRAALVM_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce
just certs install       # el paso 3 firma con el token de la grada B
just flatpak-smoke
```

`just flatpak-smoke` (`verifica.sh`) da cinco pasos: construye e instala el
flatpak, comprueba que la ventana arranca y sigue viva, corre el ciclo
trifásico completo con rúbrica de imagen contra la librería instalada en el
bundle y lo valida con `pdfsig`, comprueba que un documento entrado por el
portal llega con sus bytes intactos y que el sandbox **sí** puede escribir en el
perfil de Firefox y en `~/.pki/nssdb` (ADR-0005). La invariante del ADR-0012
—un solo `librfirma_crypto.so`, `libawt.so` en ninguna parte— no se repite aquí:
la aplica `verify-packages` sobre cada paquete.

El ciclo trifásico corre en el anfitrión y no dentro del sandbox: dentro no hay
token PKCS#11, ni `pdfsig`, ni forma de invocar los comandos sin el WebView.
Necesita el token de la grada B y `poppler-utils`.

## Pendiente antes de publicar

El canal es propio: paquetes en GitHub Releases y **tres** repositorios en
`rfirma.sgomez.me` —ostree, apt y dnf—. Ver el
[ADR-0015](../../docs/adr/0015-canal-de-distribucion-propio.md).

- **Publicar el repositorio ostree.** `flatpak build-export` +
  `flatpak build-update-repo`, firmado con GPG, servido como ficheros estáticos,
  más el `.flatpakref` con la huella de la clave.

## Construir sin red

Ya está hecho: **ningún módulo declara `--share=network`**. El único
`--share=network` del manifiesto está en `finish-args` —permiso de la
aplicación ya instalada, para la consulta de versión a GitHub (#270)— y no
alcanza a la construcción. Las dependencias
de cargo entran vendorizadas desde `cargo-sources.json`, y `cargo build` corre
con `--offline`.

El generador es de
[flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) y **no
se versiona aquí**: se trae a mano la primera vez, y hace falta
[uv](https://docs.astral.sh/uv/), que resuelve sus dependencias.
`just flatpak-sources` falla nombrando lo que falte y el comando que lo trae:

```bash
curl -fsSL -o packaging/flatpak/flatpak-cargo-generator.py \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
```

```bash
just flatpak-sources   # cuando cambie Cargo.lock
```

Esa receta regenera `cargo-sources.json` **y** reescribe `sources.lock` con el
`sha256` de `Cargo.lock`. El CI no los regenera: `just check-repo` ejecuta
`packaging/flatpak/check-sources.sh`, que compara ese `sha256` y falla si se ha
movido. Un fichero generado dentro del CI es un fichero que nadie ha mirado.

### Cuando `just flatpak-sources` no corre

En un entorno sin red `flatpak-cargo-generator.py` no está y `pip install
aiohttp` no resuelve contra PyPI. Entonces `cargo-sources.json` se reproduce a
mano: dos entradas por crate de `registry+…crates.io-index`, ordenadas por
nombre y por **versión semver** (`0.9.6` antes que `0.10.2`, no orden
lexicográfico). El `sha256` de cada crate **no se calcula**: ya está en
`Cargo.lock`, en el campo `checksum` de ese paquete, y es el mismo valor que va
en la entrada `archive` y dentro de la `inline`. Lo único que sí se sella con
`sha256sum` es `sources.lock`, con el hash de `Cargo.lock`.

Si el cambio es solo **hacer directa una dependencia que ya estaba en el árbol
transitivo**, `Cargo.lock` cambia en una sola línea y `cargo-sources.json` no
se toca: basta con regenerar el sha de `sources.lock`.

### Medir el sandbox sin GUI

`flatpak run --command=python3 me.sgomez.rfirma -` mete un script por la
entrada estándar dentro del bundle ya instalado, con sus permisos reales.
`org.gnome.Platform` trae `python3` con PyGObject y `gdbus`, pero **no**
`strings` ni `busctl`.

El frontend se construye en el anfitrión y entra hecho en `rfirma-app/dist`:
`org.gnome.Sdk//50` no trae `node`, y el ADR-0013 explica por qué no se vendoriza
npm.
