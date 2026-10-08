# El flatpak trae su OpenSC y usa el `pcscd` del anfitrión

En el `.deb` y el `.rpm`, rFirma ve las tarjetas con lo que tiene el sistema: el
`pcscd` del anfitrión y los módulos PKCS#11 que registra p11-kit, OpenSC entre
ellos (ADR-0048). En el sandbox no entra nada de eso: el runtime no trae OpenSC
ni `libpcsclite`, flatpak solo reenvía por p11-kit la confianza del anfitrión, y
el `opensc-pkcs11.so` del anfitrión no carga contra las bibliotecas del runtime.
Forzarlo con `LD_LIBRARY_PATH` mezclaría dos glibc en un mismo proceso.

El flatpak resuelve cada mitad por separado:

1. **El demonio lo pone el anfitrión.** `--socket=pcsc` monta el socket de su
   `pcscd`, y el manifiesto compila solo la biblioteca cliente de `pcsc-lite`.
   Nunca un `pcscd` propio ni `--device=all`: el USB del lector lo sigue
   gobernando el sistema, con su polkit. El permiso deja hablar con cualquier
   tarjeta de cualquier lector del anfitrión, y no hay portal más estrecho que
   lo sustituya.
2. **El cliente de `pcsc-lite` no baja de la 2.4.1**, la primera que reintenta
   con la versión de protocolo del servidor. Así negocia con los `pcscd` de
   todas las distribuciones con soporte.
3. **OpenSC va dentro del paquete**, compilado en el manifiesto con versión y
   `sha256` fijos, y con zlib, OpenSSL y SM exigidos al configurar: sin zlib el
   driver del DNIe carga, pero no lee sus certificados. Se sube de versión
   cuando OpenSC publica, nunca desde `master`.
4. **rFirma lo encuentra como en el `.deb`**, por el `.module` de p11-kit que
   instala el propio OpenSC bajo `/app`. La raíz `/app` solo se mira en el canal
   flatpak.

Solo entra OpenSC: el módulo de un fabricante que el anfitrión registra en
p11-kit, y su `opensc.conf`, no llegan al sandbox. Quien los necesite tiene el
`.deb` y el `.rpm`. OpenSC no es la librería nativa del ADR-0004: su regla de un
solo `.so` vale para `/app/lib/rfirma`, no para `/app/lib/pkcs11`.

Las mediciones, en `docs/research/dnie-en-flatpak.md`.

## Consequences

- Las correcciones de seguridad de OpenSC dentro del flatpak son de rFirma, no
  de la distribución.
- El flatpak no enseña la ventana de confirmación de firma del DNIe que trae el
  OpenSC de Debian y Ubuntu (`--enable-dnie-ui`), igual que Fedora: el PIN lo
  pide rFirma (ADR-0047).
- El socket se monta al arrancar. Si el anfitrión no tiene `pcscd`, o lo
  reinicia con rFirma abierto, el estado es «sin lector» hasta reabrir rFirma.
- Contra un `pcscd` con protocolo 4:4, la detección en caliente sigue al número
  de lectores.
