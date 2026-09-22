# Dos sedes: la publicada para lo de punta a punta, la escrita a mano para la gramática

La suite interpretaba la sede con el `autoscript.js` publicado de AutoFirma 1.9.2, y varias
comprobaciones se daban por NO OBSERVABLE con el motivo de que «el cliente publicado nunca lo
pide». Pero 49 comprobaciones ya no cargaban `autoscript.js`: sus guiones `protocol-*` construyen
el `afirma://` a mano y hablan WSS o TLS en crudo. Otras parcheaban el `autoscript.js` (v3, IPv6,
gzip, puertos). La regla escrita y la suite decían cosas distintas.

**La sede tiene dos formas, y cada guion declara cuál usa**:

- **Sede publicada**: `autoscript.js` bajo Node, parcheado o no. Mide las operaciones de punta a
  punta, como las pediría una sede real.
- **Sede a mano**: mensajes escritos en crudo. Mide la gramática del protocolo: lo que un cliente
  recibe si otra sede lo envía, aunque la publicada no sepa pedirlo.

«La publicada nunca lo pide» deja de ser un motivo de NO OBSERVABLE. Lo es que ninguna de las dos
formas llegue a ver el resultado.

## Consequences

- Lo que hoy es NO OBSERVABLE solo por ese motivo pasa a medirse con la sede a mano: la barra final
  de los verbos, `jsonbatch` en mayúsculas, versiones fuera de rango en el lanzamiento, entre otras.
- La forma se declara en el guion, no en la comprobación, y la comprobación la hereda del guion
  que usa.
- Una comprobación no puede fingir la respuesta del cliente. Si la sede no llega a observar nada,
  el resultado es NO OBSERVABLE, nunca uno compuesto por el propio guion.

## Considered Options

- **Solo la sede publicada.** Descartada: dejaría sin medir casi todo el transporte y los
  parámetros, justo la parte del protocolo que un cliente recibe de sedes que no usan
  `autoscript.js`.
- **La sede a mano, también para lo que hoy se parchea.** Descartada por ahora: reescribir en crudo
  cuatro guiones que funcionan no cambia lo medido.
