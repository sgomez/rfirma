# Catálogo de cadenas propio, seis idiomas, y errores que traducen situaciones

`rfirma` reutiliza el motor criptográfico de Cliente @firma, así que la
suposición razonable es que reutilice también sus `.properties`: están
traducidos a las seis lenguas y la licencia ya lo permite (la rama EUPL está
tomada en el [ADR-0008](0008-licencia-eupl-1-2.md)). **No lo hacemos.** El
catálogo de cadenas de `rfirma` se escribe desde cero con el vocabulario de
`CONTEXT.md`.

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

## Los seis idiomas

Español, català, euskara, galego, valencià e inglés: la misma lista que
`LanguageManager.AFIRMA_DEFAULT_LOCALES` del cliente oficial. La lista se toma
entera y no por partes. Un subconjunto de las lenguas cooficiales no es una
decisión técnica sino una asimetría entre lenguas, y en una herramienta de
firma ante la Administración se lee como tal. El coste tampoco lo justifica:
el valencià es en la práctica el català con variantes léxicas —por eso
AutoFirma mantiene `va_ES` como locale propio— y el galego es el único texto
verdaderamente nuevo.

## Consequences

- **Cobertura obligatoria, calidad revisable.** Un idioma no aparece en el
  desplegable de [Preferencias](../design/preferencias.md) si le falta una
  sola clave; lo comprueba el harness de QA. Caer al castellano a mitad de
  pantalla no es una degradación aceptable. La revisión lingüística por un
  hablante nativo es posterior y se anota en el repositorio, nunca en la
  interfaz.
- **Los errores no se traducen: se clasifican.** `cryptoki` devuelve códigos
  (`CKR_PIN_INCORRECT`, `CKR_TOKEN_NOT_PRESENT`) y el puente Java devuelve
  excepciones cuyo texto está incrustado en el código —`afirma-crypto-pdf` no
  tiene ni un `.properties` localizado—. Ninguno de los dos se enseña como
  mensaje. Se traducen a una **situación** del catálogo de `rfirma`, que sí
  está traducida a las seis; el texto original viaja aparte, en un detalle
  técnico plegado y **sin traducir**, para poder pegarlo en un informe de
  fallo. Lo que no sepamos clasificar cae en un mensaje genérico traducido más
  su detalle técnico crudo.
- **El texto de la firma visible sigue al idioma de la aplicación.** AutoFirma
  lo deja en castellano fijo (`PdfSessionManager.getDefaultLayer2Text()` lo
  construye incrustado, sin pasar por ningún `ResourceBundle`); nosotros no.
  Es contenido del PDF y lo lee el destinatario, pero el recorrido enseña el
  recuadro en el visor antes de firmar, así que no hay sorpresa. No lleva
  ajuste propio.
- **El empaquetado queda fuera del circuito de traducción.** El `.desktop` y
  el `metainfo.xml` del flatpak no se traducen: lo que muestran es el nombre
  propio del programa.

## Enmienda: son cinco idiomas, y la completitud se vigila en la construcción

