# Seguridad

rFirma firma documentos con el certificado de una persona. Un fallo aquí no es una
molestia: es una firma que no vale, o una firma que vale y no debería. Este fichero dice
cómo avisar de uno, qué claves sostienen la distribución y qué hace rFirma con la red.

## Cómo informar de una vulnerabilidad

**Usa el aviso privado de GitHub**, no un issue público:

> <https://github.com/sgomez/rfirma/security/advisories/new>

**No hay dirección de correo, y es a propósito**: un correo personal en un fichero público
es un dato personal publicado para siempre, y además no da acuse de recibo. El aviso privado
sí: queda registrado, se puede conversar dentro y termina en un aviso publicado con crédito
a quien lo encontró.

Cuenta qué versión usabas, en qué canal la instalaste (flatpak, `.deb`, `.rpm`) y cómo
reproducirlo. Si lo que has encontrado toca a la firma, di también qué formato
(PAdES, CAdES, XAdES o FacturaE) y si la rúbrica era visible.

**Qué esperar**: acuse de recibo en una semana. rFirma lo mantiene una sola persona y está
en alfa, así que no hay más promesa que ésa; lo que sí hay es que ningún aviso se queda sin
respuesta.

## Versiones con soporte

| Versión | Soporte |
|---|---|
| La última publicada | Sí |
| Cualquier anterior | No |

rFirma está en alfa y no hay ramas de mantenimiento: un arreglo de seguridad sale en la
versión siguiente. Por eso el canal recomendado son los repositorios de
`rfirma.sgomez.me` y no el paquete suelto — **un paquete instalado a mano no se actualiza
nunca**, y sin actualización no hay forma de hacerte llegar el arreglo
([ADR-0015](docs/adr/0015-canal-de-distribucion-propio.md)).

## Las claves de larga vida

**Hay una sola clave GPG**, y firma todo lo que una persona puede verificar: el
`SHA256SUMS` de cada Release, cada `.rpm`, el `InRelease` del repositorio apt, el
`repomd.xml` del de dnf y los *commits* del repositorio ostree. Dos raíces de confianza
para el mismo enunciado —«esto lo hizo rFirma»— serían peor seguridad, no mejor.

| Clave | Qué firma | Dónde vive |
|---|---|---|
| Maestra (sólo certificación, sin caducidad) | Nada: sólo certifica sus subclaves | **Fuera de línea**, con su certificado de revocación, en ningún sistema conectado |
| Subclave de firma (caduca a los dos años) | Releases, ostree, apt, dnf y cada `.rpm` | Secreto del entorno `release` del CI, y en ninguna otra parte |

**La pública** se sirve en <https://rfirma.sgomez.me/rfirma.asc> — es el `Signed-By` de apt
y el `gpgkey` de dnf— y su huella es:

```
C8D6 A81C 1ED4 3A28 D426  8112 A6E0 EE02 2344 6A16
```

La misma huella se publica en la portada de <https://rfirma.sgomez.me>. Contrástala con lo
que descargues (`gpg --show-keys rfirma.asc`) y añádela a lo que ya sepas de rFirma por
otro camino: es lo que hace que la firma signifique algo.

**Si la subclave se filtra**, se revoca, se emite otra bajo la misma maestra y **la huella
que tienes en tu `Signed-By` sigue valiendo**: no tienes que volver a dar de alta el
repositorio. Ésa es toda la razón de que la maestra no baje nunca al CI.

La clave la genera —y los secretos los da de alta— `packaging/setup-signing-key.sh`, que
ejecuta una persona en su equipo y ningún CI.

**No hay clave de autoactualización.** rFirma no se actualiza sola: no hay *updater*, ni
clave minisign, ni `latest.json`. Lo único que hace es enseñarte que existe una versión
nueva.

## Cómo verificar lo que descargas

Con los dos ficheros de la Release, `SHA256SUMS` y `SHA256SUMS.asc`:

```sh
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check SHA256SUMS
```

Un `.rpm` suelto lleva además la firma dentro, así que se verifica solo:

```sh
sudo rpm --import https://rfirma.sgomez.me/rfirma.asc
rpm --checksig rfirma-*.rpm
```

Los `.deb` no se firman de uno en uno —apt firma el índice del repositorio— así que el
`.deb` suelto se verifica por el `SHA256SUMS.asc` de arriba.

Cada paquete lleva además una **atestación de procedencia** de GitHub, que dice de qué
*commit* y de qué ejecución salió:

```sh
gh attestation verify rfirma_*.deb --repo sgomez/rfirma
```

## La comprobación de versión es la primera conexión saliente

Y hoy es también la única. Al arrancar, rFirma pregunta a GitHub por la última publicación:

- `GET https://api.github.com/repos/sgomez/rfirma/releases/latest`
- Como mucho una vez cada 24 horas, con 10 segundos de espera y sin reintentos.
- **Sin credenciales y sin identificador de ninguna clase.** Lo único que rFirma dice de sí
  misma es el `User-Agent`, `rfirma/<versión>`, porque la API de GitHub rechaza con 403 lo
  que no se presenta.
- Cualquier tropiezo —sin red, DNS que no resuelve, GitHub que contesta 500— es silencio:
  no hay error que enseñar ni reintento que hacer.
- Lo que llega es un número de versión para pintar una franja. **No se descarga ni se
  instala nada.**

Se apaga en **Preferencias → Privacidad**, con el ajuste para dejar de avisar, que está
siempre visible.

Ningún otro camino de rFirma abre una conexión: firmar, elegir certificado y hablar con el
token PKCS#11 ocurren enteros en tu equipo, y la clave privada no sale de él
([ADR-0001](docs/adr/0001-firma-trifasica-clave-privada-solo-en-rust.md)).
