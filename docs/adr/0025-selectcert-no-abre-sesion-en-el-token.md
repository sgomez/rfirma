# `selectcert` no abre sesión en el token: una operación que no firma no pide el PIN

La sede que pide `selectcert` quiere identidad, no una firma: necesita el
certificado que la persona elige, y nada más. AutoFirma resuelve esa petición
pidiendo al almacén sólo los certificados con clave privada
(`checkPrivateKeys=true`), y en un token PKCS#11 eso obliga a abrir sesión, que
es tanto como pedir el PIN antes de que haya nada que firmar.

rFirma no lo hace: lista los certificados por sus objetos públicos y devuelve el
elegido sin abrir sesión en el token. **Una operación que no firma no pide el
PIN.**

## Consequences

- Un certificado presente en el token sin clave privada asociada puede llegar a
  ofrecerse en la lista. Es el precio, y es el lado correcto del que
  equivocarse: quien elige un certificado inservible lo descubre al firmar, y
  quien sólo se identifica no ha tecleado su PIN por el camino.
- Frente al cliente publicado, rFirma sale **no conforme** en la exigencia
  `selectcert_only_offers_certificates_with_a_private_key` de la suite de
  conformidad, y su línea base lo declara así citando este ADR: es una
  desviación deliberada, no una sorpresa.

## Considered Options

- **Imitar `checkPrivateKeys=true`.** Descartada: pedir el PIN en una operación
  que no firma enseña a la persona a teclearlo sin mirar por qué se lo piden,
  que es justo el hábito que un cliente de firma no debe cultivar; y deja la
  sesión del token abierta para un trámite que no la necesita.
- **Filtrar por clave privada sin abrir sesión.** Descartada: el atributo que lo
  diría no es legible en todos los módulos PKCS#11 sin sesión, así que el filtro
  sería silenciosamente distinto según el token, y un filtro que a veces filtra
  es peor que no filtrar.
