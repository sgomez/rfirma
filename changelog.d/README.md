# Fragmentos del CHANGELOG

Cada issue que cambia el comportamiento visible de rFirma entrega aquí su
fragmento, en vez de escribir directamente en `CHANGELOG.md` (ID-153). Los
sub-issues de un hito corren en paralelo, y un bloque `## [Unreleased]`
compartido sería un conflicto de fusión garantizado: dos ramas tocando la
misma línea del mismo fichero. Un fragmento por issue evita el choque porque
cada uno escribe en su propio fichero.

## Cómo escribir un fragmento

- Un fichero por issue: `changelog.d/<issue>.md`, con el número tal y como
  aparece en el rastreador (por ejemplo, `changelog.d/252.md`).
- Contenido: una o varias líneas en formato
  [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/), agrupadas bajo
  su categoría:

  ```
  ### Added
  - Mecanismo de CHANGELOG por fragmentos (#252).
  ```

  Categorías disponibles: `Added`, `Changed`, `Fixed`, `Removed`, `Security`.
- En castellano, como el resto de la documentación del proyecto.
- Cada línea de nota termina con el issue entre paréntesis, `(#N)`.

## Cómo se publican

`just changelog-release <version>` reúne todos los fragmentos presentes en la
sección `## [<version>]` de `CHANGELOG.md` y borra los ficheros de
`changelog.d/` una vez incorporados. Este `README.md` no se borra nunca.
