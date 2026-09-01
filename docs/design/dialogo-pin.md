# Diálogo de PIN

Pide el PIN de la tarjeta criptográfica. Es el único momento del recorrido en
que el usuario aporta el secreto que desbloquea la clave privada.

## Casos de uso que la usan

- Firmar un PDF en local — entre la prefirma y la firma.

## Cuándo aparece

Después de pulsar «Firmar documento» y **después de la prefirma**, nunca antes:
no tiene sentido pedir el PIN sin saber todavía qué se va a firmar. Ver la
secuencia en [ventana-principal.md](ventana-principal.md).

## Estructura

`.rf-dialog` de 420 px sobre `.rf-scrim`, con la ventana atenuada detrás:

1. Título «Introduce el PIN de la tarjeta».
2. Subtítulo con el titular del certificado y su DNI, para que se vea con qué
   identidad se está firmando.
3. Campo PIN, enmascarado, con tracking amplio.
4. Pista.
5. Divisor y, abajo a la derecha, «Cancelar» (`--ghost`) y «Firmar»
   (`--primary`).

### Geometría

- Diálogo de 420 px, `.rf-dialog` sobre `.rf-scrim`.
- Título en `.rf-title` y, 4 px debajo, «titular · DNI» en
  `.rf-prose rf-text-muted` —no en `.rf-hint`: es quién va a firmar, y se lee
  antes de teclear nada.
- El campo del PIN va a **18 px con 6 px de tracking**: se teclea a ciegas y
  los puntos se cuentan con la vista.
- Acciones abajo a la derecha, `Cancelar` fantasma a la izquierda de `Firmar`
  primario.

## Estados

- **Pidiendo PIN**: pista neutra, «El PIN se usa solo para esta firma y no se
  guarda en ningún sitio». Es una promesa de la aplicación, no relleno: lo
  contrario sería una decisión de seguridad.
- **PIN incorrecto**: `.rf-field--error` en el campo (borde de 2 px, la pista
  en negrita con un `!` delante) y el texto pasa a «PIN incorrecto. Te quedan
  **2 intentos** antes de que la tarjeta se bloquee».

El número de intentos restantes es información que da el propio módulo PKCS#11
y **hay que enseñarla**: bloquear una tarjeta por no avisar es un daño real y
no siempre reversible.

## Componentes y tokens

`.rf-dialog`, `.rf-scrim`, `.rf-field`, `.rf-field--error`, `.rf-label`,
`.rf-input`, `.rf-hint`, `.rf-divider`, `.rf-btn--ghost|--primary`.

Sin color de error: el sistema no lo tiene. El fallo se señala con borde, peso
y glifo, como manda [design-system.md](design-system.md).

## Lo que este diálogo deja abierto

Los demás fallos de PKCS#11 —tarjeta bloqueada, token ausente, sesión caducada,
módulo no encontrado— no están dibujados. Están pendientes en el mapa.

## Decisiones

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, artboards «6 · Pidiendo PIN» y «7 · PIN incorrecto».
