# Empaquetado flatpak

`flatpak` es el **único canal de distribución** de rfirma
([ADR-0004](../../docs/adr/0004-libreria-nativa-distribuida-en-el-paquete.md)).

| Fichero | Qué es |
|---|---|
| `me.sgomez.rfirma.yml` | El manifiesto |
| `me.sgomez.rfirma.desktop` / `.metainfo.xml` | Entrada de menú y metadatos |
| `verifica.sh` | Verificación reproducible de punta a punta |
| `probe/` | **Sonda desechable**, no es rfirma (ver abajo) |

## La sonda

Mientras `rfirma-app` no exista, el manifiesto empaqueta una aplicación Tauri
mínima que sirve para *medir* el arenero: carga la librería nativa, ejecuta el ciclo
trifásico completo con rúbrica de imagen, firma el PK1 con PKCS#11 e informa de
lo que ve dentro (portales, WebKitGTK, glibc, permisos). Se construyó para el
[#22](https://github.com/sgomez/rfirma/issues/22).

Cuando exista la aplicación, se sustituye el módulo `sonda` del manifiesto por
`rfirma-app` y se borra `probe/`. El resto del manifiesto —runtime, permisos,
la librería en `/app/lib/rfirma`, pcsc-lite y OpenSC— se queda tal cual.

## Verificar

```bash
export GRAALVM_HOME=~/.sdkman/candidates/java/25.3.4+1.r25-graalce
rfirma-native-bridge/testbench/build-native-fonts.sh ce25-noui
packaging/flatpak/verifica.sh
```

Lo medido y las decisiones que salen de ello están en
[`docs/research/flatpak-canal-unico.md`](../../docs/research/flatpak-canal-unico.md).

## Pendiente antes de publicar en Flathub

- **Construir sin red.** El módulo `sonda` declara `--share=network` para bajar
  las dependencias de cargo. Flathub no lo permite: hay que vendorizarlas con
  `flatpak-cargo-generator.py` a partir del `Cargo.lock`.
