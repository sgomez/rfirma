# Acerca de rFirma

Identidad de la aplicación, **estado de la versión**, aviso de independencia
respecto del cliente oficial y licencias. Se abre desde el menú de la
[cabecera](cabecera.md) y desde la acción de la franja de notificación de la
[ventana principal](ventana-principal.md). Desde él se instala la versión nueva,
con su propia confirmación.

Componentes: `rfirma-app/src/about/AboutDialog.tsx` y, para instalar,
`rfirma-app/src/updates/InstallUpdateDialog.tsx`. Historias:
`AboutDialog.stories.tsx`, en «Acerca de/1 · Diálogo», e
`InstallUpdateDialog.stories.tsx`, en «Acerca de/2 · Instalar actualización».

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido.
- Actualizar rFirma — es la única pantalla que dice si hay versión nueva.

## Historias

El diálogo cambia solo en la tarjeta de la versión:

- `UpToDate`: al día.
- `NewVersionInstallable`: versión nueva que la aplicación puede instalar por sí
  misma, con su botón.
- `NewVersionAnnouncedOnly`: versión nueva que no es instalable desde aquí
  (empaquetado que gestiona otro): se anuncia y nada más.
- `NewVersionWithoutOffer`: versión nueva con «Avisar de versiones nuevas»
  apagado en [Preferencias](preferencias.md): sigue el estado, pero sin oferta.
- `Confirming` (instalar): la confirmación con la versión.

Los otros dos estados de instalar —instalando y fallida— se llegan por un gesto
y no tienen historia; los cubre `AboutDialog.test.tsx`.

## Flujo

1. Se abre con lo que se sabía desde el arranque, y **vuelve a preguntar al
   puerto** `VersionCheck`: lo que conteste sustituye a lo sabido. Sin respuesta
   la tarjeta se queda como estaba.
2. Si la versión es instalable y se ofrece, el botón abre
   `InstallUpdateDialog` **encima**, que pide confirmación con el número de
   versión.
3. Confirmar instala; si sale bien, la aplicación se cierra. Si falla, el mismo
   diálogo lo explica con una causa por resultado (`updates.install.failed.*`) y
   *Acerca de* sigue abierto. Mientras instala no hay salida.

## Estructura

Un `Dialog` sin relleno propio y en tres zonas:

1. **Cabecera**: el nombre con la versión en una insignia a su lado y la
   **tarjeta del estado de la versión**.
2. **Ficha técnica**, en lista de términos: licencia, «Firma con» y código fuente.
3. **Aviso de independencia**, tras un filete, y el pie con «Cerrar» y nada más.

El código fuente es un **enlace que se pulsa**: abre el navegador, y lo abre el
backend por identificador, como el resto de destinos externos.

## Estado de la versión

**Una tarjeta, y nada más.** Quien tiene el diálogo delante ya ha instalado
rFirma, así que las órdenes de alta del repositorio no le sirven para
actualizar: quien se dio de alta actualiza con su gestor de paquetes, y quien no
las encuentra donde se instala, en `packaging/repo/index.html`.

- **Hay versión nueva**: borde marcado, flecha hacia arriba y
  `updates.newVersion`. Con ella, `updates.install.action` si es instalable y se
  ofrece.
- **Al día**: borde discreto, marca de verificación atenuada y
  `about.update.upToDate`.

Sin red **no hay tercer estado**: se calla. Un «no se ha podido comprobar» sería
un fallo que no le pide nada a nadie.

## Textos

Claves: `app.name`, `about.version`, `about.update.upToDate`,
`updates.newVersion`, `updates.install.*`, `about.facts.*`, `about.licenses.*`,
`about.repository`, `about.independenceLead`, `about.independence` y
`actions.close`. Los textos no se copian aquí: se leen en `po/es.po`.

**La frase de qué hace la aplicación se retiró.** Decía «con tu certificado o tu
tarjeta criptográfica», que dejó de ser cierto con la v0.4, y que el documento y
la clave privada no salen del ordenador, que tranquiliza sobre lo evidente. Lo
que la garantía significa de verdad —que la clave privada nunca cruza a Java— está
en el [ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md).

## El aviso de independencia

Va al pie del cuerpo, con un icono de información atenuado: el encabezado
(`about.independenceLead`) en negrita y el resto en texto atenuado. **Tiene que
estar:** una aplicación que firma ante la Administración con la misma
criptografía que la oficial se puede confundir con ella, y esa confusión hay que
deshacerla en el sitio donde la gente va a preguntar qué es esto. Es la misma
cadena que enseña el [primer arranque](primer-arranque.md).

## Licencias y código fuente

Ver [ADR-0008](../adr/0008-licencia-eupl-1-2.md) para por qué esa combinación se
sostiene.

| Término | Valor |
| --- | --- |
| Licencia | EUPL-1.2 |
| Firma con | Bibliotecas del proyecto Cliente @firma, con sus licencias (GPL-2.0+ / EUPL-1.1) debajo, atenuadas |
| Código fuente | El repositorio, enlace que se pulsa |

## Componentes y tokens

`Dialog`, `Button`, `Badge`, `Row` y `Stack`, los iconos de versión nueva, al
día, información y abrir fuera; `.rf-heading`, `.rf-prose`, `.rf-label`,
`.rf-hint`, `.rf-text-muted`, `--rf-surface`, `--rf-border-subtle|-strong`,
`--rf-radius-md` y `--rf-space-*`. La geometría —460 px de ancho, el nombre
bajado a 32 px y alineado por la línea base con la insignia— es de
`AboutDialog.css`.

## Decisiones

**Tres zonas, la versión en tarjeta y la ficha técnica siempre a la vista.** Con
ella visible no quedaba nada que desplegar y se quitó «Ver las licencias» en vez
de inventarle otro destino.

**El enlace al código se pulsa.** Revoca el criterio anterior de que no hubiera
enlaces mientras `opener:deny-open-url` siguiera denegado: el enlace no abre el
permiso, lo abre el backend.

**El aviso lleva icono.** Revoca el criterio de que fuera un párrafo sin icono
para no darle aire de alarma: una información atenuada no alarma y se ve.

**Instalar se confirma antes de hacerse**, y su fallo no cierra *Acerca de*:
quien acaba de pedir actualizar necesita ver por qué no se pudo y seguir donde
estaba.

El canal propio y sus tres repositorios están en el
[ADR-0015](../adr/0015-canal-de-distribucion-propio.md).
