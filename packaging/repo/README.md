# rfirma.sgomez.me: la landing y la publicación

La imagen que sirve `https://rfirma.sgomez.me` es **Caddy (`caddy:alpine`) con esta landing, y nada
más** ([ADR-0015](../../docs/adr/0015-canal-de-distribucion-propio.md)). Los tres
repositorios de paquetes (ostree, apt, dnf) no van dentro de la imagen: los publica un
montaje de directorio del anfitrión aparte, por `rsync`, y Caddy los sirve a través de un
enlace simbólico.

| Fichero | Qué es |
|---|---|
| `site/` | La landing: un proyecto Astro estático en cinco idiomas, con su propio `package.json` |
| `site/src/i18n/` | Un diccionario por idioma con las claves en inglés, y las pruebas que exigen los cinco al 100 % |
| `site/src/components/` | Un componente por sección de la página |
| `site/src/scripts/` | Las cuatro funciones de la página: pestañas, copiar, demo guiada y aparición al bajar |
| `Caddyfile` | Configuración de Caddy (no-root, puerto 3000, cabeceras, healthcheck y las rutas de los tres repositorios) |
| `Dockerfile` | `caddy:alpine` no-root más la landing y Caddyfile |
| `download-series.sh` | Baja y verifica **toda** la serie menor vigente desde las Releases |
| `build-tree.sh` | Reconstruye los tres repositorios enteros, desde cero, en un directorio nuevo |
| `publish-tree.sh` | Sube el árbol al anfitrión e intercambia el enlace `actual` |
| `*.test.sh` | Las pruebas de los dos anteriores; `just check-repo` las corre |

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

Volver a publicar a mano se lanza **sobre la propia etiqueta**, porque el entorno `release`
solo admite etiquetas `v*` y rechaza una ejecución desde `main`:

```
gh workflow run publish.yml --ref v0.4.1 -f tag=v0.4.1
```

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
  que venga sin firma. El orden de esos pasos lo vigila `.github/check-workflows.sh`,
  dentro de `just check-repo`.

