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
| 0007 | Sin barra de menús: una sola barra con la identidad, los documentos y el menú |
| 0008 | Licencia EUPL-1.2 |
| 0009 | Catálogo de cadenas propio, cinco idiomas, errores que clasifican situaciones |
| 0010 | Qué recuerda rFirma entre sesiones y dónde |
| 0011 | Dónde cae el documento firmado |
| 0012 | La rúbrica la normaliza Rust, no Java |
| 0013 | Estructura del repositorio y cadena de compilación (el `justfile`, el *bundler*) |
| 0014 | Gradas de prueba y puerta de calidad (CRAP), y qué carriles del CI corren en cada evento |
| 0015 | Canal propio: repositorios y canal de Windows en `rfirma.sgomez.me`, y Releases |
| 0016 | El sello de sesión: una sola invariante |
| 0017 | La arquitectura de los dos lados: puertos en la ventana, contextos con capas en el backend |
| 0018 | rFirma no es un lector de PDF: la firma empieza por un verbo |
| 0019 | El recuadro que pide la sede cruza crudo al puente, sin la conversión del local |
| 0020 | La ventana de sede existe antes que la operación, y solo se enseña cuando hay algo que decir |
| 0021 | La versión negociada al abrir el canal rige la sesión; `ver` solo cuenta sin canal |
| 0022 | El almacén que nombra la sede: cuál se obedece, a qué módulo descubierto acota `PKCS11:<ruta>` y cuál sale con `SAF_08` |
| 0023 | Se sigue al original en los casos felices salvo contradicción o riesgo grave: SHA-1 solo si la persona lo permite, XAdES explícita no |
| 0024 | Un proceso por trámite de sede, y el escritorio aparte |
| 0025 | `selectcert` no abre sesión en el token: sin PIN en una operación que no firma |
| 0029 | La envoltura XAdES la declara la sede; el nombre del formato solo la suple |
| 0030 | Una firma que pide sello de tiempo y no se puede sellar no sale: `SAF_09`, nunca una firma sin sello |
| 0032 | La selección automática que pide la sede (`headless`, `mandatoryCertSelection=false`) se respeta solo si la persona lo permite en sus preferencias |
| 0033 | La interfaz se prototipa en Claude Design, y lo que manda vive en el repositorio: fichas, bundle y copia de los artboards (sustituido por el 0045) |
| 0034 | El Almacén de rFirma: una base NSS cifrada con un PIN que solo guarda el llavero del escritorio |
| 0035 | Windows como segunda plataforma: dependencias por target y adaptadores no disponibles |
| 0036 | El saludo TLS del canal local fuera de Linux (Windows y macOS) es de rustls |
| 0037 | Los servicios de la sede se esperan sin límite una vez conectados: 30 s para conectar, ninguno para contestar |
| 0038 | Las cookies de la sede duran una operación, compartidas por los cuatro clientes HTTP |
| 0039 | Un servicio de la sede con un certificado TLS no reconocido falla: sin el diálogo de confianza del original |
| 0040 | macOS en Apple Silicon como tercera plataforma: `.dylib` en `Contents/Frameworks`, `.dmg` sin notarizar y adaptadores pendientes |
| 0041 | La línea de órdenes sigue a la de AutoFirma, sin la contraseña en argv |
| 0042 | La lista de `-certtui` habla por `/dev/tty` y la pinta ratatui |
| 0043 | La validez de una firma: tres valores, gana el peor problema y el sello de tiempo prueba la fecha |
| 0045 | La verdad de una pantalla implementada es su código y sus historias de Storybook; Claude Design solo explora (sustituye al 0033) |

Los ADR que solo afectan a la suite de conformidad viven en `rfirma-conformance/docs/adr/` y
comparten la numeración: el siguiente ADR, esté donde esté, toma el número libre más alto.

| ADR | Sobre qué manda |
|---|---|
| 0026 | La exigencia es lo que pretende el código de AutoFirma; el manual solo rebaja |
| 0027 | Dos sedes: la publicada para lo de punta a punta, la escrita a mano para la gramática |
| 0028 | Un clic por trámite: la suite quita el PIN, pero no esconde la elección del certificado |
| 0031 | La sede verifica las firmas sin dependencias externas, con su propia C14N inclusiva |

## `research/` — mediciones (por qué algo es como es)

Se consultan **solo si vas a cambiar la decisión que sostienen**. Son los
ficheros más grandes del repositorio (hasta 32 KB).

`ancla-y-paginas-en-el-puente` · `arrastre-bajo-el-sandbox` · `barra-de-titulo-en-linux` ·
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
`timeout-lote-remoto` · `token-flags-login` · `token-pkcs11-pruebas` ·
`validacion-eidas-en-319-102-1`

## Sueltos en `docs/`

`afirma/1.9.2/` — Manual de referencia del protocolo `afirma://` de AutoFirma 1.9.2 (16 capítulos, el anexo A1 con el catálogo de bugs del original, el A2 con la revisión de las funciones compartidas que sí compara con rFirma y el B1 con la revisión de `UrlHttpManagerImpl`).

`mapa-protocolo.md` — El mapa del protocolo de AutoFirma cruzado con el trámite de sede de rFirma, con el esqueleto de auditoría.

`pruebas-manuales-protocolo.md` (2 KB) — la **segunda puerta manual** del
ADR-0014: lo del protocolo `afirma://` que necesita un navegador o una sede de
verdad y por eso no lo tiene el CI. Se ejecuta una vez por etiqueta `v*`. Se
abre para **ejecutarla** o para mover una fila, no para entender el protocolo:
eso está en `research/contrato-protocolo-afirma.md`.

`casos-de-sedes-reales.md` — el procedimiento para capturar, redactar y
guardar la URL `afirma://` de un trámite que falló en una sede real: dónde
capturarla, qué se le quita antes de que entre en el repositorio y si su
destino es solo la grada A o también un guion de sede y una comprobación de
conformidad.

## `design/` — una ficha por pantalla (lo que ve el usuario)

`ventana-principal` · `cabecera` · `pestanas-de-documentos` ·
`visor-de-documento` · `panel-de-firma` · `preferencias` · `dialogo-pin` ·
`dialogo-progreso-firma` · `dialogo-paginas-sin-firma-visible` ·
`dialogo-firmar-de-todos-modos` ·
`dialogo-ver-firmas` (las firmas que ya trae el documento, con su validez, antes de firmar) ·
`acerca-de` ·
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

## Cómo se escribe un mapa

**Una fila de un mapa dice qué es el fichero, y se para ahí.** Una frase, la que
hace falta para saber si es el que se busca; y, cuando ayude a no confundirlo,
qué **no** es. El *cómo* funciona lo dice el código, y el porqué un ADR, citado
por número. Sin tamaños ni números de línea —los da `just outline` en el momento—, sin
citas a identificadores de spec y sin números de PR o de issue.

En el backend, en la interfaz y en el puente Java no hay filas: la misma regla vale para la primera
línea `//!` de cada módulo —`.rs`, `.ts`, `.tsx` o `.java`—, entera en una línea y de 300
caracteres como mucho, que `just outline <directorio>/` junta en el índice. El
mapa de cada zona y de cada contexto se queda con lo que el código no confiesa:
las carpetas, los módulos que se leen antes que sus hermanos, las trampas y las
secciones «Al tocar…». Lo vigila
`rfirma-app/src-tauri/tests/modules_open_with_a_header.rs`, que exige que todo
módulo de las tres zonas que no sea de prueba abra con esa línea.
