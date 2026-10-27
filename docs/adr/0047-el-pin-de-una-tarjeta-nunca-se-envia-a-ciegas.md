# El PIN de una tarjeta nunca se envía a ciegas

Un DNIe se bloquea al tercer PIN erróneo, y desbloquearlo exige ir a un punto
de actualización del DNIe. Cada `C_Login` con un PIN equivocado gasta un
intento que la persona no recupera desde casa, así que el código que habla con
una tarjeta cumple estas reglas, y su incumplimiento no se ve hasta que una
tarjeta de verdad queda bloqueada:

1. **Antes de pedir el PIN y antes de enviarlo, rFirma lee la información de
   token de la ranura del certificado.** Con `CKF_USER_PIN_LOCKED` no hay
   `C_Login` ni diálogo: el caso de uso termina con la situación de PIN
   bloqueado (`pinLocked`).
2. **El PIN bloqueado se explica.** El mensaje dice que con esa tarjeta no se
   puede firmar hasta desbloquearla y menciona los puntos de actualización del
   DNIe; no habla de un PUK, que el DNIe no tiene. La sede recibe el código de
   almacén bloqueado, `SAF_52`.
3. **`CKF_USER_PIN_FINAL_TRY` es un aviso en el diálogo, antes de teclear.** La
   petición de secreto lleva el estado del PIN: con `FINAL_TRY` el diálogo
   avisa de que un PIN incorrecto bloquea la tarjeta, y con
   `CKF_USER_PIN_COUNT_LOW` avisa, más suave, de que ya ha habido algún intento
   fallido. Las señales se vuelven a leer antes de cada diálogo, también tras
   un fallo.
4. **`attempts_left` dice solo lo que se sabe:** 0 con el PIN bloqueado, 1 con
   `FINAL_TRY` y desconocido en cualquier otro caso.
5. **Un secreto rechazado no se reenvía nunca.** Un `C_Login` fallido devuelve
   el control a la persona, el secreto se descarta y cada intento nuevo exige
   volver a teclearlo. No hay reintento automático en ninguna capa.
6. **En un ciclo o un lote el PIN se pide una vez, y el primer fallo corta
   todo.** El secreto aceptado sirve para todas las prefirmas del ciclo.
   Cualquier fallo de una de ellas —PIN incorrecto, PIN bloqueado, tarjeta
   retirada— corta el ciclo sin ningún `C_Login` más, y el lote entero se da
   por fallido.
7. **Login por operación cuando la clave lo exige.** Si la clave privada
   declara `CKA_ALWAYS_AUTHENTICATE`, cada firma hace su login de contexto
   específico con el secreto ya aceptado del ciclo, sin volver a pedirlo. Ese
   login cumple las reglas 5 y 6.
8. **El intento que bloquea se reconoce.** Tras un `CKR_PIN_INCORRECT` el
   adaptador relee las banderas de la ranura en el mismo proceso; con
   `CKF_USER_PIN_LOCKED` el caso de uso termina como PIN bloqueado (regla 2) y
   no vuelve a pedir el PIN.

## En el Almacén de Windows, las reglas del PIN son las de Windows

Las reglas de arriba son las del camino PKCS#11. En el Almacén de Windows, tarjetas incluidas,
el PIN lo pide Windows con su diálogo (ADR-0035) y los intentos que quedan y el bloqueo son cosa
del minidriver: rFirma no lee ni el contador ni el estado de bloqueo, ni tiene diálogo propio
del PIN. Traduce el error que devuelve Windows: PIN incorrecto, tarjeta bloqueada o diálogo
cancelado por la persona.

## El DNIe por rdnie da los intentos restantes exactos

En Linux y macOS el DNIe 3.0 y 4.0 no va por PKCS#11 sino por rdnie (ADR-0048), que lee los
intentos restantes con un VERIFY sin PIN, sin gastar ninguno. Ahí las reglas 1,
3 y 4 se cumplen con el número exacto: con cero no hay diálogo y el caso de uso
termina como PIN bloqueado; con más, el diálogo dice siempre cuántos quedan, y
resalta el aviso con uno o dos. rdnie vuelve a leerlos justo antes de enviar el
PIN y no lo envía si han cambiado: eso corta el ciclo como cualquier otro fallo
(regla 6). Las reglas 5 y 6 valen igual.

## Por qué PKCS#11 no da un número exacto

PKCS#11 no tiene contador de reintentos: solo las tres banderas de la regla 3 y
la de bloqueo, y cada módulo enciende las que quiere. OpenSC no enciende ninguna
con el DNIe antes de un fallo, y tras él solo en el proceso que falló
(`docs/research/dnie-en-linux.md`): el primer PIN de cada trámite se pediría
sin señales. Por eso el DNIe va por rdnie y no por OpenSC. Un número de
intentos inventado a partir de lo que rFirma ha visto sería falso en cuanto la
tarjeta se usara con otro programa, así que no se da.

## Por qué el DNIe real no entra en ninguna grada automática

Cada prueba con un PIN erróneo gasta intentos de una tarjeta personal. El camino
PKCS#11 se prueba en la grada B (ADR-0014) contra un módulo falso del workspace,
`fake-pkcs11`: una tarjeta con el aspecto del DNIe que da las tres señales del
PIN y exige el login por firma. Sirve para las reglas de arriba y para comprobar
que la ranura de OpenSC del lector que lee rdnie no se abre. Entra por la misma
costura que SoftHSM y nunca llega a un paquete (ADR-0004). El camino rdnie se prueba contra `rdnie-emulator`, que
habla el protocolo del DNIe con un titular ficticio y solo es dependencia de
pruebas. El DNIe real solo se usa a mano, en la suite de conformidad, y solo en
el camino feliz: nunca se teclea un PIN erróneo a propósito.

## Consequences

- Una tarjeta que declara el PIN bloqueado nunca recibe un `C_Login` de rFirma,
  ni al pedir el PIN ni justo antes de enviarlo.
- Las reglas del camino PKCS#11 se prueban contra señales que el módulo falso
  da siempre. Un módulo real que solo las enciende tras un fallo queda cubierto
  por la relectura de la regla 8, pero ninguna prueba lo imita.
- Sin ningún DNIe por PKCS#11 (ADR-0048) no hay reintento ante
  `CKR_USER_NOT_LOGGED_IN`: solo pasaba cuando otro programa abría su canal
  seguro con el DNIe por OpenSC, y sube como cualquier otro fallo de `C_Login`.

## Considered Options

- **Probar con una tarjeta simulada (jCardSim con IsoApplet, u opcard con
  `vpicc`).** Descartada: probaría la cadena OpenSC → lector, pero ninguna
  imita el DNIe entero, y el módulo falso ya basta para las reglas.
