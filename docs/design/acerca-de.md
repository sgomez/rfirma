# Acerca de rFirma

Identidad de la aplicación, **estado de la versión**, aviso de independencia
respecto del cliente oficial y licencias. Se abre desde el menú de la
[cabecera](cabecera.md) y desde la acción de la franja de notificación de la
[ventana principal](ventana-principal.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido.
- Actualizar rFirma — es la única pantalla que dice si hay versión nueva.

## Estructura

`.rf-dialog` de 460 px sobre `.rf-scrim`, en cinco bloques:

1. **Identidad**: `rFirma` en `.rf-heading` y la versión debajo.
2. **Qué hace**, en una frase: firmar documentos PDF con tu certificado digital.
3. **Estado de la versión**, tras un `.rf-divider`: una sola línea con su icono.
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

## Estado de la versión

**Una línea, y nada más.** Quien tiene el diálogo delante **ya ha instalado
rFirma**, así que las órdenes de alta del repositorio —que estuvieron aquí
hasta la v0.4— no le dicen nada que le sirva para actualizar: son las de
instalar lo que ya tiene. Quien se dio de alta actualiza con su gestor de
paquetes sin volver a esta pantalla, y quien no, encuentra esas órdenes donde
se instala, en `packaging/repo/index.html`.

Con ellas se fueron el selector de tres canales, el bloque de órdenes, su botón
«Copiar» y las tres cadenas del catálogo (`about.update.channel.*`).

**No hay botón de descarga, y no hay ningún enlace que se pulse**:
`opener:deny-open-url` sigue denegado (ID-85).

## El aviso de independencia

> **Proyecto independiente.** rFirma no está relacionada con AutoFirma ni con
> la Administración General del Estado, que publican el cliente oficial, ni
> cuenta con su respaldo. Si necesitas la aplicación oficial, descárgala de su
> web. Los problemas con rFirma se comunican en su propio repositorio, no a
> quienes publican el cliente oficial.

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

- Diálogo de **460 px**, con `--rf-space-sm` de relleno y 12 px entre bloques.
  Fueron 520 mientras el bloque de órdenes estuvo dentro, porque las del `.deb`
  envolvían a cuatro renglones.
- El nombre va en `.rf-heading` **bajado a 32 px** —los 48 plenos llenan medio
  diálogo— y, 4 px debajo, la versión en `.rf-body rf-text-muted`.
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

**Dos, y se diferencian sólo en esa línea:**

- **Hay una versión nueva**: flecha hacia arriba y «Hay una versión nueva:
  0.4.1».
- **Al día**: marca de verificación atenuada y «Estás en la última versión».

Sin red **no hay tercer estado**: se calla. Un «no se ha podido comprobar» sería
un fallo que no le pide nada a nadie.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-heading`, `.rf-prose`, `.rf-body`,
`.rf-text-muted`, `.rf-divider`, `.rf-btn--ghost|--primary`.

## Decisiones

El aviso llevó primero icono de aviso y borde; se retiró por lo dicho arriba.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Acerca de · desde el menú», con la palanca
**Estado de la versión**; la de **Canal** ya no se usa.

El canal propio y sus tres repositorios están en el
[ADR-0015](../adr/0015-canal-de-distribucion-propio.md); las órdenes de alta,
en `packaging/repo/index.html`.
