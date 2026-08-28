# Las dependencias Java de AutoFirma se consumen desde `~/.m2`, no se copian

El motor criptográfico procede del repositorio oficial
[ctt-gob-es/clienteafirma](https://github.com/ctt-gob-es/clienteafirma). No
copiamos ni enlazamos simbólicamente sus módulos (`afirma-core`,
`afirma-crypto-*`, …) dentro de este repositorio: se compilan en su propia
ubicación con `mvn clean install` y `rfirma-native-bridge` los consume desde la
caché local de Maven como dependencias ordinarias declaradas en su `pom.xml`.

## Considered Options

Vendorizar los módulos Java dentro de `rfirma` habría hecho el repositorio
autocontenido y la compilación reproducible sin pasos previos. Se descarta
porque nos convertiría de facto en un fork del cliente @firma: cada
actualización de la suite oficial —que es donde se corrigen los fallos
criptográficos y se adapta la normativa— exigiría una fusión manual, y la
frontera entre "nuestro código" y "el suyo" dejaría de estar clara.

## Consequences

- Compilar este repositorio requiere haber compilado antes el repositorio
  original. Es un paso manual asumido, no un defecto que haya que "arreglar"
  copiando código.
- Un agente que encuentre a faltar una clase Java debe añadir la dependencia al
  `pom.xml`, nunca traerse el fuente.
