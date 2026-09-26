# La interfaz se prototipa en Claude Design, y lo que manda vive en el repositorio

Las pantallas de rFirma se dibujan, comparan y validan en **Claude Design**: un proyecto con un
lienzo por caso de uso y un artboard por pantalla en cada estado. Es la superficie donde se decide
una pantalla, y no se sustituye por rutas de prueba dentro de la aplicación. Pero **nada normativo
vive solo allí**, porque el repositorio es público y su interfaz no puede quedar especificada detrás
de un servicio con cuenta. Tres cosas del repositorio son la referencia:

- **La ficha `docs/design/<pantalla>.md`** es la referencia normativa de cada pantalla. El lienzo es
  la fuente primaria de la decisión que la sostiene, y la ficha lo enlaza.
- **El bundle de `rfirma-app/src/design-system/bundle/`** es el sistema de diseño normativo. Es el
  CSS que consume la aplicación, y viaja al proyecto de Claude Design, nunca al revés. Si el
  proyecto y el bundle no coinciden, gana el bundle.
- **`docs/design/artboards/`** es la copia literal de los artboards, para revisar la interfaz sin
  cuenta de Claude. Se conserva hasta cerrar la v1.0.

La rama lógica de un prototipo (máquinas de estado, flujos) no pasa por Claude Design: sigue siendo
un HTML local.

## Considered Options

**Cortar con Claude Design** una vez transcrita la interfaz a JSX: borrar los artboards, quitar
toda referencia al proyecto y prototipar sin servicio externo. Era el plan inicial, y los artboards
se importaron como algo de un solo uso. Se descartó porque, versión tras versión, el lienzo siguió
siendo el sitio donde se decidía cada pantalla. Sin él, cada cambio de interfaz se decidiría a
ciegas o sobre la propia aplicación. La preocupación que motivaba el corte, que la interfaz no
dependa de una cuenta, ya la resuelven las fichas, el bundle versionado y la copia de los artboards.

**Prototipar dentro de la aplicación**, con rutas desechables y variantes por parámetro. Se
descartó porque mezcla código de prueba en el árbol que se publica y no permite comparar varios
estados uno al lado del otro, que es lo que de verdad valida una pantalla.
