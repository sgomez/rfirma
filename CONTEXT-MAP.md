# Mapa de contextos

El repositorio tiene dos bounded contexts, cada uno con su glosario. Un término significa lo que
dice el glosario del contexto en que se usa, y el mismo nombre puede significar cosas distintas en
cada uno. Por ejemplo, en la aplicación el **cliente publicado** es el `autoscript.js` de la sede;
en la suite, **cliente** es la aplicación a prueba y el `autoscript.js` es parte de la **sede**.

| Contexto | Glosario | Qué es |
|---|---|---|
| Aplicación rFirma | `CONTEXT.md` | La aplicación de firma, su protocolo con la sede y sus pruebas, incluidos el banco de conformidad y el cliente de canal. |
| Suite de conformidad | `rfirma-conformance/CONTEXT.md` | La herramienta local que mide si una aplicación, AutoFirma o rFirma, cumple el protocolo `afirma://`. |

Las decisiones de la aplicación y del repositorio viven en `docs/adr/`; las que solo afecten a la
suite, cuando las haya, irán a un `docs/adr/` junto a su glosario.