Añadido con el hito v0.3 ([#148](https://github.com/sgomez/rfirma/issues/148)).
La promesa de este ADR —**nunca media pantalla en otro idioma**— se mantiene
entera. Lo que cambia es la lista, el mecanismo que la sostiene y de dónde salen
las cadenas. Donde el texto de arriba diga «seis», léase esta enmienda.

### Cinco idiomas: `es`, `eu`, `ca`, `en`, `gl`

El **valencià sale**, y el criterio es técnico y se aplica igual a todos: **no
se soportan variantes de ningún idioma**, tampoco un `es-MX` frente al `es`. Las
reglas de plural de `Intl.PluralRules` se definen sobre el idioma y una variante
no aporta ninguna. Medido en el [#151](https://github.com/sgomez/rfirma/issues/151):
`es` y `ca` dan `one`, `many`, `other`; `eu`, `gl` y `en` dan `one`, `other`; y
**`va` se resuelve a `und`**, con `other` y nada más. CLDR no lo reconoce e
i18next no lo valida ni lo normaliza: se lo pasa a `Intl.PluralRules` tal cual.
Ese catálogo está roto para plurales **hoy**, antes de migrar nada, y todo plural
en valencià se resolvería siempre por `other`, sin error y sin aviso.

Esto retira el argumento de arriba —«un subconjunto de las lenguas cooficiales
no es una decisión técnica sino una asimetría»— sólo en lo que tocaba al
valencià como *locale* propio: la asimetría que aquel párrafo temía era elegir
entre lenguas, y aquí no se elige entre lenguas sino entre idioma y variante.
Toca también `signing/language.rs`, no sólo el frontal.

### Se retira la promesa de una lista fija: un idioma entra por colaboración

Este ADR prometía seis idiomas como conjunto cerrado que se toma «entero y no
por partes». Se retira. **v0.3 publica `es` y `en`**; `ca`, `eu` y `gl` existen
al 0 % y **no se prometen a nadie** —llevan así desde v0.1, y una traducción
generada que nadie revisa es, en una aplicación de firma, peor que la ausencia—.
La lista es **ampliable, por colaboración y no por calendario**, y el arnés de
abajo es justamente lo que permite abrirla sin riesgo: un idioma nuevo entra al
0 % y no se publica hasta estar al 100 %.

### La completitud deja de ser un filtro en ejecución y pasa a ser una puerta

«Un idioma no aparece en el desplegable si le falta una sola clave; lo comprueba
el harness de QA» se implementó como `isComplete()`, un filtro **en tiempo de
ejecución** que escondía el idioma incompleto de un desplegable que, aun así, lo
llevaba dentro. Se sustituye por una regla de construcción: **el castellano
siempre al 100 %; los demás, al 100 % o al 0 %; y el idioma que no esté al 100 %
no llega a existir en una versión publicada.** El estado intermedio deja de ser
algo que se detecta y pasa a ser algo que no se puede representar.

Con ella entra un **respaldo al castellano** (`returnEmptyString: false`): una
cadena sin traducir cae al castellano en vez de pintar un hueco. Deja de ser la
red de la persona usuaria —lo intermedio no se publica— y pasa a ser la red de
quien desarrolla mientras trabaja. Desaparecen `completeLanguages()` e
`isComplete()`, y `LANGUAGES` se deriva de qué catálogos existen.

### La fuente de verdad de las cadenas es gettext, no TypeScript

Los catálogos dejan de escribirse en TypeScript. La cadena es la ortodoxa:
plantilla **`messages.pot`** versionada → cinco **`.po`** versionados, con
`msgmerge` de bisagra → los `.ts` **generados y no versionados**, porque son
datos compilados. **Ningún fichero de cadenas se edita en TypeScript**, `en`
incluido. El `msgid` es la clave con puntos y no el texto castellano: con el
texto, corregir una errata en castellano invalidaría las cinco traducciones de
golpe.

Esto no toca el «catálogo propio» que este ADR decide —las cadenas siguen siendo
nuestras y escritas con el vocabulario de `CONTEXT.md`—, ni la clasificación de
errores en situaciones, ni que el texto de la firma visible siga al idioma de la
aplicación. Cambia el formato en el que viven y quién puede aportarlas: un `.pot`
con sus comentarios `#.` es lo que permite que alguien de fuera traduzca sin
tocar código.

Se evaluaron y se descartaron **tres** herramientas —Lingui
([#151](https://github.com/sgomez/rfirma/issues/151)), `i18next-conv`
([#163](https://github.com/sgomez/rfirma/issues/163)) y Paraglide JS
([#162](https://github.com/sgomez/rfirma/issues/162))—, y las tres fallan en el
mismo sitio: no traen puerta de completitud, o la traen rota, y **fallan en
silencio y con salida 0**. La promesa de este ADR no es lo que el mercado
optimiza.
