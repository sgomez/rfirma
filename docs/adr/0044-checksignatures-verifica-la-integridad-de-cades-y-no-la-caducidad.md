# `checkSignatures` verifica la integridad de una CAdES y no mira la caducidad del certificado

Con `checkSignatures=true`, la sede valida las firmas previas antes de cofirmar
o contrafirmar. AutoFirma 1.9.2 llama a `ValidateBinarySignature.verifySign`
con `checkCertificates=false` para eso, y con `false` no verifica ni el valor
de firma ni el `messageDigest`: una CAdES con un byte del contenido cambiado
salía válida. Este ADR fija qué comprueba rFirma en ese punto y completa la
regla del ADR-0023 para el protocolo afirma.

## La regla

1. **En el protocolo afirma, rFirma puede ser más permisiva que AutoFirma, pero
   nunca más estricta, salvo con un criterio que ningún documento legítimo
   pueda disparar.** Romper un trámite que hoy funciona hace perder a quien lo
   usa.
2. **Integridad: se rechaza.** Una CAdES manipulada no es nunca legítima,
   AutoFirma también la rechaza y EN 319 102-1 §5.1.4.1 prohíbe desactivar esa
   comprobación. El motor de validación verifica el valor de firma y el
   `messageDigest` de cada firmante, y el veredicto es *no válida*.
3. **Certificado caducado: no se rechaza, en ningún formato.** Cofirmar un
   documento antiguo cuyo firmante ya tiene el certificado caducado es el caso
   más común, eIDAS lo clasifica como `INDETERMINATE` y la sede valida en su
   servidor. rFirma se aparta a propósito de AutoFirma, que solo lo rechaza en
   CAdES y XAdES con `SAF_39`, y lo hace como efecto de pasar `checkCertificates=true`
   a esos validadores, no como criterio propio.
4. **Cómo se verifica una CAdES con el certificado caducado.** La integridad se
   comprueba con la clave pública del certificado, no con el certificado: el
   `signing-time` puede caer fuera de su vigencia y SpongyCastle rechazaría la
   firma. Es la misma verificación que usa el informe de firmas previas.
5. **No se añade cadena, revocación ni listas de confianza a `checkSignatures`.**
   Rechazaría documentos que AutoFirma acepta y necesitaría red.

## Consequences

- Las comprobaciones de la suite de conformidad
  `check_signatures_stops_a_cosign_over_a_tampered_cades` (AutoFirma y rFirma
  coinciden en `SAF_39`) y
  `check_signatures_cosigns_over_a_cades_with_an_expired_certificate` miden
  esta regla; la segunda lleva la etiqueta `rfirma:adr-0044`, porque
  AutoFirma responde `SAF_39` y rFirma continúa.
- PAdES no cambia: respeta `checkCertificates`, por defecto `false`, y acepta
  el certificado caducado como antes.

## Considered Options

- **Pasar `checkCertificates=true` a secas.** Se descartó porque rechazaría la
  CAdES íntegra con el certificado caducado, que rFirma acepta hoy y que ningún
  documento legítimo debería romper.
- **Dejar `checkCertificates=false`, como estaba.** Se descartó porque incumple
  EN 319 102-1 §5.1.4.1 y diverge del original, que verifica la criptografía.
- **Rechazar también el certificado caducado, como el original en CAdES y
  XAdES.** Se descartó por la regla 1: es el caso más común al cofirmar y no es
  un problema de integridad.
