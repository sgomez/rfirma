# Prototipado

Dónde vive la salida de la skill `prototype` en este repo (y de quien la
invoque: `/wayfinder` con un ticket `wayfinder:prototype`, `/grill-with-docs`,
o el usuario directamente).

## La regla

La rama **UI** de `prototype` ("¿qué aspecto debería tener esto?") **no** se
construye como ruta throwaway con `?variant=` dentro de la app React. Se
prototipa en **Claude Design** y, una vez validada, se describe en un fichero
de `docs/design/`.

Claude Design es **solo** la superficie de prototipado: es desechable, sirve
para que el usuario mire y decida. La referencia duradera vive siempre en el
repo, en Markdown.

La rama **lógica** (`LOGIC.md`: máquinas de estado, flujo trifásico, errores de
PKCS#11) **no** cambia: sigue siendo el fichero HTML único y local que describe
la skill. El lienzo es para pantallas, no para simuladores.

## Granularidad: un canvas por caso de uso

- **Un canvas = un caso de uso** (un flujo completo: "firmar un PDF con DNIe",
  "validar una firma existente", "configurar almacenes de claves").
- **Un artboard = una pantalla en un estado concreto**: vacío, con datos,
  pidiendo PIN, en error, en progreso. Los estados son lo que de verdad valida
  una decisión de UI, así que van todos al canvas.
- **Una ficha `docs/design/<pantalla>.md` = una pantalla**, no un caso de uso.
  Una pantalla que aparece en varios flujos (el selector de certificados, por
  ejemplo) tiene **una sola ficha**, que lista los flujos que la usan. No
  dupliques su descripción por flujo.

Al validar un canvas se escriben o actualizan **varias** fichas, una por
pantalla del flujo. Es la única asimetría del esquema y es deliberada: el
canvas se organiza por recorrido, la documentación por pieza reutilizable.

## Dónde viven los canvas

Proyecto de Claude Design del repo:

- **Nombre**: `Autofirma de escritorio en Rust`
- **URL**: <https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132>
- **projectId**: `c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132`

**Corrección medida sobre el terreno:** un `.dc.html` **no** es un caso de uso,
es **un artboard**. El caso de uso es el proyecto entero, y su reparto en
pantallas-estado lo fija `canvas.json`, que además admite **páginas** con
nombre. Así que un caso de uso = un fichero `Main.dc.html` (el artboard de
entrada, obligatorio) más un `<Pantalla>.dc.html` por pantalla-estado, y las
páginas de `canvas.json` separan conjuntos (variantes frente a estados). Si
llega un segundo caso de uso y las páginas no bastan para mantenerlos
separados, entonces sí toca un proyecto por caso de uso.

El `Canvas.dc.html` inicial estaba vacío y se ha borrado: `Main.dc.html` es
ahora el artboard de entrada.

Si el editor de canvas no llega a mostrar más de un lienzo por proyecto, la
alternativa es un **proyecto por caso de uso** creado con la skill `design`;
en ese caso el `projectId` deja de ser único y manda el registro de canvas
de más abajo. Comprueba con `list_files` antes de asumir nada.

## Cómo publicar un canvas

1. **Redactar los artboards**: skill `design` (ficheros `.dc.html`, un artboard
   por pantalla-estado del flujo). La skill `design` sirve para **crear o
   re-sembrar** un lienzo; un lienzo ya publicado se edita en su Artifact.

   El `<helmet>` de un artboard nuevo **se copia de
   `docs/design/artboards/_helmet.part`, nunca de un `get_file` del proyecto**.
   La copia del proyecto se queda atrás y no hay nada allí que lo detecte: el
   02/09/2026 tres artboards entraron con dos tokens de sombra desfasados
   porque se redactaron mirando al proyecto. La dirección es siempre
   **repo → proyecto**.
2. **Transportar ficheros al proyecto**: herramienta `DesignSync` con el
   `projectId` de arriba. Orden obligatorio: `list_files` / `get_file` →
   `finalize_plan` (declarando writes y deletes) → `write_files`.
3. **Pasar `docs/design/artboards/comprueba.sh`** sobre la copia del repo, que
   verifica que todos llevan el `<helmet>` de `_helmet.part`. Compararlos entre
   sí no basta: trece ficheros de acuerdo entre ellos dan verde con el sistema
   de diseño equivocado entero.
4. **Anotarlo en el registro de canvas** de más abajo.
5. **Enseñar la URL al usuario** con la lista de artboards, y esperar su
   veredicto. La validación es siempre humana.

Si falta la autorización de design (la llamada de lectura falla por scopes),
dilo y cae a la rama local de `UI.md` en lugar de bloquear el prototipo.

## Variantes

Se mantiene la regla de la skill: **varias variantes radicalmente distintas**
(3 por defecto, tope 5) para la pantalla que sea el nudo del caso de uso —
distinta jerarquía de información y distinta acción principal, no distinto
color. Las variantes son artboards paralelos del mismo canvas, etiquetados
`A` / `B` / `C`. Las pantallas satélite del flujo se dibujan una sola vez,
siguiendo la variante que se esté evaluando.

## Sistema de diseño

- Lo **normativo** es el bundle versionado en
  `rfirma-app/src/design-system/bundle/` (#85): es el CSS que consume la
  aplicación y el único sitio donde vive un valor. `docs/design/design-system.md`
  lo describe —temas, roles de color, tokens `--rf-*`, componentes— y una prueba
  de grada A impide que se separen. Léela antes de dibujar nada y **no fijes
  colores a mano**.
- El bundle **no se edita en el repositorio**: se cambia en el proyecto de
  sistema de diseño, se reexporta entero sobre `bundle/` y se resella con
  `just seal-ds-bundle`. Un retoque a mano sale en rojo en `just lint`.
- El proyecto lleva ya adjunto el sistema de diseño compilado en
  `_ds/rfirma-design-system-ca5219d0-609a-4ce1-957f-e1d1d38e0c8c/` (tokens,
  `styles.css`, fuentes). Los artboards consumen esos tokens. Su `<helmet>` es
  una copia comprimida para previsualizar, no una fuente: si un valor difiere
  del bundle, gana el bundle.
- El proyecto es de tipo `PROJECT_TYPE_PROJECT`, **no**
  `PROJECT_TYPE_DESIGN_SYSTEM`, y el tipo es inmutable. Por tanto el flujo
  completo de `/design-sync` (subir una librería de componentes local como
  design system) **no aplica** aquí: `DesignSync` se usa solo como transporte de
  ficheros del lienzo.

## Al validar: la ficha de pantalla

Cuando el usuario da por buena una variante, el prototipo ha cumplido. Lo que
baja a `main` es Markdown, no HTML:

- Escribe o actualiza **`docs/design/<pantalla>.md`** por cada pantalla del
  flujo, con esta estructura:

  ```markdown
  # <Nombre de la pantalla>

  Una frase: qué resuelve y en qué punto del flujo aparece.

  ## Casos de uso que la usan
  - <caso de uso> — <en qué paso>

  ## Estructura
  Regiones y jerarquía. Acción principal, acciones secundarias.

  ## Estados
  Un apartado por estado (vacío, cargando, error, …): qué cambia y qué ve el usuario.

  ## Componentes y tokens
  Clases y tokens `--rf-*` del sistema de diseño que emplea. Nada de colores literales.

  ## Decisiones
  Qué se descartó y por qué. Enlace al canvas que lo validó.
  ```

- **Una ventana con una secuencia lleva una sola ficha**, aunque tenga varios
  artboards. La regla es «una ficha por pantalla» porque lo normal es que cada
  pantalla se entienda sola; cuando los artboards son **momentos de la misma
  ventana** —`ventana-de-sede.md`, con sus cinco
  ([#332](https://github.com/sgomez/rfirma/issues/332))— partirla obligaría a
  leer todos los ficheros para saber qué ve la persona de principio a fin. La
  ficha lista entonces los estados en una tabla con su artboard.
- **Un artboard de trabajo nace para morir.** Si para decidir has creado
  artboards o páginas aparte, al validar **se funden en el artboard de la
  pantalla y se borran**, del repositorio y del proyecto, en el mismo turno en
  que el usuario elige. La pantalla es la misma, y dos sitios donde mirarla son
  dos fuentes de verdad. Lo que se conserva es el *porqué*, en la anotación de
  la página que sobrevive.
- Marca el canvas como `validado` en el registro, con la fecha. El canvas se
  queda como fuente primaria de la decisión; no se borra ni se promociona a
  código tal cual. Su URL vive a partir de ahí en la sección "Decisiones" de la
  ficha, que es donde se lee de verdad.
- Comenta en el ticket de wayfinder o en el issue de implementación la
  **respuesta** (qué variante gana y por qué) más los enlaces al canvas y a las
  fichas, antes de cerrarlo.
- Si la decisión introduce vocabulario o una regla visual nueva y transversal,
  actualiza `docs/design/design-system.md`; si es una decisión de arquitectura,
  va a `docs/adr/`.

## Registro de canvas

Estado de los prototipos en vuelo. Es estado de proceso, no documentación de
producto: cuando un caso de uso se valida y sus fichas están escritas, su fila
puede desaparecer de aquí — el enlace al canvas ya vive en las fichas.

| Caso de uso | Canvas | Estado | Fichas |
| ----------- | ------ | ------ | ------ |
| _(ninguno en vuelo)_ | | | |

El caso de uso **v0.5 · la ventana de sede** ([#317](https://github.com/sgomez/rfirma/issues/317))
se validó el **05/09/2026** y salió de esta tabla. Es la excepción declarada a la
regla de «una ficha por pantalla»: sus cinco artboards —`SedeEspera`,
`SedeConsentimiento`, `SedeFirmando`, `SedeDesenlace` y `SedeSinCertificado`— son
cinco momentos de **una sola ventana**, así que tienen **una sola ficha**,
[`ventana-de-sede.md`](../design/ventana-de-sede.md)
([#332](https://github.com/sgomez/rfirma/issues/332)). De rebote se tocaron
`dialogo-pin` —el `autofocus` del campo del secreto—, `panel-de-firma` —el
recorte del desplegable de certificados— y `design-system`, que estrena la regla
de redacción y el componente de desplegable. Las cuatro enlazan el canvas desde
su sección «Decisiones».

El caso de uso **v0.4 · salir del sandbox** ([#250](https://github.com/sgomez/rfirma/issues/250))
se validó el 04/09/2026 y salió de esta tabla: sus siete fichas —`ventana-principal`,
`preferencias`, `acerca-de`, `dialogo-pin`, `dialogo-progreso-firma`,
`panel-de-firma` y `design-system`— enlazan el canvas desde su sección
«Decisiones».

El caso de uso **firmar un PDF en local** se validó el 31/08/2026 y salió de
esta tabla: su canvas está enlazado desde la sección «Decisiones» de cada ficha
de `docs/design/`, empezando por
[ventana-principal.md](../design/ventana-principal.md).

Estados: `en revisión` (esperando veredicto del usuario) / `validado (YYYY-MM-DD)`.