**Las firmas del árbol no las prueba nadie automáticamente**, y no puede ser de otra manera:
firmar necesita una clave privada, las de rFirma las crea una persona siguiendo
[La clave de firma](#la-clave-de-firma) y ninguna prueba puede fabricarse una que valga. Por eso
`build-tree.sh` tiene un modo `SIN-FIRMA-SOLO-PRUEBAS` que es el que usa su test, y por eso
`.github/check-workflows.sh` prohíbe que esa cadena aparezca en un workflow. El camino con
clave se ensaya con una etiqueta `v*-rc.N`.

## Las pruebas: `just check-repo`

`publish-tree.sh` es la única parte de la tubería que **no** puede ensayarse con una etiqueta
`v*-rc.N` —el ensayo se detiene justo antes de tocar el anfitrión—, así que se prueba aquí.
`publish-tree.test.sh` no simula el destino remoto: levanta el **mismo `rrsync`** que vive en
el `authorized_keys` del VPS detrás de un `ssh` de mentira, así que las opciones de `rsync`
que rrsync no admite (`--filter`, por ejemplo) se ven en el momento y no el día de la
entrega. Si `rrsync` no está instalado, esa pata avisa y se salta; el resto corre igual.

## Coolify

Coolify construye esta imagen **desde `main`**, y desde que la landing es un proyecto Astro
el contexto de construcción es **la raíz del repositorio**: la página toma el sistema de
diseño de `rfirma-app/src/design-system/bundle/`, que está fuera de este directorio. En la
aplicación de Coolify eso son dos campos (*Build Pack*: Dockerfile; *Base Directory*: `/`;
*Dockerfile Location*: `packaging/repo/Dockerfile`), y **hay que cambiarlos a mano una vez**:
con el `Base Directory` viejo la construcción falla al no encontrar el bundle.

La imagen se construye en dos etapas: `node:24-alpine` instala las dependencias del sitio y
ejecuta `astro build`, y `caddy:alpine` se queda solo con el `dist/`. Un cambio en `site/`,
en el `Dockerfile` o en el sistema de diseño en `main` dispara un redespliegue.

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

   ```bash
   ssh-keygen -t ed25519 -N '' -C ci@rfirma -f publish
   gh secret set PUBLISH_SSH_KEY --env release < publish
   echo "command=\"rrsync /srv/rfirma-repo\",restrict $(cat publish.pub)"
   ```

   La última línea es la que va al `authorized_keys` del usuario (en
   `/var/lib/rfirma-publish/.ssh/`, `700` y `600`, de `rfirma-publish`). Antes de añadirla,
   borra la que acabe en `ci@rfirma`: si no, la clave vieja sigue pudiendo escribir. Después,
   `shred -u publish`.

   `restrict` quita pty, reenvío de puertos y agente. Con eso, la clave del CI no da consola:
   sólo sabe escribir en el directorio que ya sirve ficheros públicos. `rrsync` viene en el
   paquete `rsync` (Debian/Ubuntu: `/usr/bin/rrsync`).

3. **Las variables del repositorio y el secreto del entorno `release`** en GitHub:

   | Nombre | Tipo | Qué |
   |---|---|---|
   | `PUBLISH_SSH_KEY` | secreto | la clave privada de despliegue, sin passphrase |
   | `PUBLISH_SSH_USER` | variable | `rfirma-publish` |
   | `PUBLISH_SSH_HOST` | variable | el nombre del VPS |
   | `PUBLISH_SSH_KNOWN_HOSTS` | variable | la línea de `ssh-keyscan <host>`, para que `StrictHostKeyChecking=yes` tenga con qué comparar |

   ```bash
   gh variable set PUBLISH_SSH_USER --body rfirma-publish
   gh variable set PUBLISH_SSH_HOST --body <host>
   gh variable set PUBLISH_SSH_KNOWN_HOSTS --body "$(ssh-keyscan <host> 2>/dev/null)"
   ```

   Las tres son variables del repositorio. El secreto, en cambio, es del entorno.

4. **El montaje de la aplicación de Coolify**, el del apartado anterior.

5. **El contexto de construcción de la aplicación de Coolify**: `Base Directory` a `/` y
   `Dockerfile Location` a `packaging/repo/Dockerfile`, como explica el apartado anterior.

El resto de la infraestructura —dominio y certificado TLS— también es aprovisionamiento
humano.

## La clave de firma

Una maestra fuera de línea que solo certifica, y una subclave de firma que caduca a los dos
años y es lo único que baja al CI (ADR-0015). La subclave actual caduca el **2028-09-04**.
Todo se hace en un equipo propio, nunca en el CI, con un anillo de usar y tirar:

```bash
export GNUPGHOME="$(mktemp -d)"; chmod 700 "$GNUPGHOME"; umask 077
read -rs FRASE && printf '%s' "$FRASE" > "$GNUPGHOME/frase" && unset FRASE
GPG=(gpg --batch --pinentry-mode loopback --passphrase-file "$GNUPGHOME/frase")
```

La frase protege la maestra y la subclave, y es también el secreto
`GPG_SIGNING_PASSPHRASE`: guárdala en el gestor antes de escribirla.

### Crear

```bash
UID_CLAVE='rFirma signing <correo del proyecto, nunca uno personal>'
"${GPG[@]}" --quick-generate-key "$UID_CLAVE" rsa4096 cert never
HUELLA="$(gpg --with-colons --list-keys "$UID_CLAVE" | awk -F: '$1=="fpr"{print $10; exit}')"
"${GPG[@]}" --quick-add-key "$HUELLA" rsa4096 sign 2y
cp "$GNUPGHOME/openpgp-revocs.d/$HUELLA.rev" rfirma-revocacion.asc
```

Después, [Subir la subclave](#subir-la-subclave), y a mano:

- `gh variable set GPG_FINGERPRINT --body "$HUELLA"`, que es variable del repositorio, no del
  entorno.
- La huella en `SECURITY.md` y en `site/src/components/Transparency.astro`: `release.yml`
  comprueba que coincide con la importada.
- El entorno `release`, restringido a las etiquetas `v*`:

  ```bash
  gh api --method PUT "repos/{owner}/{repo}/environments/release" --input - <<'JSON'
  {"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}
  JSON
  gh api --method POST "repos/{owner}/{repo}/environments/release/deployment-branch-policies" \
    -f name='v*' -f type=tag
  ```

- Un *tag ruleset* `v*` con *Restrict creations*, *updates* y *deletions*, y **Repository
  admin en la *Bypass list***: sin él, ni el administrador puede crear la etiqueta.

La pública no se sube a mano: `publish.yml` la exporta de la huella y `build-tree.sh` la
sirve en `/rfirma.asc`.

### Renovar la subclave

Antes de que caduque, con la copia fuera de línea como `GNUPGHOME`:

```bash
SUB="$(gpg --with-colons --list-keys "$HUELLA" | awk -F: '$1=="sub"{s=1; next} s && $1=="fpr"{print $10; exit}')"
"${GPG[@]}" --quick-set-expire "$HUELLA" 2y "$SUB"
```

Después, [Subir la subclave](#subir-la-subclave). La huella no cambia. Quien ya tiene
configurado apt, dnf o flatpak conserva su copia local de la pública con la fecha vieja: no se
ha comprobado qué hace cada gestor con ella, y hay que resolverlo antes de la fecha de arriba.

### Revocar

- **Si se filtra la subclave:** `gpg --edit-key "$HUELLA"`, luego `key 1`, `revkey` y `save`
  (no hay orden en lote para revocar una subclave). A continuación, una subclave nueva con el
  `--quick-add-key` de [Crear](#crear) y [Subir la subclave](#subir-la-subclave).
- **Si se filtra la maestra:** `gpg --import rfirma-revocacion.asc`, quitando antes el `:`
  que gpg pone delante de `-----BEGIN` para que no se importe por accidente. La identidad
  muere, y hay que volver a [Crear](#crear) con una huella nueva en todas partes.

### Subir la subclave

```bash
"${GPG[@]}" --yes --armor --output subclave-ci.asc --export-secret-subkeys "$HUELLA"
gpg --list-packets subclave-ci.asc | grep -q gnu-dummy   # la maestra no viaja
gh secret set GPG_SIGNING_SUBKEY --env release < subclave-ci.asc
gh secret set GPG_SIGNING_PASSPHRASE --env release < "$GNUPGHOME/frase"
```

Si el `grep` no encuentra nada, el fichero lleva la maestra: no se sube. Al terminar, copia
`$GNUPGHOME` y `rfirma-revocacion.asc` fuera de línea, y
`shred -u "$GNUPGHOME/frase" subclave-ci.asc; rm -rf "$GNUPGHOME"`.
