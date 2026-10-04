# Prototipado

Dónde vive la salida de la skill `prototype` en este repo (y de quien la
invoque: `/wayfinder` con un ticket `wayfinder:prototype`, `/grill-with-docs`,
o el usuario directamente).

## La regla

La rama **UI** de `prototype` ("¿qué aspecto debería tener esto?") **no** se
construye como ruta throwaway con `?variant=` dentro de la app React. Se
explora en **Claude Design**, con los componentes reales, y una vez validada se
escribe como código, historias y ficha.

Claude Design es **solo** la superficie de exploración: es desechable, sirve
para que el usuario mire y decida. La verdad vive en el repo: el código, sus
historias de Storybook y las fichas de `docs/design/`.

La decisión y lo que se descartó están en el ADR-0045, que sustituye al ADR-0033.

La rama **lógica** (`LOGIC.md`: máquinas de estado, flujo trifásico, errores de
PKCS#11) **no** cambia: sigue siendo el fichero HTML único y local que describe
la skill. Claude Design es para pantallas, no para simuladores.

## El flujo

1. **Cuándo explorar**: solo si hay una decisión de aspecto que mirar antes de
   escribir código. Lo que la ficha o el sistema de diseño ya resuelven va
   directo a código.
2. **Sincronizar antes**: `/design-sync` sube a «rFirma Components» los
   componentes y las historias de `main`. Se explora sobre eso, no sobre
   dibujos a mano.
3. **Explorar**: en Claude Design, con los primitivos y las piezas de dominio
   reales. Varias variantes radicalmente distintas (3 por defecto, tope 5) para
   la pantalla que sea el nudo del caso de uso: distinta jerarquía y distinta
   acción principal, no distinto color.
4. **Validar**: siempre humana. La persona usuaria elige.
5. **De lo validado a código**: el componente, sus historias (una por
   variante, junto al componente) y la ficha.

Si falta la autorización de design (la llamada de lectura falla por scopes),
dilo y cae a la rama local de `UI.md` en lugar de bloquear el prototipo.

## Proyectos de Claude Design

- **Vigente**: «rFirma Components», `projectId`
  `312bca0c-2f94-494a-820a-e947e03f9ade`. Lo llena `/design-sync` desde
  Storybook; su configuración está en `.design-sync/`.
- **Congelados, solo histórico** (no se escriben ni se sincronizan):
  - «Autofirma de escritorio en Rust», `c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132`,
    el canvas de artboards.
  - «rFirma Design System», `ca5219d0-609a-4ce1-957f-e1d1d38e0c8c`, el bundle
    anterior.

## Sistema de diseño

El CSS de `rfirma-app/src/design-system/bundle/` es código fuente normal, sin
sello: se edita como cualquier otro fichero y una prueba de grada A impide que
se separe de `docs/design/design-system.md`. Léela antes de explorar y **no
fijes colores a mano**.

## La ficha de pantalla

Una ficha `docs/design/<pantalla>.md` por pantalla, no por caso de uso. Una
pantalla que aparece en varios flujos tiene **una sola ficha**, que lista los
flujos que la usan; una ventana con una secuencia de momentos
(`ventana-de-sede.md`) también tiene una sola.

```markdown
# <Nombre de la pantalla>

Una frase: qué resuelve y en qué punto del flujo aparece.

## Casos de uso que la usan
- <caso de uso> — <en qué paso>

## Estructura
Regiones y jerarquía. Acción principal, acciones secundarias.

## Estados
Un apartado por estado: qué cambia y qué ve el usuario, con su historia.

## Componentes y tokens
Clases y tokens `--rf-*` del sistema de diseño que emplea. Nada de colores literales.

## Decisiones
Qué se descartó y por qué.
```

- Comenta en el ticket de wayfinder o en el issue de implementación la
  **respuesta** (qué variante gana y por qué) antes de cerrarlo.
- Si la decisión introduce vocabulario o una regla visual nueva y transversal,
  actualiza `docs/design/design-system.md`; si es de arquitectura, va a
  `docs/adr/`.
