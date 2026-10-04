# Primer arranque

El asistente que ve quien abre rFirma por primera vez: cuenta qué es esto y qué
no es, y deja el equipo configurado —el certificado propio instalado y rFirma
puesta como programa que abren las sedes— sin mandar a nadie a buscarlo por su
cuenta. Es la primera pantalla de la aplicación, antes de que haya documento
alguno. **Informa y además hace.**

Componente: `rfirma-app/src/setup/SetupWizard.tsx`. Historias:
`SetupWizard.stories.tsx`, en «Primer arranque», con las filas de estado del
equipo de `setupStoryFixtures.ts`, que comparten las pruebas.

## Casos de uso que la usan

- **Primer arranque de rFirma**, de principio a fin: es el único caso de uso que
  abre esta pantalla. La reparación posterior de las dos cosas que configura
  vive en el [panel de estado](panel-de-estado.md), no aquí.

## Qué resuelve

Instalada la aplicación quedan dos cosas por hacer que nadie va a adivinar: el
certificado propio que hace segura la conexión del navegador con rFirma, y que
las sedes electrónicas abran rFirma en vez de AutoFirma. El asistente las junta
donde la persona ya está mirando y pone por delante el deslinde: rFirma no es la
aplicación oficial.

## Historias

- `Welcome`: la primera pantalla.
- `NothingDone`: la segunda, con las dos acciones pendientes.
- `CertificateDoneHandlerPending`: el certificado ya instalado y las sedes sin
  asignar.
- `BothDone`: las dos hechas, sin botones de paso.
- `WithoutAutoFirma`: las sedes sin programa asignado, que cambia la frase del
  segundo paso.
- `ProtectionOff`: la protección contra firmas accidentales apagada.

Los estados que dependen de un gesto —un paso trabajando, un paso fallado con su
lista de destinos, una acción declinada— **no tienen historia**: se llegan
pulsando, y los cubre `SetupWizard.test.tsx`. Para abrir la historia en la
segunda pantalla, el componente acepta `initialStep`.

## Estructura

**Es la ventana principal, no un diálogo sobre ella.** El primer arranque no
tiene documento que tapar, y un modal con la aplicación muerta detrás miente
sobre lo que hay debajo. Lleva la cabecera en su variante sin documentos y nada
más de la ventana principal: ni botón de abrir, ni pestañas, ni panel de firma,
ni recientes. En Linux la cabecera es la barra de título nativa
([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)), sin tira de
pestañas.

Tres regiones: la **cabecera**; el **cuerpo**, desplazable, con una columna de
lectura centrada y arriba el indicador de paso (`setup.step`); y el **pie**, con
las acciones a la derecha, que **no flota sobre el contenido** sino que es
hermano de la zona que se desplaza, así que no puede tapar un control enfocado.

### Dos pantallas

| # | Qué lleva | Pie |
| - | --------- | --- |
| 1 | Bienvenida: titular, qué es rFirma, el deslinde de independencia y el idioma | omitir · continuar |
| 2 | Las dos acciones como pasos numerados y, aparte, la protección contra firmas accidentales | atrás · terminar |

Claves del pie: `setup.actions.skip`, `actions.continue`, `actions.back` y
`setup.actions.finish`.

**El rechazo es por acción y el recorrido entero se puede omitir.** Cada acción
lleva su `actions.notNow` en secundario junto al botón, y no hay primario
desactivado con una línea que diga qué falta. La bienvenida lleva omitir porque
rFirma firma documentos sin sedes, y a quien solo quiere eso no se le hace
recorrer una configuración que no necesita. Omitir cuenta como haber visto el
asistente; las dos acciones siguen en el panel de estado.

### La segunda pantalla: pasos numerados

**Sin tarjetas.** Las dos acciones son dos pasos con un círculo a la izquierda,
`1` y `2`, unidos por una línea vertical. **El círculo dice el estado**: el
número mientras está pendiente, un spinner mientras trabaja, una marca cuando
está hecho y un aviso cuando ha fallado. A la derecha, el título, la frase y,
debajo, los botones; **el resultado ocupa el sitio de los botones**, en el mismo
nodo.

- **Certificado de rFirma**: `status.signals.localCaCertificate` con
  `setup.certificate.body`, y `status.actions.install`. Trabajando:
  `setup.certificate.installing`. Hecho: `setup.certificate.installedTitle` y,
  cuando hay que reiniciar el navegador, `status.notices.restartFirefox`. Fallado:
  `setup.certificate.failedTitle`, la lista con marca por destino
  (`status.storeBrands.*`) y `actions.retry`.
- **Usar rFirma por defecto**: `setup.handler.title` con `setup.handler.body`,
  que **se omite** cuando AutoFirma no aparece entre los candidatos, y
  `status.actions.useRfirma`. Hecho: `setup.handler.done`.

**La protección contra firmas accidentales va aparte, al final**, tras una
divisoria: no es un paso sino un ajuste, con el interruptor a la derecha y el
título de `preferences.consentCountdown.label`. Es el mismo ajuste que
[Preferencias](preferencias.md), y se escribe por `PreferencesStore` en cuanto
cambia.

## Flujo y estados

Los dos pasos son **independientes**: cualquiera puede estar hecho, declinado o
fallado sin que el otro se entere. Al montar, el asistente **mide el equipo** con
los mismos casos de uso del panel de estado (`StatusPort`); no tiene los suyos
propios, y el estado de los pasos no se guarda entre sesiones. Mientras mide, no
ofrece botones de paso.

