# rFirma se publica bajo EUPL-1.2, por la rama EUPL de Cliente @firma

`rfirma` incorpora el motor criptográfico de **Cliente @firma**, que se publica
con **licencia doble: GPL-2.0+ y EUPL-1.1**. Una licencia doble se elige: hay
que decir por cuál de las dos ramas se toma el código, porque de eso depende
bajo qué licencia se puede publicar lo que construimos encima.

`rfirma` toma la **rama EUPL-1.1** y se publica bajo **EUPL-1.2**.

El camino es el de la propia EUPL-1.1, que permite distribuir la obra y sus
derivadas bajo una versión posterior de la misma licencia. Tomando esa rama no
hace falta entrar en la compatibilidad entre la EUPL y la GPL, que es el punto
donde estas combinaciones se complican.

La EUPL encaja además con lo que es el proyecto: es la licencia que la Comisión
Europea publica para software del sector público, con textos oficiales en las
lenguas de la Unión, y es la que usa el propio cliente oficial. Es copyleft, de
modo que quien distribuya una versión modificada de `rfirma` tiene las mismas
obligaciones que nosotros.

**Esto es la decisión de proyecto y su razonamiento, no asesoría legal.** Antes
de la primera distribución pública conviene que alguien con criterio jurídico
confirme la lectura, sobre todo si en algún momento se incorpora código que
solo esté disponible bajo GPL.

## Consequences

- Hay que **declarar explícitamente** que se toma la rama EUPL-1.1 de Cliente
  @firma. Sin esa declaración, la licencia doble deja ambiguo bajo qué términos
  se usó el código, y la ambigüedad es justo lo que hay que evitar.
- La [librería nativa se distribuye como ficheros del paquete](0004-libreria-nativa-distribuida-en-el-paquete.md),
  así que **el flatpak contiene código derivado** de Cliente @firma. Las
  obligaciones de la licencia viajan con el paquete: aviso de licencia,
  atribución y disponibilidad del código fuente correspondiente.
- El diálogo [Acerca de](../design/acerca-de.md) tiene que enseñar las dos
  licencias, y su botón «Ver las licencias» tiene que llevar a los textos
  completos, que hay que empaquetar.
- Incorporar en el futuro una dependencia **solo GPL** obligaría a rehacer este
  análisis. La lista de licencias compatibles de la EUPL-1.2 es cerrada:
  conviene comprobarla antes de añadir dependencias, no después.
- El aviso de independencia respecto del cliente oficial y de la Administración
  no es una consecuencia de la licencia, pero se apoya en ella: reutilizar
  código publicado por la Administración no implica ningún respaldo suyo, y el
  «Acerca de» lo dice.
