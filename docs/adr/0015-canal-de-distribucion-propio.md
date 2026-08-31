# El canal de distribución es propio: repositorio en `rfirma.sgomez.me` y bundle en GitHub Releases

El [ADR-0004](0004-libreria-nativa-distribuida-en-el-paquete.md) fijó que **flatpak es el
único canal soportado**, y el [#22](https://github.com/sgomez/rfirma/issues/22) eligió
**Flathub** como destino de paso, sin que esa parte llegara a decidirse: se dio por hecha.
Aquí se decide, y la respuesta es que **v0.1 no va a ninguna tienda**.

Flatpak sigue siendo el formato. Lo que cambia es que el artefacto llega al usuario por dos
sitios nuestros:

- **GitHub Releases** guarda el `.flatpak` de cada versión, con su `SHA256SUMS`. Es el
  fichero suelto, para quien quiera instalar a mano o sin remoto.
- **`rfirma.sgomez.me` sirve un repositorio ostree** —`flatpak build-export` a un directorio
  y `flatpak build-update-repo`, ficheros estáticos por HTTPS— más un `.flatpakref` de un
  clic. Es el camino recomendado, y el único que da actualizaciones.

## Por qué el repositorio y no solo el bundle

**Un bundle instalado no se actualiza nunca.** `flatpak update` no sabe de dónde vino un
`.flatpak` suelto. En una aplicación cualquiera eso es una molestia; en esta no, porque el
mapa lleva medidas **tres** maneras de invalidar una firma en silencio —`extraParams`, `TIME`
y zona horaria, las tres con `Digest Mismatch` y sin excepción, unificadas en el sello de
sesión del [ADR-0012](0012-sello-de-sesion-una-sola-invariante.md)—. Si una versión se lleva
alguna por delante, sin canal de actualización el usuario se queda ahí y no hay forma de
avisarle.

Lo que cuesta el repositorio es un paso en el carril de etiquetas del CI, una clave GPG y un
secreto de despliegue. Aplazarlo tiene precio y conviene saberlo: flatpak **no migra el
origen** de una aplicación ya instalada, así que quien instale desde bundle tendrá que
desinstalar y reinstalar desde el remoto el día que exista.

El repositorio va **firmado con GPG**, con la clave pública en la web y su huella dentro del
`.flatpakref`, de modo que `flatpak` verifica cada actualización solo. Un remoto con
`--no-gpg-verify` no es defendible en una aplicación de firma electrónica: sin ficha en un
centro de software, esa firma es la única cadena de confianza que el usuario tiene.

## El runtime sigue viniendo de Flathub

El bundle no lleva `org.gnome.Platform//50` dentro, así que sin el remoto de Flathub añadido
la instalación falla con «runtime not found». **Consumir un runtime no es publicar en la
tienda** y no lo condiciona nada de lo anterior. Se documenta como requisito de instalación
—una línea de `flatpak remote-add --if-not-exists flathub …`, que la mayoría de escritorios
ya traen puesta— y no se resuelve por otro lado: servir el runtime desde nuestro repositorio
son cientos de megas para ahorrar un comando.

## Consecuencias

- El `type: dir` del manifiesto **deja de ser un problema**: lo prohibía el linter de Flathub,
  y nada más. `flatpak-builder` lo construye igual.
- Construir sin red deja de ser una obligación externa y pasa a ser preferencia nuestra. El
  [ADR-0013](0013-estructura-del-repositorio-y-cadena-de-compilacion.md) ya la había adoptado
  por su cuenta —fuentes generadas y versionadas, el CI comprueba que están al día— y se
  mantiene por esa razón, no por la de Flathub.
- El [#37](https://github.com/sgomez/rfirma/issues/37) preguntaba cómo entran los `.so` en una
  construcción apta para Flathub. Con la tienda fuera de v0.1, **la pregunta no llega a
  importar**: la medición se conserva por si algún día se retoma.
- **Flathub no queda cerrado para siempre**, solo fuera de v0.1. Volver es un esfuerzo nuevo
  —vendorizar el árbol Maven, y lo que sus reglas digan cuando toque—, no la continuación de
  este.
