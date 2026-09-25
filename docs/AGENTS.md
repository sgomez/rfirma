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
| 0004 | La librería nativa va en el paquete, y hay tres paquetes: flatpak, `.deb` y `.rpm` |
| 0005 | Servidor local HTTPS, y la CA la instala la aplicación en los almacenes NSS |
| 0006 | La firma visible se configura sobre el documento |
| 0007 | Sin barra de menús: cabecera única |
| 0008 | Licencia EUPL-1.2 |
| 0009 | Catálogo de cadenas propio, cinco idiomas, errores que clasifican situaciones |
| 0010 | Qué recuerda rFirma entre sesiones y dónde |
| 0011 | Dónde cae el documento firmado |
| 0012 | La rúbrica la normaliza Rust, no Java |
| 0013 | Estructura del repositorio y cadena de compilación (el `justfile`, el *bundler*) |
| 0014 | Gradas de prueba y puerta de calidad (CRAP) |
| 0015 | Canal propio: tres repositorios en `rfirma.sgomez.me` y Releases |
| 0016 | El sello de sesión: una sola invariante |
| 0017 | La arquitectura de los dos lados: puertos en la ventana, contextos con capas en el backend |
| 0018 | rFirma no es un lector de PDF: la firma empieza por un verbo |
| 0019 | El recuadro que pide la sede cruza crudo al puente, sin la conversión del local |
| 0020 | La ventana de sede existe antes que la operación, y solo se enseña cuando hay algo que decir |
| 0021 | La versión negociada al abrir el canal rige la sesión; `ver` solo cuenta sin canal |
| 0022 | El almacén que nombra la sede: cuál se obedece, a qué módulo descubierto acota `PKCS11:<ruta>` y cuál sale con `SAF_08` |
| 0023 | Se sigue al original en los casos felices salvo contradicción o riesgo grave: SHA-1 sí, XAdES explícita no |
| 0024 | Un proceso por trámite de sede, y el escritorio aparte |
| 0025 | `selectcert` no abre sesión en el token: sin PIN en una operación que no firma |
| 0029 | La envoltura XAdES la declara la sede; el nombre del formato solo la suple |
| 0030 | Una firma que pide sello de tiempo y no se puede sellar no sale: `SAF_09`, nunca una firma sin sello |

Los ADR que solo afectan a la suite de conformidad viven en `rfirma-conformance/docs/adr/` y
comparten la numeración: el siguiente ADR, esté donde esté, toma el número libre más alto.

| ADR | Sobre qué manda |
|---|---|
| 0026 | La exigencia es lo que pretende el código de AutoFirma; el manual solo rebaja |
| 0027 | Dos sedes: la publicada para lo de punta a punta, la escrita a mano para la gramática |
| 0028 | Un clic solo donde hay algo que decidir: la suite quita el PIN y rFirma de conformidad consiente lo que no deja elección |
| 0031 | La sede verifica las firmas sin dependencias externas, con su propia C14N inclusiva |

## `research/` — mediciones (por qué algo es como es)

Se consultan **solo si vas a cambiar la decisión que sostienen**. Son los
ficheros más grandes del repositorio (hasta 32 KB).

`ancla-y-paginas-en-el-puente` · `arrastre-bajo-el-sandbox` ·
`ca-en-los-almacenes-de-confianza` · `ca-nss-navegador-abierto` · `campos-de-firma-vacios` ·
`contrato-protocolo-afirma` · `coordenadas-recuadro-pades` ·
`exclusion-afirma-ui-utils` · `filtros-sede-unmeasured` · `firma-visible-trifasica` ·
`flathub-libreria-nativa` · `flatpak-canal-unico` · `glibc-libreria-nativa` ·
`graalvm-libawt-shared` · `i18next-y-el-po` · `native-image-postfirma` ·
`native-image-postfirma-ce25` · `native-image-shared-pades` ·
`native-image-xades` ·
`opensc-del-sistema` · `p12-en-almacen-nss` · `pades-triphase-contract` ·
`pinentry-gtk-temas-empaquetado` · `pkcs11-mecanismo-firma` ·
`prefirma-en-seco-pdfjs` · `recuadro-replicado-pdfsig` · `rutas-de-firefox-y-nss` ·
`rutas-reales-con-filesystem-home` ·
`timeout-lote-remoto` · `token-flags-login` · `token-pkcs11-pruebas`

## Sueltos en `docs/`

`afirma/1.9.2/` — Manual de referencia del protocolo `afirma://` de AutoFirma 1.9.2 (16 capítulos y el anexo A1 con el catálogo de bugs del original).

`mapa-protocolo.md` — El mapa del protocolo de AutoFirma generado a tag fijado del original y cruzado con el trámite de sede de rFirma, con el esqueleto de auditoría.

`pruebas-manuales-protocolo.md` (2 KB) — la **segunda puerta manual** del
ADR-0014: lo del protocolo `afirma://` que necesita un navegador o una sede de
verdad y por eso no lo tiene el CI. Se ejecuta una vez por etiqueta `v*`. Se
abre para **ejecutarla** o para mover una fila, no para entender el protocolo:
eso está en `research/contrato-protocolo-afirma.md`.

## `design/` — una ficha por pantalla (lo que ve el usuario)

`ventana-principal` · `cabecera` · `bandeja-de-documentos` ·
`visor-de-documento` · `panel-de-firma` · `preferencias` · `dialogo-pin` ·
`dialogo-progreso-firma` · `dialogo-paginas-sin-sello` · `acerca-de` ·
`ventana-de-sede` (la ventana que abre una sede por `afirma://`, entera: espera,
consentimiento, firma, desenlace y sin certificado utilizable) ·
`primer-arranque` (el asistente que configura el equipo la primera vez) ·
`panel-de-estado` (la tabla de señales de la instalación, con sus reparaciones) ·
`retirar-certificado` (el velo que confirma y ejecuta la retirada del
certificado de rFirma de los almacenes) ·
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
issues hijos: enlace nativo de sub-issue, `## Spec extract` y `## Complexity`).

Los lee el orquestador y los trabajadores de `/developer`. **No los leas si no
vas a publicar un issue o una PR.**
