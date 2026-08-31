# El banco de medición

Estos guiones son el banco con el que se midieron los issues
[#2](https://github.com/sgomez/rfirma/issues/2),
[#12](https://github.com/sgomez/rfirma/issues/12),
[#13](https://github.com/sgomez/rfirma/issues/13),
[#14](https://github.com/sgomez/rfirma/issues/14),
[#23](https://github.com/sgomez/rfirma/issues/23) y
[#36](https://github.com/sgomez/rfirma/issues/36). Las notas de
`docs/research/` los citan por su nombre, así que se conservan tal cual: son la
reproducción de una medición que ya está hecha, no herramientas de trabajo.

**No corren contra el puente actual.** El
[#48](https://github.com/sgomez/rfirma/issues/48) reescribió `NativeBridge` y
con él la frontera FFI: los puntos de entrada se llaman `autofirma_*`, devuelven
JSON, y la postfirma exige el **sello de sesión** del
[ADR-0016](../../docs/adr/0016-sello-de-sesion-una-sola-invariante.md), que estos
guiones no tienen de dónde sacar. `run-jvm-control.sh` además invocaba un `main`
de control que ya no existe: las pruebas de JUnit del puente son ahora ese
camino, y se ejecutan con `just test-java` (grada A) y `just test-native`
(grada C, la que valida el PDF con `pdfsig`).

Quien necesite volver a medir algo de aquí, que adapte el guión a la frontera
nueva en el mismo commit en que lo use.
