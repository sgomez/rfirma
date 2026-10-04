# La línea de órdenes sigue a la de AutoFirma, sin la contraseña en argv

AutoFirma 1.9.x tiene, además del protocolo `afirma://`, una línea de órdenes
propia (`CommandLineLauncher`): `sign`, `cosign`, `listaliases`… con `-i`, `-o`,
`-format`, `-store`, `-alias`, `-filter`, `-certgui` y `-password`. Con `-alias`
firma sin enseñar nada ni pedir consentimiento, y el PIN solo lo recibe por
`-password` en argv. rFirma la imita por la regla del ADR-0023 —se sigue al
original en el caso feliz—, con una desviación: **no acepta la contraseña en
argv**.

## La regla

1. **Las órdenes y los parámetros son los del original**, en un subconjunto:
   `sign`, `cosign`, `listaliases` y `verify`; `-i`, `-o`, `--format`, `--store`,
   `--alias`, `--filter`, `--certgui`, `--algorithm`, `--config`, `--gui` y
   `--xml`. **Se documentan como `--opción` todas las de más de una letra**; `-i`,
   `-o` y `-h` son las cortas, y las de una letra no se agrupan.
   Al entrar, el parser normaliza `--opción` a la forma interna, y
   `-opción`, la del original, se sigue aceptando sin documentarla, de modo que
   un script de AutoFirma funciona. La ayuda y los mensajes de error nombran la
   forma `--`. `--version` (o `-version`) imprime `rfirma X.Y.Z` y, en una
   segunda línea, `AutoFirma 1.9.2`, la versión de la que salen los validadores,
   en stdout y con código 0. `--config` entra por el mismo camino que las
   propiedades de una sede, y `verify`
   comprueba la caducidad del certificado como el original —sin revocación ni
   red— y sale con 0 aunque la firma sea inválida, también como él.
   `countersign` queda fuera, como en el trámite de sede. `--certgui` abre la ventana de sede, que hace
   las veces del diálogo de selección del original; `--certtui`, propio de rFirma,
   elige en la terminal para quien no tiene pantalla. Firmar sin consentimiento con `--alias` o
   `--filter` es lo que hace el original y se mantiene: la ventana nunca fue una
   barrera frente a un proceso del mismo usuario, que puede sacar el PIN del
   llavero y firmar por su cuenta.
2. **`--password` se rechaza** con un mensaje que nombra la alternativa. En Linux
   `/proc/<pid>/cmdline` lo lee cualquier usuario del equipo, y además queda en
   el historial de la shell. Las vías del secreto son, en este orden: ninguna,
   si el almacén lo resuelve solo (el Almacén de rFirma, ADR-0034); la TTY, sin
   eco, cuando la hay; y `--password-fd`, un descriptor que abre quien llama. Con
   `--certgui`, el PIN se pide en la ventana, como en un trámite de sede.
3. **`--store` acota como en el ADR-0022**, con los nombres de la línea de
   órdenes. Sin `--store` se busca en todos los almacenes. Un nombre que el
   original no reconoce **se rechaza**, porque así lo hace su línea de órdenes,
   al contrario que su protocolo.
4. **stdout es solo para lo que se consume**: el XML de `--xml` o el JSON de
   `--json`. Los mensajes y los registros van a stderr, al contrario que el
   original, que los mezcla.
   **`--json` es un formato propio de rFirma**: es un fichero de datos para que
   un programa compruebe la salida de cualquier orden, `verify` incluida, aunque
   el original no tenga ahí `--xml`. Se rechaza junto a `--xml` y no cambia los
   códigos de salida. Sus reglas, comunes a todas las órdenes:
   - **Solo hay JSON si la orden sale bien.** Si falla, stdout queda vacío, el
     motivo va a stderr y el código de salida no es 0. En `verify`, una firma
     inválida o un fichero sin firmas salen bien; unas firmas que no se pueden
     leer, no.
   - **Sin sobre:** cada orden saca sus datos en la raíz, sin envoltura
     `afirma` ni versión del formato. Añadir una clave no rompe a nadie; quitar o
     renombrar una, sí.
   - **Claves en inglés y en camelCase**, y valores con su tipo (`true` es un
     booleano, no `"true"`). Una lista es siempre una lista, aunque tenga uno o
     ningún elemento; una clave sin valor se omite, no va a `null`.
   - **Vocabulario de las normas** donde ya nombran el concepto, aunque rFirma
     no aplique la norma entera: los atributos y campos de X.509 (RFC 5280), los
     DN de RFC 4514, el número de serie en hexadecimal, los algoritmos con su
     nombre de RFC y su OID, y los nombres oficiales de los formatos (`PAdES`).
     Los valores propios, en camelCase. Un certificado sale igual en todas las
     órdenes.
   - **Fechas en RFC 3339 y UTC**; texto en UTF-8 sin escapar; una sola línea
     compacta con salto final.
   - **El formato de cada orden lo fija su JSON Schema** (draft 2020-12),
     versionado con el código, y los tests validan contra él la salida real.
     Se construye con tipos propios de la línea de órdenes, no serializando los
     del dominio, y el `--xml` no comparte con él nada.
