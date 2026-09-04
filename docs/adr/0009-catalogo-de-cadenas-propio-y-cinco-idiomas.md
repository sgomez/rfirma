# Catálogo de cadenas propio, cinco idiomas, y errores que traducen situaciones

`rfirma` reutiliza el motor criptográfico de Cliente @firma, así que la
suposición razonable es que reutilice también sus `.properties`: están
traducidos y la licencia ya lo permite (la rama EUPL está tomada en el
[ADR-0008](0008-licencia-eupl-1-2.md)). **No lo hacemos.** El catálogo de
cadenas de `rfirma` se escribe desde cero con el vocabulario de `CONTEXT.md`.

La razón es que la interfaz es un rediseño completo
([ADR-0006](0006-firma-visible-se-configura-sobre-el-documento.md) y
[ADR-0007](0007-cabecera-unica-sin-barra-de-menus.md)): las cadenas de
AutoFirma tienen forma de diálogo Swing —títulos de ventana, aceleradores de
menú, confirmaciones modales— y no hay correspondencia con nuestras pantallas.
Además su terminología es de la que `CONTEXT.md` se aparta a propósito.
Reutilizarlas importaría el vocabulario que hemos decidido no usar.

Los `.properties` cooficiales de AutoFirma **sí** se consultan como referencia
terminológica: cómo dice el cliente oficial «certificado» o «almacén» en
euskara es lo que el usuario ya ha visto, y apartarse de eso sin motivo es
gratuito. Referencia para elegir palabras, no origen de cadenas.

## Cinco idiomas: `es`, `eu`, `ca`, `en`, `gl`

Español, euskara, català, inglés y galego. **No se soportan variantes de ningún
idioma** —ni un `va_ES` frente al `ca`, ni un `es-MX` frente al `es`—, y el
criterio es técnico y se aplica igual a todos.

