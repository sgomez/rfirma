# El banco de referencia

`make-reference-signatures.sh` y `validate.sh` (con `reference-signer/`, que
solo resuelve su clasepath desde Maven local, ADR-0002) llaman a los firmadores
monofásicos del original 1.9.2 (`AOCAdESSigner`, `AOXAdESSigner`,
`AOFacturaESigner`) para producir y validar `testdata/reference/`, con el que
cada formato compara su salida con la del original. Ver
`testdata/reference/README.md`. `make-previous-signature-samples.sh` y
`openssl-tsa.py` fabrican las muestras de `testdata/previous-signatures/` (ver
su `README.md`).

Los guiones de medición que sostuvieron las notas de `docs/research/` se
borraron; cada nota enlaza el commit anterior al borrado.
