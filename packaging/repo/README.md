# rfirma.sgomez.me: la landing y la publicación

La imagen que sirve `https://rfirma.sgomez.me` es **Caddy (`caddy:alpine`) con esta landing, y nada
más** ([ADR-0015](../../docs/adr/0015-canal-de-distribucion-propio.md)). Los tres
repositorios de paquetes (ostree, apt, dnf) no van dentro de la imagen: los publica un
montaje de directorio del anfitrión aparte, por `rsync`, y Caddy los sirve a través de un
enlace simbólico.

| Fichero | Qué es |
|---|---|
| `index.html` | La landing, escrita a mano, sin generador y sin paso de construcción |
| `rfirma-main-window.png` | Captura real de la interfaz principal de rFirma |
| `Caddyfile` | Configuración de Caddy (no-root, puerto 3000, cabeceras, healthcheck y las rutas de los tres repositorios) |
| `Dockerfile` | `caddy:alpine` no-root más la landing y Caddyfile |
| `download-series.sh` | Baja y verifica **toda** la serie menor vigente desde las Releases |
| `build-tree.sh` | Reconstruye los tres repositorios enteros, desde cero, en un directorio nuevo |
| `publish-tree.sh` | Sube el árbol al anfitrión e intercambia el enlace `actual` |
| `*.test.sh` | Las pruebas de los dos anteriores; `just check-publish` las corre |

## Cómo entra en servicio una versión

`publish.yml` reacciona a una Release **publicada** (no a la etiqueta, y nunca a una
candidata), comprueba su firma y encadena los tres guiones. Lo que queda en el anfitrión:

```
/srv/rfirma-repo/
├── arboles/
│   ├── v0.4.0/          el árbol anterior, entero: la vuelta atrás es reapuntar el enlace
│   └── v0.4.1/          el árbol nuevo, reconstruido desde las Releases
└── actual -> arboles/v0.4.1
```

Y el orden, que es todo el mecanismo:

1. **el árbol nuevo entero**, a un directorio suyo (`rsync --archive --delete`). Mientras
   esto no termina, nadie ve nada: `actual` sigue apuntando al árbol de antes;
2. **el enlace `actual`**, enviado como enlace simbólico y renombrado encima del anterior.
   Es el único gesto que cambia lo servido, y es atómico;
3. **la poda**, que deja el vigente y el anterior. Va después del intercambio para que un
   fallo nunca borre lo que se está sirviendo.

El árbol es **derivado**: la fuente de verdad son las Releases, que no se borran nunca. Se
puede tirar `/srv/rfirma-repo` entero y volver a publicar; el servicio queda idéntico.

## Qué hay dentro del árbol

```
rfirma.asc                          la clave pública: el Signed-By de apt y el gpgkey de dnf
rfirma.flatpakref                   instalación de un clic, con la clave dentro
flatpak/                            el repositorio ostree (modo archive), firmado
apt/pool/main/r/rfirma/*.deb        toda la serie menor vigente
apt/dists/stable/                   Release, InRelease, Release.gpg y main/binary-amd64/
apt/rfirma.sources                  el deb822 de la landing, servido para descargarlo
rpm/*.rpm                           toda la serie, con la firma ya dentro de cada paquete
rpm/repodata/                       el índice y su repomd.xml.asc
rpm/rfirma.repo                     el .repo de la landing, con la URL literal
```

Tres cosas que no son de estilo:

- **Reconstruir no obliga a nadie a redescargar** (ID-173). Importar los mismos bundles en un
  ostree vacío da el **mismo commit**, así que un cliente ya instalado no ve nada nuevo. Para
  que eso siga siendo cierto hacen falta los tres cabos: `ostree init` delante,
  **todos** los bundles de la serie y en orden de versión —la historia se trunca a lo que se
  importe— y **re-firmar siempre**, porque la firma es metadato desacoplado que no viaja
  dentro del bundle. Lo comprueba `build-tree.test.sh` construyendo el árbol dos veces.
- **apt con suite `stable`, no repositorio plano** (ID-175). El plano es más barato y no
  admite `Suites:`/`Components:` en un `.sources` deb822, que es el formato obligado para que
  la clave vaya en `Signed-By` sin `apt-key`, retirado.
- **Los `.rpm` llegan aquí ya firmados.** Firmar un `.rpm` lo modifica, así que se firma en
  `release.yml` —antes del `SHA256SUMS` y antes de la atestación—; aquí sólo se rechaza el
  que venga sin firma. El orden de esos pasos lo vigila `just check-actions`.