5. **Es un tercer rol del proceso**, el proceso de terminal: no se une al
   proceso de escritorio, y la única ventana que abre es la de sede, con
   `--certgui`.

## Considered Options

- **`-opción` con un guion como forma documentada**, la del original. Descartada:
  viene de Java, no de Unix (`--opción`) ni de Windows (`/opción`), y las
  herramientas modernas de los tres sistemas usan `--`. Se sigue aceptando para
  no romper los scripts de AutoFirma, pero no se documenta.
- **Una sintaxis propia** (`rfirma sign doc.pdf`). Descartada: quien ya tiene un
  script para AutoFirma no podría reutilizarlo, y la palabra «compatible» ya
  tiene en el glosario su lista cerrada de desviaciones.
- **Siempre interactiva**, sin firmar nunca sin consentimiento. Descartada por
  el ADR-0023: no protege a nadie, como se explica en la regla 1, y rompe la
  forma en que se usa el original.
- **Aceptar `--password` con un aviso en stderr**, como mysql o docker.
  Descartada: el aviso no quita el secreto de `/proc` ni del historial.
- **Variable de entorno y fichero de contraseña.** Descartadas: la variable la
  heredan los procesos hijos y acaba en volcados y registros; el fichero deja
  el secreto en disco. Lo que justificaba el fichero, la comodidad, lo cubre
  `secret-tool lookup` sobre el descriptor, que lee el llavero del escritorio
  sea cual sea (GNOME Keyring, KWallet, KeePassXC).
- **Un solo selector que elige la terminal o la ventana según haya TTY.**
  Descartado: la misma orden se comportaría distinto según desde dónde se
  lance. Con dos parámetros lo decide quien la escribe.
- **`AttachConsole` en Windows** en lugar de un binario de consola aparte.
  Descartada para cuando llegue Windows: cmd no espera al proceso y el código
  de salida no es fiable. El original instala `AutofirmaCommandLine.exe`.

## Enmienda: la línea de órdenes en Windows

Un ejecutable de subsistema gráfico no puede servir a la terminal en Windows:
cmd no lo espera y su código de salida no llega. Por eso hay **dos binarios en
la carpeta de instalación**:

- **`rfirma.exe`**, el de siempre, de subsistema gráfico: la ventana, el
  protocolo `afirma://` y el proceso de escritorio.
- **`rfirma.com`**, de subsistema de consola: atiende las órdenes de terminal,
  la ayuda y la versión, y cualquier otra cosa se la pasa a `rfirma.exe` de la
  misma carpeta sin esperarlo. Cargo no genera `.com`: es el binario
  `rfirma-console`, con la bandera `console`, que la receta de empaquetado
  renombra; va en el instalador como un recurso más, así que el desinstalador
  lo borra y la firma del actualizador lo cubre.

**`rfirma.com` gana a un nombre distinto** (`rfirma-cli.exe`, `rfirmac.exe`)
porque se escribe `rfirma` igual que en Linux: `PATHEXT` pone `.COM` antes que
`.EXE`, y cmd y PowerShell resuelven al de consola. Es el patrón de
`devenv.com` en Visual Studio. El CI de Windows lo comprueba en cmd,
PowerShell 5.1 y 7 y Git Bash (`just smoke-console`).

**El PATH del usuario**, porque la instalación es por usuario: el instalador
añade su carpeta al instalar y al actualizar si no está, sin duplicarla, y la
quita solo en una desinstalación de verdad. Lo lee y lo escribe sin pasar por
las cadenas de NSIS, que lo truncarían a 1024 caracteres, y avisa al sistema
para que las consolas nuevas lo vean. "App Paths" se descarta: cmd no lo
consulta.

**`--store` en Windows**: `windows` y `auto` acotan al almacén de certificados
de Windows, cuyo PIN pide Windows; `pkcs11:<módulo>` se comporta como en Linux,
y los almacenes que rFirma no abre en Windows (`mozilla`, `pkcs12:`, `dni`,
`mac`…) se rechazan sin abrir ninguno. `--password` se rechaza con un mensaje
propio de Windows, y `--password-fd` y `--certtui` aún no existen ahí.

**Git Bash**: en su ventana (mintty), bash encuentra `rfirma.exe`, que responde
por las tuberías. Dentro de Windows Terminal hay que escribir `rfirma.com`; no
se instala un script `rfirma` sin extensión.
