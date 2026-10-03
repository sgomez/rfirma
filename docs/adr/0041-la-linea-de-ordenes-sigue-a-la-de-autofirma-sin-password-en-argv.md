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
   **`--json` es la traducción del `--xml` de la misma orden**: solo existe donde
   existe `--xml`, se rechaza junto a él y no cambia los códigos de salida. La
   regla de traducción es mecánica: un elemento con hijos es un objeto con una
   clave por hijo; uno sin hijos, una cadena con su texto, sin adivinar tipos
   (`<result>true</result>` es `"result": "true"`); uno que la respuesta de esa
   orden puede repetir (como `alias`) es siempre una lista, con uno o ninguno; y
   la raíz se mantiene como clave de primer nivel, para que sea reversible.
   `<afirma><result>ok</result><response><alias>A</alias><alias>B</alias></response></afirma>`
   es `{"afirma":{"result":"ok","response":{"alias":["A","B"]}}}`.
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
