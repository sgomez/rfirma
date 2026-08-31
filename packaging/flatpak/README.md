# Empaquetado flatpak

`flatpak` es el **único canal de distribución** de rfirma
([ADR-0004](../../docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).

| Fichero | Qué es |
|---|---|
| `me.sgomez.rfirma.yml` | El manifiesto |
| `me.sgomez.rfirma.desktop` / `.metainfo.xml` | Entrada de menú y metadatos |
| `verifica.sh` | Verificación reproducible dentro del arenero |

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

- **Construir sin red.** El módulo `rfirma-app` declara `--share=network` para
  bajar las dependencias de cargo. Se vendorizan con
  `flatpak-cargo-generator.py` a partir del `Cargo.lock`, y el frontend con
  `flatpak-node-generator` a partir del `pnpm-lock.yaml` (`just
  flatpak-sources`), que es además lo que permitiría construirlo dentro del
  arenero en vez de traerlo hecho del anfitrión. Ya no lo obliga Flathub: lo
  decidió por su cuenta el
  [ADR-0013](../../docs/adr/0013-estructura-del-repositorio-y-cadena-de-compilacion.md),
  para que un fichero generado no entre en el CI sin que nadie lo mire.
- **Publicar el repositorio ostree.** `flatpak build-export` +
  `flatpak build-update-repo`, firmado con GPG, servido como ficheros estáticos,
  más el `.flatpakref` con la huella de la clave.
