# Acerca de rFirma

Identidad de la aplicación, **cómo actualizar**, aviso de independencia respecto
del cliente oficial y licencias. Se abre desde el menú de la
[cabecera](cabecera.md) y desde la acción de la franja de notificación de la
[ventana principal](ventana-principal.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido.
- Actualizar rFirma — es la única pantalla que lo cuenta.

## Estructura

`.rf-dialog` de 520 px sobre `.rf-scrim`, en cinco bloques:

1. **Identidad**: `rFirma` en `.rf-heading` y la versión debajo.
2. **Qué hace**, en una frase: firmar documentos PDF con tu certificado digital.
3. **Cómo actualizar**, tras un `.rf-divider`: la línea de estado de versión, el
   selector de canal y el bloque de órdenes con «Copiar».
4. **Aviso de independencia**, como párrafo normal.
5. **Licencias**, y los botones «Ver las licencias» (`--ghost`) y «Cerrar»
   (`--primary`).

**La frase de qué hace se ha recortado dos veces.** Decía «con tu certificado
**o tu tarjeta criptográfica**», que deja de ser cierto en la v0.4 (ID-201 a
ID-204); y decía «el documento y la clave privada no salen de tu ordenador»,
que se retira porque tranquiliza sobre lo evidente y no cambia nada de lo que la
persona puede hacer. Lo que la garantía significa de verdad —que la clave
privada nunca cruza a Java— está donde tiene que estar, en el
[ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md).

## Cómo actualizar

**No hay botón de descarga, y no hay ningún enlace que se pulse.** Lo que se
enseña son **las órdenes de alta del repositorio** (ID-181). Es lo que hace que
el mecanismo **se autoliquide**: quien da de alta el repositorio deja de
necesitar avisos, porque a partir de ahí actualiza su gestor de paquetes. Y no
hay enlace porque `opener:deny-open-url` sigue denegado —el ID-85 no se toca—,
así que la URL sólo aparece **dentro** de una orden copiable.

**Tres canales sin que esto sea un manual**: un selector de tres pestañas
—*Flatpak*, *Debian y Ubuntu*, *Fedora y openSUSE*— y **un solo bloque de
órdenes a la vista**, con su botón «Copiar».

**El bloque está en los dos estados de versión.** Lo único que cambia entre
ellos es la línea de arriba. Con eso los dos estados miden lo mismo por
construcción, sin reservar ningún hueco vacío, y *Acerca de* sigue enseñando
cómo darse de alta a quien entra **sin** tener versión nueva — que es justo la
población que el mecanismo quiere reducir.

## El aviso de independencia

> **Proyecto independiente.** rFirma no está relacionada con AutoFirma ni con
> la Administración General del Estado, que publican el cliente oficial, ni
> cuenta con su respaldo. Si necesitas la aplicación oficial, descárgala de su
> web.

Va **como párrafo, sin icono ni recuadro**. Es un hecho sobre el proyecto, no
una advertencia sobre un riesgo del usuario, y enmarcarlo como alarma le daría
un peso que no le corresponde. Pero tiene que estar: una aplicación que firma
ante la Administración con la misma criptografía que la oficial se puede
confundir con ella, y esa confusión hay que deshacerla en el sitio donde la
gente va a preguntar qué es esto.

## Licencias

- **rFirma**: EUPL-1.2.
- **Bibliotecas de Cliente @firma**: GPL-2.0+ / EUPL-1.1.

Ver [ADR-0008](../adr/0008-licencia-eupl-1-2.md) para por qué esa combinación
se sostiene.

### Geometría

- Diálogo de **520 px**, con `--rf-space-sm` de relleno y 12 px entre bloques.
  Eran 460 hasta la v0.3: con el bloque de «cómo actualizar» dentro, las órdenes
  del `.deb` envolvían a cuatro renglones y el diálogo se estiraba hacia abajo,
  que es la dirección en la que menos sitio hay.
- El nombre va en `.rf-heading` **bajado a 28 px** —eran 32, y los 48 plenos
  llenan medio diálogo— y, 4 px debajo, la versión en `.rf-body rf-text-muted`.
- **El bloque de órdenes lleva `min-height` de 98 px**, que es lo que ocupan las
  cuatro líneas del `.deb`, la más alta de las tres. Las órdenes van en
  monoespaciada a 11,5 px con interlineado 1,7, envolviendo con
  `overflow-wrap: anywhere`, dentro de un recuadro con borde
  `--rf-border-strong` sobre `--rf-surface`.
- **El selector de canal** es una tira de tres pastillas de 6×12 px dentro de un
  carril con borde `--rf-border-subtle`; la activa va sobre `--rf-primary` con
  el texto en `--rf-on-primary`, las otras dos en `--rf-text-muted` sin fondo.
- Los dos párrafos, en `.rf-prose`; el aviso de independencia sin borde, fondo
  ni icono, como cualquier otro.
- El bloque de licencias es una pila de 6 px: las dos líneas en
  `.rf-body rf-text-muted` y la dirección del repositorio en `.rf-body`.
- Acciones abajo a la derecha: «Ver las licencias» fantasma y «Cerrar»
  primario.

El artboard dibuja las dos líneas de licencia **desplegadas**; eso es el estado
congelado del canvas y aquí es lo que revela «Ver las licencias». Lo que se ve
siempre es la dirección del repositorio, que es adónde va quien quiera
comprobar cualquiera de las dos.

## Estados

**Dos, y sólo se diferencian en una línea:**

- **Hay una versión nueva**: flecha hacia arriba y «Hay una versión nueva:
  0.4.1».
- **Al día**: marca de verificación atenuada y «Estás en la última versión».

El resto del diálogo, bloque de órdenes incluido, es idéntico. Que el estado no
cambie el tamaño no es una casualidad de maqueta: es la razón por la que el
bloque está en los dos.

Sin red **no hay tercer estado**: se calla. Un «no se ha podido comprobar» sería
un fallo que no le pide nada a nadie.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-heading`, `.rf-prose`, `.rf-body`,
`.rf-text-muted`, `.rf-divider`, `.rf-btn--ghost|--primary`,
`--rf-border-strong`, `--rf-surface`, `--rf-primary`, `--rf-on-primary`.

## Decisiones

El aviso llevó primero icono de aviso y borde; se retiró por lo dicho arriba.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Acerca de · desde el menú», con las palancas
**Estado de la versión** y **Canal**.

El bloque de «cómo actualizar» se decidió en el
[#250](https://github.com/sgomez/rfirma/issues/250) (ID-177, ID-181). El canal
propio y sus tres repositorios están en el
[ADR-0015](../adr/0015-canal-de-distribucion-propio.md).
