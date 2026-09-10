# Diálogo de secreto del almacén

Pide el secreto que abre la sesión del almacén de certificados: el **PIN** de un
módulo PKCS#11 o la **contraseña** de un fichero. Es el único momento del
recorrido en que el usuario aporta el secreto que desbloquea la clave privada.

La ficha se sigue llamando `dialogo-pin.md` porque es la que enlazan las demás,
pero el diálogo no es «el del PIN»: la palabra la elige el almacén.

## Casos de uso que la usan

- Firmar un PDF en local — al abrir la sesión del almacén, que según el almacén
  cae antes de listar los certificados o justo antes de firmar.

## Cuándo aparece

**Cuando el almacén necesita sesión, y sólo entonces.** Sin necesidad de sesión
**no hay diálogo** y se firma directo (ID-190). Hay tres situaciones, y no
comparten momento:

| Almacén | Cuándo se pide | Botón |
| ------- | -------------- | ----- |
| Módulo PKCS#11 | **antes de listar**, en «Cargando certificados» | `Continuar` |
| Perfil de navegador con contraseña maestra | **antes de listar**, ídem | `Continuar` |
| `.p12` instalado en rFirma | **al firmar** | `Firmar` |

Los dos primeros abren sesión para poder **enumerar**, así que el diálogo puede
aparecer **antes de que exista lista de certificados**: se dibuja sobre el
estado «Cargando certificados», sin lista detrás. El tercero es el contrario: un
almacén NSS de un solo fichero lista sus certificados sin secreto —sólo las
claves privadas lo exigen (ID-195)—, así que ahí el secreto llega al final,
cuando ya se sabe con qué se firma.

Esto **enmienda** lo que decía esta ficha hasta la v0.3: «después de la
prefirma, nunca antes». Era cierto para el único almacén que se contemplaba
entonces. La secuencia criptográfica no cambia —ver
[ventana-principal.md](ventana-principal.md)—; lo que cambia es que abrir la
sesión no forma parte de ella y cae donde el almacén lo pida.

## El diálogo es una ventana nativa, no una capa de la aplicación

**No lo dibuja la ventana: lo dibuja el sistema.** El secreto no cruza a
WebKit ni al canal IPC de Tauri (ADR-0001), así que el diálogo es una ventana
modal GTK del backend, transitoria de la ventana principal. Lo que sigue
describe esa ventana; los nombres de clase de la versión web —`.rf-dialog`,
`.rf-scrim`, `.rf-field`— ya no aplican, y el componente que los usaba se
retiró.

Hereda el tema del escritorio, y **no puede parecerse al diálogo de
`pinentry-gnome3`**: ese lo pinta GNOME Shell desde el compositor y ningún
cliente GTK lo replica (`docs/research/pinentry-gtk-temas-empaquetado.md`).

## Estructura

1. Título en la barra del gestor de ventanas: «Introduce el PIN» o «Introduce
   la contraseña».
2. Icono de credencial del tema y, a su lado, **qué se está abriendo**, en los
   términos de quien firma: el titular en negrita y, debajo y atenuado, su
   número de documento. **Es el mismo dato que enseña el desplegable**, leído
   del DER del firmante y nunca de la etiqueta del objeto en el token. Si el DER
   no da titular, la línea **no se pone** y en su lugar va el título.
3. Campo enmascarado, que nace con el foco.
4. **Nada debajo del campo** salvo el mensaje de fallo, cuando lo hay.
5. Abajo a la derecha, «Cancelar» y el primario, marcado como acción sugerida
   del tema.

### La botonera va al pie, no a una barra de cabecera

GNOME pone los botones de sus diálogos en la barra de cabecera, y se probó así.
**No se hace**, por dos razones: fuera de GNOME —KDE, XFCE— una barra de
cabecera CSD se lee como una ventana ajena al escritorio, y el aspecto que se
ganaría a cambio tampoco es el del diálogo de sistema, que lo pinta GNOME Shell
y ningún cliente GTK replica. Al pie es lo que menos desentona en los cuatro
escritorios, y es además lo que esta ficha pedía desde la v0.3.

### El campo del secreto nace con el foco

**Al abrirse el diálogo se puede teclear sin tocar nada**, y el foco se dibuja
con el anillo del tema.

El argumento es que el diálogo tiene **una sola entrada** y en los tres almacenes
—PIN de módulo PKCS#11, contraseña de perfil de Firefox, contraseña de `.p12`—
el gesto siguiente es siempre teclear. Aparece igual cuando el diálogo lo abre la
[ventana de sede](ventana-de-sede.md), porque ahí es exactamente el mismo
diálogo.

### La palabra la elige la clase de almacén

