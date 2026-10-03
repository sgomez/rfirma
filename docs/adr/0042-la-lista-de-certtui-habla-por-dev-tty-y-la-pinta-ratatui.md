# La lista de `-certtui` habla por `/dev/tty` y la pinta ratatui

`-certtui` (ADR-0041) elige el certificado en la terminal desde la que se lanza
la orden. Esa lista tiene que poder usarse en mitad de un script y parecerse a
la del panel: una fila por certificado aunque esté en varios almacenes, en qué
calidad firma y dónde está cada copia, y un filtro mientras se escribe.

## La regla

1. **La lista y el PIN hablan por `/dev/tty`, nunca por stdin ni por stdout.**
   stdout es de lo que se consume (ADR-0041, regla 4): con `-xml` o
   `listaliases`, un menú pintado ahí acabaría dentro del fichero de quien
   redirige, y la persona no lo vería. stdin puede ser una tubería: leer de ahí
   las teclas consumiría los datos de otro o cancelaría con el EOF. `/dev/tty` es
   la terminal que controla el proceso, se redirija lo que se redirija. Si stdin
   es esa misma terminal, crossterm lee de stdin, que es el mismo dispositivo.
2. **La pinta ratatui, en un viewport *inline* bajo el prompt, sobre crossterm.**
   El backend escribe en el `File` de `/dev/tty`, y al elegir se borra la lista
   y queda una línea con el certificado elegido.
3. **La lista no hace E/S.** El estado, el filtro y el pintado van en un módulo
   que recibe `KeyEvent` y pinta en un `Frame`; las pruebas lo atacan con
   eventos y con `TestBackend`, sin TTY. El conductor que abre `/dev/tty` y lee
   teclas es una capa fina sin pruebas propias.
4. **Las filas salen de la misma regla que el panel.** Agrupar las copias por
   emisor y número de serie, y escoger la que firma, vive en el dominio de
   `identity` y lo usan la ventana y la terminal.

## Considered Options

- **Seguir a mano, con `termios` y secuencias ANSI.** Era lo que había, y es lo
  que se ha sustituido. Ya cumplía la regla 1 y se probaba en memoria, pero el
  filtro, el desplazamiento con más filas que la terminal, el recorte por ancho
  visible y el redibujado de varias líneas por fila eran unas doscientas líneas
  más del código en el que se esconden los fallos de los caracteres anchos y del
  redimensionado.
- **dialoguer (`FuzzySelect`).** Cumple la regla 1 con
  `Term::read_write_pair` sobre `/dev/tty` y trae el filtro hecho. Descartada:
  cada opción es una sola línea, así que no cabe la fila del panel, y `console`
  lee las teclas de la TTY real, de modo que el selector deja de poder probarse
  sin ella.
- **inquire (`Select`).** Su backend crossterm escribe siempre en stderr, y el
  que habla por `/dev/tty`, termion, solo existe en Unix y no se puede inyectar:
  la API de backends es interna del crate. Descartada por la regla 1.
- **cliclack.** Solo pinta en stderr; el `interact_on` que admitiría otra
  terminal está en un módulo privado. Descartada por la regla 1.
- **Hacerlo en stderr.** Descartada: con `2>registro.log` el menú se iría al
  registro y la orden se quedaría esperando sin que se viera nada.
