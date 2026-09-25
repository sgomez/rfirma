# La selección automática que pide la sede se respeta solo si la persona lo permite

Una sede declara en `properties` que no hace falta elegir certificado de dos
maneras: `headless=true` o `mandatoryCertSelection=false`. AutoFirma 1.9.2 lee
las dos en `CertFilterManager.isMandatoryCertificate` (`:145-154`) y se las
pasa al diálogo de selección, que con **exactamente un** candidato tras los
filtros lo elige sin enseñarse (`AOKeyStoreDialog.show`, `:726-729`); con dos
o más, el diálogo sale igual. Vale para `selectcert`, `sign`, `cosign`,
`countersign`, `signandsave` y los dos lotes, que abren el mismo diálogo con
los `properties` de su petición. `headless=true` hace además una cosa que
`mandatoryCertSelection=false` no hace: lo que habría que preguntar a la
persona —el PDF certificado, la contraseña del PDF, las firmas no registradas,
la confirmación del validador— se rechaza con `SAF_50`
(`ProtocolInvocationLauncherSign.java:432, 793`). Ninguno de los dos se salta
el PIN: si el almacén lo pide, se pide.

En AutoFirma ese diálogo es el único consentimiento que hay. Encadenado con
un almacén que no pide PIN —un `.p12` instalado, un llavero abierto—, una web
cualquiera obtiene una firma, o la identidad completa de quien firma, sin que
se abra nada. Es el mismo riesgo grave que llevó al ADR-0010 a que el
certificado fijado con `sticky` no conteste por sí solo, y es una excepción del
ADR-0023 al «se sigue al original».

## La regla

1. **Por omisión, rFirma enseña siempre su consentimiento**, también con un
   solo candidato y aunque la sede pida la selección automática. El único
   candidato llega preseleccionado.
2. **La preferencia «Respetar la selección automática de certificado que pida
   la sede»** (`honour_automatic_selection`, apagada por omisión) restaura el
   original: con `headless=true` o `mandatoryCertSelection=false` y un único
   candidato, `selectcert` contesta sin ventana, y la firma y los lotes siguen
   sin esperar a la persona —la ventana consiente sola con ese candidato—. Los
   candidatos se cuentan como el original, con todo lo que admite el filtro de
   la sede: si declara filtros explícitos, los caducados que admite cuentan
   aunque rFirma no firme con ellos. Con dos o más se pregunta igual.
3. **La preferencia no se salta nada más.** El PIN, el diálogo para elegir el
   fichero sin `dat`, el de guardar de `signandsave`, el área de la firma
   visible y, con `mandatoryCertSelection=false`, las preguntas del PDF y del
   validador se siguen enseñando. Solo `headless=true` las rechaza con
   `SAF_50`, y eso no depende de la preferencia. Y la ventana solo consiente
   sola si no tiene ningún aviso que enseñar: un PDF con firmas no
   registradas, cuya pregunta rFirma hace dentro del consentimiento, lo pide
   siempre, aunque la sede declare `allowCosigningUnregisteredSignatures=true`.
4. **`sticky` no cambia con la preferencia** (ADR-0010): el fijado en la sesión
   se preselecciona entre varios candidatos, pero no contesta.
5. **La suite de conformidad mide rFirma con la preferencia encendida**: el
   perfil de usar y tirar que monta `scripts/isolated-store.sh` la activa, y
   el informe compara el comportamiento que el original tiene de fábrica.

## Considered Options

- **Respetarla siempre, como el original.** Descartada por la regla 1: la
  persona no ve nada cuando el almacén no pide PIN, y la sede, no quien firma,
  decide si hay ventana.
- **Ignorar los dos parámetros siempre.** Era lo que hacía rFirma. Deja fuera
  a quien firma a diario en la misma sede y ya ha decidido que no quiere
  confirmar cada trámite, y deja a la suite midiendo un comportamiento que el
  original no tiene.
- **Tratar `mandatoryCertSelection=false` como `headless=true`.** Era lo que
  hacía rFirma antes de esta regla: con `mandatoryCertSelection=false` también
  contestaba `SAF_50` a lo que había que preguntar. El original no lo hace.
