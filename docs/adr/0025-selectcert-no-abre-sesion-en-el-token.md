# `selectcert` no abre sesión en el token: una operación que no firma no pide el PIN

La sede que pide `selectcert` quiere identidad, no una firma: necesita el
certificado que la persona elige, y nada más. AutoFirma resuelve esa petición
pidiendo al almacén sólo los certificados con clave privada
(`checkPrivateKeys=true`), y en un token PKCS#11 eso obliga a abrir sesión, que
es tanto como pedir el PIN antes de que haya nada que firmar.

rFirma no lo hace: lista los certificados por sus objetos públicos y devuelve el
elegido sin abrir sesión en el token. **Una operación que no firma no pide el
PIN.**

Sin sesión no hay forma fiable de saber qué certificado tiene detrás una clave
privada emparejada, pero sí de saber qué certificado **no puede firmar por su
propio contenido**: uno de CA (`basicConstraints` con `cA=true`), o uno que
declara `keyUsage` sin `digitalSignature` ni `nonRepudiation`. Esos dos se
descartan del listado de un almacén de tarjeta o token **sin sesión**; el que
no declara `keyUsage` se conserva, porque su ausencia no dice que no pueda
firmar. Con sesión abierta ya hay una clave privada que emparejar, así que el
filtro por contenido no entra: el listado sigue filtrando solo por esa clave
emparejada, igual que en NSS.

## Consequences

- Un certificado presente en el token sin clave privada asociada, pero cuyo
  contenido no lo descarta, puede llegar a ofrecerse en la lista. Es el
  precio, y es el lado correcto del que equivocarse: quien elige un
  certificado inservible lo descubre al firmar, y quien sólo se identifica no
  ha tecleado su PIN por el camino.
- Los almacenes NSS y los tokens con sesión abierta no cambian de
  comportamiento por el filtro de sesión: siguen filtrando por clave privada
  emparejada, que ahí sí es legible.
- La suite de conformidad no mide si ni cuándo se pide el PIN: es interfaz,
  no protocolo, así que ninguna de sus comprobaciones sale no conforme por
  este ADR. Lo que sí viaja es qué certificado vuelve: si la suite llega a
  medir `checkPrivateKeys=true` —un certificado sin clave en el token, y la
  exigencia de que vuelva uno con clave—, rFirma saldrá **no conforme** por
  la primera consecuencia, y esa desviación deliberada tendrá su porqué en
  este ADR.

## Considered Options

- **Imitar `checkPrivateKeys=true`, o pedir el PIN al listar para
  identificarse.** Descartada: pedir el PIN en una operación que no firma
  enseña a la persona a teclearlo sin mirar por qué se lo piden, que es justo
  el hábito que un cliente de firma no debe cultivar; y deja la sesión del
  token abierta para un trámite que no la necesita.
- **Filtrar por clave privada, o por emparejamiento con la clave pública, sin
  abrir sesión.** Descartada: el atributo que lo diría no es legible en todos
  los módulos PKCS#11 sin sesión, así que el filtro sería silenciosamente
  distinto según el token, y un filtro que a veces filtra es peor que no
  filtrar. El filtro por contenido del certificado no tiene este problema:
  lee del propio DER, igual en cualquier módulo.