Las reglas de plural de `Intl.PluralRules` se definen sobre el **idioma**, y una
variante no aporta ninguna. Medido en el
[#151](https://github.com/sgomez/rfirma/issues/151): `es` y `ca` dan `one`,
`many`, `other`; `eu`, `gl` y `en` dan `one`, `other`; y **`va` se resuelve a
`und`**, con `other` y nada más. CLDR no lo reconoce e i18next no lo valida ni lo
normaliza: se lo pasa a `Intl.PluralRules` tal cual, así que un catálogo en
valencià estaría **roto para plurales antes de escribir la primera cadena**, sin
error y sin aviso. Esto alcanza también a `signing/language.rs`, no sólo al
frontal.

**La lista es ampliable, por colaboración y no por calendario.** No es un
conjunto cerrado que se tome «entero o nada»: un idioma nuevo entra al 0 % y no
se publica hasta estar al 100 %, y lo que hace seguro abrirla es la puerta de
abajo. Hoy se publican `es` y `en`; `ca`, `eu` y `gl` existen al 0 % y **no se
prometen a nadie**, porque una traducción generada que nadie revisa es, en una
aplicación de firma, peor que la ausencia.

## La completitud es una puerta de construcción, no un filtro en ejecución

La promesa es **nunca media pantalla en otro idioma**, y se sostiene así: **el
castellano siempre al 100 %; los demás, al 100 % o al 0 %; y el idioma que no
esté al 100 % no llega a existir en una versión publicada.** El estado intermedio
no es algo que se detecte: es algo que no se puede representar. `LANGUAGES` se
deriva de qué catálogos existen.

Con ella va un **respaldo al castellano** (`returnEmptyString: false`): una
cadena sin traducir cae al castellano en vez de pintar un hueco. No es la red de
la persona usuaria —lo intermedio no se publica—, es la red de quien desarrolla
mientras trabaja.

## La fuente de verdad de las cadenas es gettext

La cadena es la ortodoxa: plantilla **`messages.pot`** versionada → cinco
**`.po`** versionados, con `msgmerge` de bisagra → los `.ts` **generados y no
versionados**, porque son datos compilados. **Ningún fichero de cadenas se edita
en TypeScript**, `en` incluido.

El `msgid` es **la clave con puntos y no el texto castellano**: con el texto,
corregir una errata en castellano invalidaría las cinco traducciones de golpe.

Esto no toca el catálogo propio que este ADR decide —las cadenas siguen siendo
nuestras y escritas con el vocabulario de `CONTEXT.md`—. Cambia el formato en el
que viven y quién puede aportarlas: un `.pot` con sus comentarios `#.` es lo que
permite que alguien de fuera traduzca sin tocar código.

## Considered Options

- **Las seis lenguas de `LanguageManager.AFIRMA_DEFAULT_LOCALES`** —añadiendo el
  valencià—, tomadas «enteras y no por partes», que fue la decisión original de
  este ADR. Su argumento era que un subconjunto de las lenguas cooficiales no es
  una decisión técnica sino una asimetría entre lenguas, y en una herramienta de
  firma ante la Administración se lee como tal. **Ese argumento sigue en pie y no
  es el que se aplica aquí**: no se elige entre lenguas, se elige entre idioma y
  variante, y la regla —ninguna variante de ningún idioma— cae igual sobre el
  `es-MX`. Lo que lo remata es la medición del #151: el catálogo en `va` no
  funcionaría.
- **Una lista cerrada de idiomas**, cualquiera que fuese. Se retira con la puerta
  de completitud: en cuanto un idioma incompleto no puede publicarse, abrir la
  lista deja de tener riesgo, y cerrarla sólo impide que alguien contribuya.
- **`isComplete()`, el filtro en tiempo de ejecución** con el que se implementó
  la promesa al principio. Escondía el idioma incompleto de un desplegable que,
  aun así, lo llevaba dentro: la versión publicada seguía conteniendo media
  traducción, y lo único que faltaba para enseñarla era un fallo del filtro.
- **Los catálogos escritos en TypeScript**, que es como nacieron. Obligan a
  traducir tocando código, que es justo lo que impide que traduzca quien sabe el
  idioma y no el proyecto.
- **Lingui** ([#151](https://github.com/sgomez/rfirma/issues/151)),
  **`i18next-conv`** ([#163](https://github.com/sgomez/rfirma/issues/163)) y
  **Paraglide JS** ([#162](https://github.com/sgomez/rfirma/issues/162)).
  Evaluadas y descartadas las tres, y **fallan en el mismo sitio**: no traen
  puerta de completitud, o la traen rota, y **fallan en silencio y con salida 0**.
  La promesa de este ADR no es lo que el mercado optimiza. Medido en
  [`i18next-y-el-po.md`](../research/i18next-y-el-po.md).

## Consequences

- **Cobertura obligatoria, calidad revisable.** Caer al castellano a mitad de
  pantalla no es una degradación aceptable, y por eso la completitud es una
  puerta. La revisión lingüística por un hablante nativo es posterior y se anota
  en el repositorio, nunca en la interfaz.
- **Los errores no se traducen: se clasifican.** `cryptoki` devuelve códigos
  (`CKR_PIN_INCORRECT`, `CKR_TOKEN_NOT_PRESENT`) y el puente Java devuelve
  excepciones cuyo texto está incrustado en el código —`afirma-crypto-pdf` no
  tiene ni un `.properties` localizado—. Ninguno de los dos se enseña como
  mensaje. Se traducen a una **situación** del catálogo de `rfirma`, que sí está
  traducida; el texto original viaja aparte, en un detalle técnico plegado y **sin
  traducir**, para poder pegarlo en un informe de fallo. Lo que no sepamos
  clasificar cae en un mensaje genérico traducido más su detalle técnico crudo.
- **El texto de la firma visible sigue al idioma de la aplicación.** AutoFirma lo
  deja en castellano fijo (`PdfSessionManager.getDefaultLayer2Text()` lo construye
  incrustado, sin pasar por ningún `ResourceBundle`); nosotros no. Es contenido
  del PDF y lo lee el destinatario, pero el recorrido enseña el recuadro en el
  visor antes de firmar, así que no hay sorpresa. No lleva ajuste propio.
- **El empaquetado queda fuera del circuito de traducción.** El `.desktop` y el
  `metainfo.xml` no se traducen: lo que muestran es el nombre propio del programa.
