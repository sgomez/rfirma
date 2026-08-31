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
  la descripción del `.deb` no se traducen: lo que muestran es el nombre
  propio del programa.
