# El Almacén de rFirma: una base NSS cifrada con un PIN que solo guarda el llavero del escritorio

Instalar un certificado personal es llevarlo al almacén personal de la plataforma. En
Windows es el almacén personal de la persona usuaria; en macOS, el llavero de inicio de
sesión. Windows y macOS no tienen implementación: la regla de esta sección es la que
seguirán cuando tengan canal de distribución propio (ADR-0015).

En Linux, el destino es el **Almacén de rFirma**: una única base NSS, propia de la
aplicación, cifrada con un PIN aleatorio. El PIN lo genera rFirma la primera vez que hace
falta y lo guarda en el llavero del escritorio; la persona no lo ve ni lo teclea nunca.
Sustituye al almacén por fichero actual —un directorio con su propia base **sin PIN** por
cada `.p12` instalado, descrito en `docs/research/p12-en-almacen-nss.md`—, que se retira
**sin migración**.

## De qué protege, y de qué no

Protege el certificado y su clave privada **en reposo**: una copia de seguridad del
directorio de datos de la aplicación, un cliente de sincronización que se la lleva sin
querer, o el disco montado en otra máquina. Sin el PIN, esa base es una NSS cifrada con una
contraseña que nadie escribió en ningún sitio.

No protege frente a código que ya corre en la sesión de la persona: igual que el
ADR-0005 declara fuera de su modelo de amenaza al atacante que ya ejecuta como la persona,
aquí tampoco se persigue ese caso. Un proceso así puede pedir el secreto al llavero por el
mismo camino que rFirma, o esperar a que rFirma abra la base y leer la clave en memoria. El
cifrado no es una barrera contra ese atacante: es una barrera contra la copia del fichero
que sale de la sesión.

## Por qué la CA local no se cifra y un certificado personal sí

El ADR-0005 deja la clave privada de la CA local sin cifrar, en un fichero `0600`, y
descarta el llavero del escritorio para ella. Esta decisión no lo contradice: son dos
activos distintos.

La CA local está **restringida a la máquina** —`nameConstraints` la limita a `localhost` y
`127.0.0.1`— y no identifica a nadie. Sacarla de la máquina no le da a quien la robe nada
que no tuviera ya: no sirve para firmar nada ante nadie, y el ADR-0005 ya señala que el
mismo atacante puede plantar su propia raíz en el `nssdb` sin necesidad de robar la
nuestra. Cifrarla no compra protección real, solo la complejidad del llavero.

Un certificado personal **identifica a la persona y firma en su nombre**, sin restricción
de máquina ni de sede. Una copia de su clave privada fuera de la sesión —una copia de
seguridad, un disco ajeno— sí es un activo que vale la pena llevarse, y por eso sí compra
protección: cifrarla con un PIN que no vive en ningún fichero hace que esa copia, sola, no
sirva para nada.

## El llavero se toca solo al instalar y al firmar

Los certificados de una base NSS se leen sin PIN: listar, arrancar, `selectcert` o volver
a buscar no piden nada al llavero. Es la misma idea que el ADR-0025 aplica al token
PKCS#11: no se abre sesión hasta que hace falta. El PIN solo se pide al llavero en dos
momentos: cuando se instala un certificado nuevo (para escribir en la base, o para crearla
si es la primera vez) y cuando se firma (para abrir la sesión NSS que la firma necesita).

## El camino al llavero: el portal de secretos, con `oo7`

rFirma llega al llavero del escritorio por el portal de secretos
(`org.freedesktop.portal.Secret`), con el crate `oo7`. Dentro del flatpak no hace falta
ningún permiso nuevo en el manifiesto: el portal le da a la aplicación su propio secreto,
aislado del de cualquier otra. Fuera del flatpak —también en los paquetes deb y rpm, que
son canal propio (ADR-0015)— no hay sandbox que el portal pueda mediar, y `oo7` habla
directamente con Secret Service por D-Bus: ese proceso ve el llavero completo, igual que
cualquier otro que corra como la persona. No cambia el modelo de amenaza del ADR-0005 —ese
atacante ya queda fuera de él—, pero el aislamiento de esta sección solo existe dentro del
flatpak.