**Las firmas del árbol no las prueba nadie automáticamente**, y no puede ser de otra manera:
firmar necesita una clave privada, las de rFirma las crea una persona con
`packaging/setup-signing-key.sh` y ninguna prueba puede fabricarse una que valga. Por eso
`build-tree.sh` tiene un modo `SIN-FIRMA-SOLO-PRUEBAS` que es el que usa su test, y por eso
`just check-actions` prohíbe que esa cadena aparezca en un workflow. El camino con clave se
ensaya con una etiqueta `v*-rc.N`.

## Las pruebas: `just check-publish`

`publish-tree.sh` es la única parte de la tubería que **no** puede ensayarse con una etiqueta
`v*-rc.N` —el ensayo se detiene justo antes de tocar el anfitrión—, así que se prueba aquí.
`publish-tree.test.sh` no simula el destino remoto: levanta el **mismo `rrsync`** que vive en
el `authorized_keys` del VPS detrás de un `ssh` de mentira, así que las opciones de `rsync`
que rrsync no admite (`--filter`, por ejemplo) se ven en el momento y no el día de la
entrega. Si `rrsync` no está instalado, esa pata avisa y se salta; el resto corre igual.

## Coolify

Coolify construye esta imagen **desde `main`**, con este `Dockerfile` como raíz de
construcción (*Build Pack*: Dockerfile; *Base Directory*: `packaging/repo/`). No hace falta
ningún paso de construcción adicional: no hay `package.json`, ni `pnpm`, ni assets que
compilar. Un cambio en `index.html` o en `Dockerfile` en `main` es lo único que dispara un
redespliegue.

**El montaje**: la aplicación de Coolify necesita `/srv/rfirma-repo` del anfitrión montado en
`/srv/rfirma-repo` del contenedor, **de sólo lectura**. Sin él, las rutas de los tres
repositorios devuelven 404 y la landing sigue sirviéndose igual.

## Aprovisionamiento humano (fuera de este repositorio)

Ni el CI ni ningún agente pueden hacer esto: hay que hacerlo a mano una vez.

1. **Usuario y directorio en el VPS**, con el directorio que ya sirve Caddy:

   ```bash
   sudo adduser --system --group --home /var/lib/rfirma-publish --shell /bin/sh rfirma-publish
   sudo mkdir -p /srv/rfirma-repo
   sudo chown rfirma-publish:rfirma-publish /srv/rfirma-repo
   sudo chmod 755 /srv/rfirma-repo
   ```

   **`--shell /bin/sh` no es un descuido.** `adduser --system` deja `/usr/sbin/nologin` por
   su cuenta, y sshd lanza la orden forzada *a través del shell del usuario*: con `nologin`,
   lo que viaja por la conexión es «This account is currently not available» en vez del
   protocolo de rsync, y `rsync` responde `protocol version mismatch -- is your shell clean?`.
   Quien cierra la puerta es `command=`+`restrict`, no la ausencia de shell. Si el usuario ya
   existe: `sudo chsh -s /bin/sh rfirma-publish`.

2. **La clave de despliegue, atada a una orden forzada.** La clave privada va al secreto
   `PUBLISH_SSH_KEY` del entorno `release` y la pública al `authorized_keys` del usuario, con
   `rrsync` delante y sin nada más:

   ```
   command="rrsync /srv/rfirma-repo",restrict ssh-ed25519 AAAA... ci@rfirma
   ```

   `restrict` quita pty, reenvío de puertos y agente. Con eso, la clave del CI no da consola:
   sólo sabe escribir en el directorio que ya sirve ficheros públicos. `rrsync` viene en el
   paquete `rsync` (Debian/Ubuntu: `/usr/bin/rrsync`).

3. **Las variables y el secreto del entorno `release`** en GitHub:

   | Nombre | Tipo | Qué |
   |---|---|---|
   | `PUBLISH_SSH_KEY` | secreto | la clave privada de despliegue, sin passphrase |
   | `PUBLISH_SSH_USER` | variable | `rfirma-publish` |
   | `PUBLISH_SSH_HOST` | variable | el nombre del VPS |
   | `PUBLISH_SSH_KNOWN_HOSTS` | variable | la línea de `ssh-keyscan <host>`, para que `StrictHostKeyChecking=yes` tenga con qué comparar |

4. **El montaje de la aplicación de Coolify**, el del apartado anterior.

El resto de la infraestructura —dominio y certificado TLS— también es aprovisionamiento
humano.
