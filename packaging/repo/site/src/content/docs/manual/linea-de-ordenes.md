---
title: Línea de órdenes
description: Referencia de la línea de órdenes de rFirma (sign, cosign, listaliases y verify), con sus parámetros, sus rechazos y ejemplos para Linux, el flatpak y Windows.
---

`rfirma` atiende en la terminal las órdenes de la línea de órdenes de AutoFirma:
`sign`, `cosign`, `listaliases` y `verify`. Sirve para firmar desde un script, desde
otro programa o sin abrir la ventana.

## Cómo se escribe una orden

```bash
rfirma <orden> [parámetros…]
rfirma <orden> --help
rfirma --help
rfirma --version
```

- **La orden va siempre la primera**, y se reconoce sin distinguir mayúsculas
  (`sign`, `SIGN` y `Sign` son la misma).
- La orden se atiende en la terminal y el proceso termina con ella: **no se une a la
  ventana de rFirma** que esté abierta.
- Los parámetros de más de una letra se escriben con dos guiones (`--alias`) o, como
  en AutoFirma, con uno (`-alias`). Los de una letra llevan uno: `-i`, `-o`, `-v`.
- `rfirma <orden> --help` da la sintaxis de esa orden; `rfirma --help`, la ayuda
  completa, y `rfirma --version`, la versión de rFirma y la de AutoFirma de la que
  salen sus validadores.

### Qué devuelve una orden

| Código de salida | Cuándo                                                                     |
| ---------------- | -------------------------------------------------------------------------- |
| `0`              | La orden termina bien.                                                     |
| `1`              | La orden se atiende y falla: no hay certificado, el fichero no se lee, etc. |
| `2`              | La línea de órdenes se rechaza tal como llega: falta un parámetro, sobra otro o pide algo que rFirma no hace. |

Por la **salida estándar** solo salen los datos que se consumen: la lista de `listaliases`, la
sintaxis de `--help` y el documento de `--xml` o `--json`. Los mensajes y los
registros van a la **salida de errores**, así que se puede redirigir la salida
estándar a un fichero o a otro programa sin que se mezclen.

## `sign`: firmar un fichero

```text
rfirma sign -i <fichero> (-o <fichero> | --xml | --json)
            (--alias <alias> | --filter <filtro> | --certgui | --certtui)
            [--filter <filtro>] [--store <almacén>]
            [--format auto|pades|cades|xades] [--algorithm sha512|sha384|sha256]
            [--config <propiedades>] [--password-fd <N>]
rfirma sign --gui -i <fichero>
```

Firma el fichero de `-i` y escribe la firma en `-o`, que **se sobrescribe si
existe**. Con `--gui`, no firma: abre el fichero en la ventana de rFirma, como si lo
soltaras en ella.

```bash
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado
```

## `cosign`: añadir una firma

```text
rfirma cosign -i <fichero> (-o <fichero> | --xml | --json)
              (--alias <alias> | --filter <filtro> | --certgui | --certtui)
              [--filter <filtro>] [--store <almacén>]
              [--format auto|pades|cades|xades] [--algorithm sha512|sha384|sha256]
              [--config <propiedades>] [--password-fd <N>]
```

Añade una firma al fichero ya firmado de `-i` y escribe el resultado en `-o`, que
se sobrescribe si existe. Admite los mismos parámetros que `sign`, salvo `--gui`.

```bash
rfirma cosign -i contrato-firmado.pdf -o contrato-cofirmado.pdf --alias mi-certificado
```

## `listaliases`: listar los certificados

```text
rfirma listaliases [--store <almacén>] [--xml | --json]
```

Escribe por la salida estándar el alias de cada certificado de los almacenes, uno
por línea, o solo los del almacén de `--store`. Ese alias se pasa luego a
`--alias`. Si no hay ningún certificado, la lista sale vacía y la salida de errores
lo dice.

```bash
rfirma listaliases
rfirma listaliases --store pkcs11:/usr/lib/opensc-pkcs11.so
```

Listar no pide PIN, así que `listaliases` rechaza `--password-fd`.

## `verify`: validar las firmas

```text
rfirma verify -i <fichero> [-v | -vv | --verbose] [--xml | --json]
rfirma verify --gui -i <fichero>
```

