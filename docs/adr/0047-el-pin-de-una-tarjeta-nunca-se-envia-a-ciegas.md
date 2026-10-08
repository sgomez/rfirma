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
   volver a teclearlo. No hay reintento automático en ninguna capa, con una
   sola excepción, la regla 8.
6. **En un ciclo o un lote el PIN se pide una vez, y el primer fallo corta
   todo.** El secreto aceptado sirve para todas las prefirmas del ciclo.
   Cualquier fallo de una de ellas —PIN incorrecto, PIN bloqueado, tarjeta
   retirada— corta el ciclo sin ningún `C_Login` más, y el lote entero se da
   por fallido.
7. **Login por operación cuando la clave lo exige.** Si la clave privada
   declara `CKA_ALWAYS_AUTHENTICATE`, cada firma hace su login de contexto
   específico con el secreto ya aceptado del ciclo, sin volver a pedirlo. Ese
   login cumple las reglas 5 y 6. El DNIe por OpenSC no lo declara: OpenSC
   guarda el PIN y lo vuelve a verificar antes de cada firma.
8. **La interferencia de otro programa no es un rechazo.** `CKR_USER_NOT_LOGGED_IN`
   en `C_Login` pasa cuando otro programa abre su canal con la tarjeta entre la
   apertura de la sesión y el login, y está medido que la tarjeta no llega a
   ver el PIN. El adaptador abre otra sesión y reenvía el mismo secreto **una
   sola vez**; si vuelve a fallar, el fallo sube como cualquier otro.
9. **El intento que bloquea se reconoce.** Tras un `CKR_PIN_INCORRECT` el
   adaptador relee las banderas de la ranura en el mismo proceso; con
   `CKF_USER_PIN_LOCKED` el caso de uso termina como PIN bloqueado (regla 2) y
   no vuelve a pedir el PIN.

## Por qué no hay contador exacto de intentos

PKCS#11 no tiene contador de reintentos: solo las tres banderas de la regla 3 y
la de bloqueo, y cada módulo enciende las que quiere. Con el DNIe por OpenSC
ninguna existe antes de un fallo, y tras él solo en el proceso que falló
(`docs/research/dnie-en-linux.md`): el primer PIN de cada trámite se pide
siempre sin señales, y el bloqueo no se recuerda entre trámites. Un número de
intentos inventado a partir de lo que rFirma ha visto sería falso en cuanto la
tarjeta se usara con otro programa, así que no se da.

## Por qué el DNIe real no entra en ninguna grada automática

Cada prueba con un PIN erróneo gasta intentos de una tarjeta personal, y un
DNIe no puede vivir en el CI: no hay emulador, porque el chip lleva un sistema
operativo propietario, y SoftHSM ni se bloquea ni da las señales del DNIe. Las
reglas se prueban en la grada B (ADR-0014) contra un módulo PKCS#11 falso del
workspace, `fake-pkcs11`, que se porta como el DNIe medido y tiene otro perfil
con las tres señales; entra por la misma costura que SoftHSM y nunca llega a un
paquete (ADR-0004). El DNIe real solo se usa a mano, en la suite de
conformidad, y solo en el camino feliz: nunca se teclea un PIN erróneo a
propósito.

## Consequences

- Una tarjeta que declara el PIN bloqueado nunca recibe un `C_Login` de rFirma,
  ni al pedir el PIN ni justo antes de enviarlo.
- El DNIe por OpenSC no se beneficia de la regla 1 en el primer intento de un
  trámite: no da la señal hasta fallar. Lo cubren las reglas 3 y 9.
- Lo que el módulo falso imita es lo medido, no la tarjeta: si OpenSC o el DNIe
  cambian de conducta, el falso miente hasta que se vuelva a medir.

## Considered Options

- **Leer el contador del DNIe por APDU, como jmulticard.** Descartada por
  ahora: saltarse PKCS#11 ata rFirma a una tarjeta y a un lector, y queda fuera
  de este ADR.
- **Probar con una tarjeta simulada (jCardSim con IsoApplet, u opcard con
  `vpicc`).** Descartada: probaría la cadena OpenSC → lector, pero ninguna
  imita el DNIe entero, y el módulo falso ya basta para las reglas.