Se descarta que el flatpak pida acceso a todo el llavero de la persona, por ejemplo con un
`talk-name` a `org.freedesktop.secrets` en el manifiesto (ver *Considered Options*): dentro
del sandbox, el portal ya aísla el secreto de rFirma del resto del llavero sin pedir nada
más, que es justo lo que un cliente de firma necesita.

**Estado de KDE, sin comprobar a mano todavía.** KWallet implementa
`org.freedesktop.impl.portal.Secret` desde KDE Frameworks 6.2 (fusionado el 21 de abril de
2024, [MR !67 de `kwallet`](https://invent.kde.org/frameworks/kwallet/-/merge_requests/67),
resuelto como [bug 466197](https://bugs.kde.org/show_bug.cgi?id=466197)): antes de esa
versión, `xdg-desktop-portal-kde` no tenía backend propio para el portal de secretos, y
algunos entornos lo resolvían delegando en `gnome-keyring` por configuración
(`/etc/xdg/xdg-desktop-portal/kde-portals.conf`). Con Frameworks 6.2 o posterior, el
backend es KWallet mismo y no hace falta ese rodeo. Lo que queda por comprobar a mano —otro
ticket— es que `oo7` obtiene de verdad un secreto a través de ese backend en una sesión KDE
real, dentro y fuera del flatpak.

## Sin llavero, no hay instalación

Si no hay portal de secretos ni Secret Service, o la persona se niega a desbloquearlo,
rFirma explica que no puede guardar el certificado protegido y no instala nada. No hay
contraseña maestra que la persona teclee como alternativa, no se escribe el PIN ni la clave
en claro en ningún sitio, y el `.p12` original no se usa para firmar sin haber pasado por
la instalación.

## Si el llavero pierde el PIN

Puede existir el Almacén de rFirma —la base NSS en disco— sin que el llavero tenga ya su
PIN, o sin que lo abra: el llavero se vació, se cambió de máquina copiando solo la base, o
similar. rFirma lo detecta, se lo dice a la persona y le ofrece vaciar el almacén para
volver a instalar desde cero. Nunca lo vacía por su cuenta: sin el PIN no hay forma de
saber si la base todavía sirve para algo, y borrarla sin que la persona lo pida sería
destruir su único certificado instalado sin haberlo consentido.

## Considered Options

- **Seguir con un almacén por fichero, sin cifrar, uno por `.p12`.** Descartada: dejar la
  clave privada de un certificado personal en claro en disco no protege nada en reposo, y es
  precisamente lo que este ADR corrige.
- **Contraseña maestra que teclea la persona.** Descartada: es la misma clase de secreto
  que el PIN, pero además una contraseña más que la persona tiene que recordar, y AutoFirma
  ya demuestra el resultado cuando ese hábito se traslada a plantillas de configuración
  (`RestoreConfigLinux.java` con `KS_PASSWORD` fijo en el fuente). rFirma no añade una
  contraseña que gestionar cuando el llavero del sistema ya resuelve el mismo problema sin
  pedírsela a nadie.
- **Pedir en el flatpak un `talk-name` a `org.freedesktop.secrets`, sin pasar por el
  portal.** Descartada: el llavero completo guarda secretos de otras aplicaciones
  —contraseñas de red, tokens de otras cuentas—, y rFirma solo necesita uno propio. Dentro
  del sandbox, el portal ya ofrece ese secreto aislado sin pedir nada más.
- **Usar el `.p12` directamente para firmar, sin instalarlo.** Descartada: rompe la regla
  del ID-423 —instalar es llevar el certificado al almacén de la plataforma— y deja sin
  protección en reposo exactamente la clave que este ADR protege.

## Consequences

- El almacén por fichero actual y su base de datos sin PIN se retiran sin migración: quien
  tenga un `.p12` instalado hoy lo vuelve a instalar en el Almacén de rFirma.
- `selectcert`, el arranque y cualquier operación que no firme ni instale siguen sin tocar
  el llavero ni pedir nada a la persona.
- Sin portal de secretos ni Secret Service disponibles, Linux se queda sin instalación de
  certificados personales hasta que la persona resuelva su entorno de escritorio; no hay
  camino alternativo que la evite.
- Windows y macOS quedan sin implementar: la regla de esta sección es la que seguirán
  cuando tengan canal de distribución (ADR-0015).