Valida las firmas del fichero de `-i` con la caducidad del certificado de quien
firmó, sin consultar la revocación y sin red. **Sale con `0` aunque la firma no sea
válida**, como AutoFirma: el resultado está en su salida, no en el código de
salida.

- Con `-v` (o `--verbose`) añade el formato y una ficha por firma: quién firma, en
  nombre de quién si es un certificado de representación, quién emitió el
  certificado y la fecha declarada.
- Con `-vv` (o `-v -v`, `--verbose --verbose`) añade el número de serie del
  certificado. Este texto no es estable: no lo analices con un programa; para eso
  está `--json`.
- Con `--json` saca en una línea el formato, la validez de cada firma y de sus
  contrafirmas, quién firma, el certificado y la fecha. `-v` no lo cambia.
- Con `--gui` abre el fichero en la ventana de rFirma para ver sus firmas.

`-v` y `--verbose` solo existen en `verify`; en otra orden se rechazan.

```bash
rfirma verify -i contrato-firmado.pdf -v
```

## Parámetros

| Parámetro                    | Qué hace                                                                                                   |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `-i <fichero>`               | El fichero de entrada.                                                                                     |
| `-o <fichero>`               | El fichero de salida, que se sobrescribe si existe. Obligatorio en `sign` y `cosign`, salvo con `--xml` o `--json`. |
| `--format <formato>`         | `auto` (por omisión), `pades`, `cades` o `xades`. Con `auto`, un PDF se firma en PAdES, un XML en XAdES y lo demás en CAdES. |
| `--algorithm <algoritmo>`    | `sha512` (por omisión), `sha384` o `sha256`.                                                               |
| `--store <almacén>`          | Busca los certificados solo en ese almacén; sin él, en todos. Los valores, en [Almacenes](#almacenes).      |
| `--alias <alias>`            | Firma con el certificado de ese alias, sin preguntar. No admite `--filter`.                                 |
| `--filter <filtro>`          | Sola, firma con el único certificado que cumple el filtro. Con `--certgui` o `--certtui`, acota la lista.   |
| `--certgui`                  | Abre la ventana de sede para elegir el certificado y escribir el PIN.                                      |
| `--certtui`                  | Elige el certificado en la terminal. Propio de rFirma; no está en Windows.                                 |
| `--password-fd <N>`          | Lee el PIN del descriptor `N`, que abre quien llama. Propio de rFirma; no está en Windows.                  |
| `--config <propiedades>`     | Propiedades `clave=valor` de la firma, una por línea: las mismas que acepta rFirma de una sede.            |
| `--xml`                      | Responde con un documento XML por la salida estándar.                                                      |
| `--json`                     | Responde lo mismo que `--xml`, en JSON. No va con `--xml`.                                                 |
| `--gui`                      | Entrega el fichero de `-i` a la ventana de rFirma, sin firmar ni verificar. Necesita `-i`.                  |
| `-v`, `--verbose`            | Solo en `verify`: el detalle de cada firma.                                                                |
| `--help`, `-h`               | Con una orden, su sintaxis; sola, la ayuda completa.                                                        |
| `--version`                  | La versión de rFirma.                                                                                      |

### Elegir el certificado

`sign` y `cosign` necesitan saber con qué certificado firmar, de una de estas
formas, que se excluyen entre sí:

- `--alias`: el certificado de ese alias, el que da `listaliases`.
- `--filter` sola: el único certificado que cumple el filtro. Si lo cumplen varios,
  la orden falla y pide afinarlo o acotar el almacén con `--store`.
- `--certgui`: la ventana de sede, con la lista de certificados y el PIN.
- `--certtui`: la lista en la terminal.

`--certgui` y `--certtui` no listan los certificados caducados ni permiten cambiar
de almacén. `--filter` acepta los criterios de filtro de las sedes, como
`nonexpired:`, `subject.contains:`, `issuer.contains:` o `thumbprint:`.

```bash
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --certgui
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --certtui --filter "nonexpired:"
```

### El PIN

**La contraseña no se escribe en la línea de órdenes.** Si el almacén no resuelve el
PIN por sí solo, rFirma lo pide en la terminal sin eco, lo lee del descriptor de
`--password-fd` o lo pide la ventana de `--certgui`.

### Almacenes

| Valor de `--store` | Almacén                                                                                 |
| ------------------ | --------------------------------------------------------------------------------------- |
| `auto`             | El del sistema: en Linux y macOS, como `mozilla`; en Windows, como `windows`.            |
| `mozilla`          | Fuera de Windows: los almacenes de los navegadores, el del sistema y el de rFirma.      |
| `windows`          | En Windows: el almacén de certificados de Windows, cuyo PIN pide el propio Windows.     |
| `pkcs11:<ruta>`    | El módulo PKCS#11 de esa ruta, si es uno de los que rFirma ya ha descubierto.          |

rFirma no carga un módulo PKCS#11 solo porque lo nombre la orden. Los almacenes de
AutoFirma que rFirma no abre (`pkcs12`, `dni`, `dnie`, `mac`) se rechazan, y un
nombre que AutoFirma no reconoce, también.

## Qué rechaza rFirma

Estas formas de AutoFirma se rechazan con código de salida `2` y un mensaje que lo
explica:

- **`countersign` y `batchsign`** no existen en la línea de órdenes de rFirma.
- **`--password`** se rechaza siempre: la contraseña en la línea de órdenes la ve
  cualquier usuario del equipo y queda en el historial. En su lugar, la terminal,
  `--password-fd` o `--certgui`.
- **`--preurl`, `--posturl`, `--hformat`, `--halgorithm`, `-r`** y **`--operation`** no
  existen.
- **`--algorithm sha1`** se rechaza por débil.
- **`--format facturae`, `ooxml` y `odf`** no están soportados.

## Ejemplos

### El PIN desde el llavero del escritorio

Con `secret-tool`, el llavero del escritorio (GNOME Keyring, KWallet, KeePassXC)
entrega el PIN por un descriptor, sin pasar por argv ni por el historial. Primero se
guarda una vez:

```bash
secret-tool store --label "PIN de rFirma" service rfirma
```

Y después cada firma lo lee del descriptor 3:

```bash
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado \
    --password-fd 3 3< <(secret-tool lookup service rfirma)
```

La sustitución de procesos `<(…)` es de `bash` y `zsh`, no de `sh`.

### Firmar un adjunto desde aerc

[aerc](https://aerc-mail.org) puede firmar el adjunto seleccionado con un atajo en
`binds.conf`, pasándolo a `rfirma` por `:pipe`. Aquí el PDF firmado queda en
`~/Documents`, y cada firma sobrescribe la anterior:

```ini
[view]
S = :pipe -b bash -c 'f=$(mktemp --suffix=.pdf) && trap "rm -f $f" EXIT && cat > "$f" && rfirma sign -i "$f" -o ~/Documents/firmado.pdf --alias mi-certificado --password-fd 3 3< <(secret-tool lookup service rfirma)'<Enter>
```

### En el flatpak

Dentro del sandbox, `rfirma` solo ve la carpeta de documentos. Para un fichero que
esté fuera, `--file-forwarding` y `@@` hacen que flatpak lo exponga por el portal:

```bash
flatpak run --file-forwarding me.sgomez.rfirma sign -i @@ ~/Descargas/contrato.pdf @@ -o ~/Documents/firmado.pdf
```

### En Windows

El instalador deja `rfirma.com`, el programa de consola, junto a `rfirma.exe`, y pone
la carpeta de rFirma en el PATH del usuario. En una consola abierta después de
instalar, `rfirma` encuentra el `.com`, que escribe en la consola y devuelve el
código de salida. En cmd y en PowerShell se escribe igual que en Linux:

```powershell
rfirma listaliases --store windows
rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado
```

**En Git Bash dentro de Windows Terminal hay que escribir `rfirma.com`**: bash no
prueba la extensión `.com` y encuentra `rfirma.exe`, el de la ventana, que ahí no
responde en la consola. En la ventana propia de Git Bash (mintty), `rfirma` funciona
tal cual.

En Windows no están `--certtui` ni `--password-fd`: el PIN del almacén de Windows lo
pide Windows y, en los demás almacenes, se escribe en el diálogo del escritorio o en
la ventana de `--certgui`.

### Elegir en la ventana o en la terminal

```bash
rfirma sign -i solicitud.pdf -o solicitud-firmada.pdf --certgui
rfirma sign -i solicitud.pdf -o solicitud-firmada.pdf --certtui --store mozilla
```