- Pendiente → trabajando → hecho o fallado, por paso. Un paso fallado se
  reintenta con la misma acción.
- Un paso ya hecho al medir arranca hecho y sin botones.
- **El fallo nunca atrapa.** Un paso fallado deja seguir y el pie no se bloquea:
  impedir avanzar por algo que la persona no puede arreglar sería un callejón sin
  salida, y la reparación ya tiene sitio en el panel de estado.
- Terminar —o omitir— marca `setupWizardSeen`, haya salido como haya salido cada
  acción; una vez visto, el asistente no se monta.

**WCAG 2.2 AA**, los dos criterios que esta pantalla incumpliría por omisión:

- **4.1.3 Status Messages.** El salto de «instalando» a un resultado se anuncia
  con `role="status"` sobre el bloque de resultado —el mismo nodo cambia de
  contenido, no aparece uno nuevo— y **el foco no se mueve**.
- **2.4.11 Focus Not Obscured.** El pie no flota.

## Textos

Claves: `setup.*`, `about.independenceLead` y `about.independence` para el
deslinde, `preferences.language.label` para el idioma y las de arriba para los
pasos. Los textos no se copian aquí: se leen en `po/es.po`. `setup.welcome.body`
lleva la versión de AutoFirma como parámetro, para que cada actualización del
original no obligue a rehacer la traducción en los cinco idiomas
([ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-cinco-idiomas.md)).

## Componentes y tokens

`Button`, `Card`, `Row` y `Stack`; `Select` de Preferencias; `ErrorNotice`;
los iconos de marca, aviso y spinner; el `Header` de la ventana principal;
`.rf-heading`, `.rf-title`, `.rf-prose`, `.rf-hint`, `.rf-divider` y los tokens
de color, radio y espacio. El indicador de paso, los círculos de los pasos y las
marcas de la lista de destinos son de `SetupWizard.css` y no son componentes del
sistema de diseño.

## Decisiones

**Dos pantallas, y no tres ni una.** Tres pantallas encadenadas alargan a tres
pasos lo que son dos decisiones. Una sola página con los tres bloques apilados
no cabía en el hueco y se desplazaba, y una página se lee de un vistazo solo si
cabe de un vistazo.

**La segunda pantalla son pasos numerados, sin tarjetas.** Se compararon cuatro
maquetas con los mismos textos —lista agrupada, icono con botones debajo, pasos
numerados y acciones y ajustes bajo rótulos—, y se eligió la única en que el
estado de cada acción se lee en el círculo sin texto añadido.

**Sin botón de abrir ni pestañas**: una barra con «Abrir PDF…» invitaría a abrir
un PDF antes de terminar la configuración.

**La protección entra en el asistente**, fija en la segunda pantalla y aparte de
los pasos: es el mismo ajuste, y no un paso que haya que completar.

**El deslinde reutiliza la cadena de [«Acerca de»](acerca-de.md)**, ya traducida
a los cinco idiomas; la tarjeta separa el encabezado como título y el resto como
cuerpo, sin partir ni reescribir la cadena.

**El idioma se elige en la primera pantalla** porque es la única que se lee entera
antes de decidir nada: quien no entiende el idioma del sistema no puede llegar a
Preferencias para cambiarlo. Arranca en el idioma resuelto
([ADR-0010](../adr/0010-memoria-entre-sesiones.md)) y el cambio repinta el
asistente al momento; si no se guarda, el aviso sale en la tarjeta.

**`afirma://` no se nombra en la interfaz.** Sigue siendo el mecanismo
([ADR-0005](../adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md)), pero
quien usa esto no es técnico. La pantalla habla de **qué programa abren las
sedes**.

**Se dice «certificado propio», no «autofirmado» ni «certificado» a secas.**
«Autofirmado» es la palabra de las pantallas rojas del navegador y arrastra su
alarma a algo que aquí es correcto; «certificado» a secas choca con el de firma
de la persona. Por lo mismo, se cuenta por **lo que consigue** —que la conexión
del navegador con rFirma sea segura— y no por lo que es. El canal cifrado es entre
el navegador y rFirma, en el mismo ordenador, y el texto no insinúa un canal con
la sede.

**La línea de reiniciar el navegador sale siempre**, sin detectar nada: la
escritura del certificado nunca falla ni se pierde con el navegador abierto, y
instalar se ve en caliente casi siempre; quien caiga en el resto creería que no
ha funcionado. No hay aviso previo de cerrar los navegadores.

**El fallo se cuenta con una lista con marca por destino**, no en prosa: la señal
es agregada, y enumerar en una frase qué entró y qué no obliga a releerla.

**La HIG no respalda nada de esto, y está asumido:** no tiene patrón de
asistente y `GtkAssistant` está obsoleto desde GTK 4.10. Sirve para acotar
—«minimize the number of steps»—, no para justificar.

## Lo que queda abierto

- **Tres botones primarios en la pantalla 2**: cada paso ofrece su acción y el
  pie navega, pero con los dos pasos sin hacer el ojo no tiene dónde caer. La
  salida, si se toma, es degradar a secundario los dos botones de los pasos.
- **El aviso del permiso de red local** se sacó del asistente: le toca aparecer
  en la primera firma, y esa pantalla no está diseñada.
- **Qué se recuerda cuando el asistente queda a medias**: hoy es un único
  booleano para dos acciones independientes. Es una enmienda al ADR-0010.
- **Desde dónde se vuelve a ver el asistente**, sin decidir.