**«PIN» para un módulo PKCS#11; «contraseña» para un fichero** —un perfil NSS de
navegador, un `.p12` instalado— (ID-188). Se diverge a propósito de AutoFirma,
que le dice «contraseña» al módulo genérico porque tiene un registro de tarjetas
que rFirma no tendrá.

**No se discrimina por hardware**, y está medido: los indicadores de ranura
extraíble de PKCS#11 valen `false` tanto en SoftHSM como en NSS, así que
preguntarle al módulo si es una tarjeta no responde nada. La palabra la decide
la **clase de almacén**, que sí se sabe: un módulo sin perfil NSS detrás es un
módulo y pide PIN; un perfil de Firefox o de Chromium, un `nssdb` suelto o un
`.p12` instalado son ficheros y piden contraseña.

### Lo que el diálogo no nombra

Ni la clase de módulo criptográfico ni la etiqueta del token. «Módulo PKCS#11 ·
SoftHSM (rfirma-test)» era vocabulario de implementación y, encima, el nombre de
un token de pruebas. Lo que se nombra es la cosa de la persona, o no se nombra
nada.

### Geometría

Ancho de 400 px y alto el que pida el contenido; los márgenes y el espaciado
son los del tema, no los de rFirma. **No se reservan huecos** para la línea de
titular ni para el mensaje de fallo: la ventana se levanta de nuevo en cada
intento, así que no hay salto que evitar.

## Estados

- **Pidiendo el secreto**: campo vacío y **nada bajo él**. Ni pista, ni promesa,
  ni instrucciones de uso.
- **Secreto incorrecto**: el campo toma la clase `error` del tema —el borde que
  el escritorio use para eso, sin glifo— y bajo el campo, en negrita, «PIN
  incorrecto» o «Contraseña incorrecta». Nada más.

**No hay contador de reintentos, y no es un hueco por rellenar: es estructural**
(ID-191). Los intentos restantes los cuenta el DNIe, que está fuera del alcance
de la v0.4; un almacén local no los tiene, y la información de token de PKCS#11
tampoco los trae. Hasta la v0.3 esta ficha prometía «te quedan **2 intentos**
antes de que la tarjeta se bloquee» y además argumentaba que había que
enseñarlos; el argumento era bueno y el dato no existe, así que se retiran los
dos. No se sustituye por ninguna promesa parecida —«puede que se bloquee», «ten
cuidado»— porque avisar de un límite que no se sabe contar es peor que callar.

Tampoco cruza: `SecretView` no lleva contador, y el ciclo reintenta hasta que se
acierta o se cancela.

**Tampoco hay pistas.** La que había —«El PIN se usa solo para esta firma y no
se guarda en ningún sitio»— tranquilizaba sobre lo evidente; la que se llegó a
escribir para el `.p12` —que la contraseña se teclea en cada firma— narraba el
mecanismo. Ninguna de las dos cambia lo que la persona puede hacer en esta
pantalla, así que ninguna se queda.

## Componentes

`gtk::Dialog` con su área de acciones al pie, `gtk::Image` con
`dialog-password`, `gtk::Entry` enmascarado y las clases del tema `dim-label`,
`suggested-action` y `error`.

**Ningún color en duro.** El fallo se señala con **borde y peso** —como manda
[design-system.md](design-system.md), y sin glifo antepuesto, que se retiró en
la v0.4—, y quien pone el borde es el tema del escritorio.

## Lo que este diálogo deja abierto

Los demás fallos de PKCS#11 —token ausente, sesión caducada, módulo no
encontrado— no están dibujados. Están pendientes en el mapa. **Tarjeta
bloqueada ya no está en esa lista**: la v0.4 retira tarjetas y DNIe del alcance
y del dibujo (ID-201 a ID-204).

## Decisiones

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «6 · Pidiendo PIN» y «7 · PIN incorrecto», con
la palanca **Clase de almacén** que recorre las tres situaciones. El diálogo
aparece además, copiado literal en su posición de perfil de Firefox, sobre
«3 · Cargando certificados»: dos pantallas contando lo mismo con dos textos
distintos serían dos verdades.

Decidido en el [#250](https://github.com/sgomez/rfirma/issues/250) (ID-188,
ID-190, ID-191, ID-195).

El **`autofocus` del campo del secreto** se añadió al validar la
[ventana de sede](ventana-de-sede.md) el 05/09/2026
([#317](https://github.com/sgomez/rfirma/issues/317)), sobre el mismo artboard
«6 · Pidiendo PIN». Se descartó darle al diálogo de sede una palanca de contexto
propia: la pantalla es idéntica a la del recorrido local, y con ella se fue la
frase que explicaba que cancelar cancela.
