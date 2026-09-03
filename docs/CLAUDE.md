# Índice de la documentación

**Ningún documento de `docs/` se lee entero por costumbre.** Casi todos pesan
entre 5 y 30 KB; abrir uno «para ver si dice algo» cuesta más que el trabajo.
Busca en este índice, y si aun así necesitas el fichero, entra con
`grep -n '<término>' <fichero>` antes que con `cat`.

## `adr/` — decisiones (lo que ya está decidido y no se rediscute)

| ADR | Sobre qué manda |
|---|---|
| 0001 | La clave privada nunca cruza a Java: firma trifásica |
| 0002 | Las dependencias Java se consumen desde `~/.m2` |
| 0003 | Memoria manual en la frontera FFI |
| 0004 | La librería nativa va en el paquete, y el paquete es un flatpak |
| 0005 | Servidor local HTTPS con CA propia |
| 0006 | La firma visible se configura sobre el documento |
| 0007 | Sin barra de menús: cabecera única |
| 0008 | Licencia EUPL-1.2 |
| 0009 | Catálogo de cadenas propio, cinco idiomas (enmendado), errores que clasifican situaciones |
| 0010 | Qué recuerda rFirma entre sesiones y dónde |
| 0011 | Dónde cae el documento firmado |
| 0012 | La rúbrica la normaliza Rust, no Java |
| 0013 | Estructura del repositorio y cadena de compilación (el `justfile`) |
| 0014 | Gradas de prueba y puerta de calidad (CRAP) |
| 0015 | Canal de distribución propio |
| 0016 | El sello de sesión: una sola invariante |
| 0017 | La arquitectura de los dos lados: puertos y capas |

## `research/` — mediciones (por qué algo es como es)

Se consultan **solo si vas a cambiar la decisión que sostienen**. Son los
ficheros más grandes del repositorio (hasta 32 KB).

`ancla-y-paginas-en-el-puente` · `arrastre-bajo-el-arenero` ·
`campos-de-firma-vacios` · `contrato-protocolo-afirma` ·
`coordenadas-recuadro-pades` ·
`exclusion-afirma-ui-utils` · `firma-visible-trifasica` ·
`flathub-libreria-nativa` · `flatpak-canal-unico` · `glibc-libreria-nativa` ·
`graalvm-libawt-shared` · `i18next-y-el-po` · `native-image-postfirma` ·
`native-image-postfirma-ce25` · `native-image-shared-pades` ·
`pades-triphase-contract` · `pkcs11-mecanismo-firma` ·
`prefirma-en-seco-pdfjs` · `recuadro-replicado-pdfsig` · `token-flags-login` ·
`token-pkcs11-pruebas`

## `design/` — una ficha por pantalla (lo que ve el usuario)

`ventana-principal` · `cabecera` · `bandeja-de-documentos` ·
`visor-de-documento` · `panel-de-firma` · `preferencias` · `dialogo-pin` ·
`dialogo-progreso-firma` · `dialogo-paginas-sin-sello` · `acerca-de` ·
`design-system`

Al implementar una pantalla, **la ficha de esa pantalla es la fuente**, no el
sistema de diseño entero (14 KB).

## `agents/` — contratos de proceso

`issue-tracker.md` (mecánica de issues) · `code-host.md` (mecánica de PR) ·
`triage-labels.md` · `domain.md` · `prototyping.md` ·
`developer-defaults.md` · `delivery-ledger.md`

Dos de ellos son **anexos de una sola fase**, y no se abren fuera de ella:
`code-host-ci.md` (esperar, leer o clasificar el CI de una PR — incluye qué
verifica de verdad el verde y los dos carriles) y `issue-authoring.md` (crear
issues hijos: enlace nativo de sub-issue y `## Spec extract`).

Los lee el orquestador y los trabajadores de `/developer`. **No los leas si no
vas a publicar un issue o una PR.**
