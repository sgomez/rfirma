# Acerca de rFirma

Identidad de la aplicación, aviso de independencia respecto del cliente oficial
y licencias. Se abre desde el menú de la [cabecera](cabecera.md).

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido.

## Estructura

`.rf-dialog` de 460 px sobre `.rf-scrim`, en cuatro bloques:

1. **Identidad**: `rFirma` en `.rf-heading` y la versión debajo.
2. **Qué hace**, en una frase, incluida la garantía que de verdad importa: el
   documento y la clave privada no salen del ordenador.
3. **Aviso de independencia**, como párrafo normal.
4. **Licencias**, y los botones «Ver las licencias» (`--ghost`) y «Cerrar»
   (`--primary`).

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

## Estados

Uno.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-heading`, `.rf-prose`, `.rf-body`,
`.rf-text-muted`, `.rf-divider`, `.rf-btn--ghost|--primary`.

## Decisiones

El aviso llevó primero icono de aviso y borde; se retiró por lo dicho arriba.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboard «Acerca de · desde el menú».
