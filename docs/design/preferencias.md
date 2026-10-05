# Preferencias

Los ajustes de la aplicación. Se abre desde el menú de la
[cabecera](cabecera.md) y ocupa la ventana entera bajo ella. Esta ficha cuenta
el flujo entre secciones y el porqué de cada decisión; lo que se ve lo dicen las
historias.

Componentes: `rfirma-app/src/preferences/PreferencesView.tsx` (la vista, con el
estado y el modal de confirmación), `PreferencesSections.tsx` (una pieza por
sección) y los primitivos `design-system/Select.tsx` y `design-system/Switch.tsx`. La lista de certificados pinta cada fila
con `CertificateCard` (`signing/CertificateCard.tsx`), la misma pieza del selector de
certificado. Historias: `PreferencesView.stories.tsx`, en «Preferencias/Pantalla»,
con los ajustes y los certificados de ejemplo de `preferencesFixtures.ts`.

## Casos de uso que la usan

- Firmar un PDF en local — fuera del recorrido, en cualquier momento.

## Historias

Una por sección y por variante de lo que enseña:

- `General` y `GeneralWithActivityOff`: Privacidad con los dos interruptores
  encendidos y apagados.
- `SigningWithoutOriginalFolder`: solo la carpeta de destino, que es lo que ve
  quien corre bajo el sandbox.
- `SigningNextToTheOriginal` y `SigningInTheDestinationFolder`: los dos radios
  del destino, con cada uno elegido.
- `NoCertificates` y `CertificatesInstalled`: la lista vacía, y con un
  certificado personal, uno de representación y uno caducado.
- `Appearance`: tema e idioma.

Cuatro estados **no tienen historia** porque se llegan por un gesto y las
historias no llevan interacción: el diálogo de confirmar el borrado, el aviso de
ajuste no guardado, el de la lista que no se vacía y el de un certificado que no
se ha podido instalar o quitar. Los cubren las pruebas de comportamiento de
`PreferencesView.test.tsx` y `PreferencesView.certificates.test.tsx`.

## Estructura

**Un visor de pestañas en vertical, no un diálogo.** Es una **vista del
cuerpo**: sustituye lo que hubiera bajo la cabecera y ocupa ese hueco. La
cabecera pasa a su variante sin documentos —las pestañas son de documentos, y
Preferencias no es de ninguno— y se queda **viva, con su menú alcanzable**,
porque nada se pinta encima de ella.

Tres regiones:

1. **Índice de secciones**, permanente a la izquierda: *General*, *Firma*,
   *Certificados* y *Apariencia*. Es el patrón ARIA de pestañas (`tablist`,
   `tab`, `tabpanel`); pulsar una fila cambia el panel, no navega.
2. **Panel de la sección activa**, centrado y **solo ese**, con el título de la
   página arriba y su divisoria. Se entra siempre por *General*; no se recuerda
   la última sección, porque lo que una pantalla de ajustes tiene que recordar
   es el ajuste y no el sitio desde el que se miró.
3. **Pie fijo** con `Cerrar`.

**No hay vuelta atrás.** Las salidas son `Cerrar` y `Escape`, y `Escape` cierra
Preferencias entera desde cualquier sección. Las flechas arriba y abajo mueven la
sección con vuelta en los extremos. **El foco no se atrapa aquí dentro**: el
menú de la cabecera se alcanza por teclado con Preferencias delante.

**El índice no colapsa con el ancho** y **el desplazamiento es del panel**, no
de la pantalla: el índice y el pie no se mueven nunca.

Los cambios se aplican al hacerlos: **no hay «Guardar» ni «Cancelar»**, y
cambiar de panel no enturbia nada porque no hay transacción que confirmar.

## Los ajustes

Diez, en cuatro paneles. Los textos están en `preferences.*` de `po/es.po` y no
se copian aquí.

### General

Un solo grupo, **Privacidad** (`preferences.sections.privacy`):

- **Recordar mi actividad** (`preferences.rememberActivity.*`), con «Vaciar la
  lista» (`recents.clear`) al lado. Cubre los documentos recientes y el
  certificado usado: es la misma promesa a quien firma en un ordenador
  compartido. Apagarlo **borra** lo guardado, previa confirmación; vaciar sin
  apagar es «hoy no, mañana sí».
- **Avisar de versiones nuevas** (`preferences.notifyNewVersion.label`), sin
  ayuda. Vive bajo Privacidad porque la comprobación de versión es la única
  conexión saliente de rFirma.

### Firma

- **Recordar la firma visible** (`preferences.rememberVisibleSignature.label`),
  sin ayuda. Apagado significa no guardarla: el recuadro arranca en el valor por
  omisión en cada documento. La posición no se recuerda aquí sino por documento,
  en su fila de recientes.
- **Dónde guardar** (`preferences.destination.*`). Con `offersOriginalFolder`,
  dos radios: junto al original y en la carpeta elegida, con el **nombre** de la
  carpeta —no su ruta— y «Cambiar carpeta…», que abre el selector de directorio
  del sistema. Sin él, solo la carpeta y su botón.
- **Esperar antes de firmar** (`preferences.consentCountdown.label`): la pausa
  de tres segundos de la ventana de sede, y de nada más
  ([ventana-de-sede.md](ventana-de-sede.md)). Es el mismo ajuste que ofrece el
  [primer arranque](primer-arranque.md).
- **Usar el certificado que elija la sede**
  (`preferences.honourAutomaticSelection.label`), apagado por omisión: si solo
  sirve uno, la sede lo elige sin preguntar.
- **Permitir SHA-1** (`preferences.allowSha1.*`), apagado por omisión, con su
  explicación debajo, sin tecnicismos: algunas sedes antiguas lo piden y es
  menos seguro, así que se activa solo si se confía en la sede.

### Certificados

La lista de los `.p12` instalados en rFirma, con **dos gestos y nada más**:
`preferences.certificates.add` y, en cada fila, `actions.remove`
(`preferences.certificates.remove` es su nombre accesible).

- **Cada fila es un `CertificateCard`**, como en el selector: titular o
  representado, línea con el NIF, almacén, caducidad y motivo. Preferencias pasa
  a verse como el selector y deja de pintar el certificado a su manera, que es un
  cambio visible decidido.
- **Lo que identifica la fila es el certificado, no el fichero.** Del fichero no
  se recuerda nada, ni la ruta: instalar copia al almacén de rFirma lo que hace
  falta.
- **Un caducado se queda en la lista**, con el motivo por el que no puede
  firmar: que desaparezca no le explica nada a quien lo instaló.
- **Sin ninguno**, `preferences.certificates.empty` y nada más: el botón ya está
  encima.
- **No se copia el registro de almacenes de AutoFirma.** Allí hace falta porque
  la aplicación elige un almacén; rFirma los barre todos.
- **Una clave que no es RSA ni de curva elíptica se rechaza al instalar, no al
  firmar**, con un aviso de una línea en este panel y sin detalle técnico.

### Apariencia

- **Tema** (`preferences.theme.*`): el del sistema, claro u oscuro. El del
  sistema **no es «claro»**: es no forzar nada y dejar que mande
  `prefers-color-scheme`.
- **Idioma** (`preferences.language.label`, `languages.*`): los cinco, y solo
  aparece el que tiene todas las cadenas traducidas (ADR-0009).

Los valores posibles viven dentro de los desplegables; no hay textos debajo que
los enumeren.

## Estados y fallos

- **Confirmando el borrado**: apagar «Recordar mi actividad» abre un diálogo
  pequeño **encima** de la vista, con `Cancelar` y `Borrar y apagar`
  (`preferences.rememberActivity.confirm.*`). El interruptor **no se mueve**
  hasta confirmar, y el diálogo es modal: el teclado no sale de él, y `Escape`
  lo cancela sin cerrar Preferencias. Es un diálogo y no una confirmación en
  línea porque el borrado es irreversible, y tampoco se borra ofreciendo
  deshacer: un aviso temporal sobre algo ya borrado es el fallo silencioso otra
  vez.
- **Ajuste no guardado**: `ErrorNotice` (situación `settingNotSaved`) **dentro
  del panel donde se pulsó**, con el detalle técnico del rechazo. El control ya
  ha vuelto al valor anterior.
- **Lista que no se vacía**: el aviso (`activityNotForgotten`) va siempre en
  *General*, pegado a su botón.
- **Certificado que no se instala o no se quita**: el aviso, en *Certificados*,
  entre «Añadir…» y la lista, que no cambia. Si el llavero perdió el PIN del
  almacén, el aviso ofrece vaciarlo, con confirmación (ADR-0034).

Los avisos se pintan donde se hizo el gesto y nunca en una franja común: con
cuatro paneles, un aviso común obliga a leer el texto para saber qué se rompió.

## Componentes y tokens

Los primitivos `Button`, `Dialog`, `Row` y `Stack`; `CertificateCard` y
`ErrorNotice`; `Select` y `Switch`, del sistema de diseño; `.rf-label`,
`.rf-title`, `.rf-prose`, `.rf-hint`, `.rf-divider`; `--rf-surface` y
`--rf-border-subtle`. La geometría (índice de 220 px, panel de 720 px como
máximo) es de `PreferencesView.css`.

Hay **tres niveles de texto dentro de un panel** y ninguno estrena estilo: el
título de página es `.rf-label` en versalitas con su divisoria; el encabezado de
grupo, `.rf-title` sin divisoria; la etiqueta de un control, `.rf-label` en caja
baja y apagada. Se separan por tamaño, peso y color a la vez, que es lo que
impide confundir un grupo con la etiqueta del control de al lado. Solo *General*
tiene grupos.

## Decisiones

**Visor de pestañas y no navegación de dos niveles.** Con el índice permanente
desaparecen la vuelta atrás, el `Escape` ambiguo y el comportamiento distinto por
ancho, en vez de contestarse. Se descartó `AdwNavigationView`: compra un gesto de
más en cada cambio de sección a cambio de un retroceso que aquí nadie necesita.
Que `Escape` cierre el contenedor entero es lo que respalda la GNOME HIG, y sale
barato porque no hay nada sin aplicar.

**Cuatro secciones y no cinco.** *Sedes* se fue: su desplegable se mudó a la fila
*Firma en sedes* del [panel de estado](panel-de-estado.md), único sitio donde se
elige el programa que atiende las sedes, y «Preguntarme al arrancar» se borró
porque gobernaba un aviso que ya no existe. *Privacidad* pasó a ser el único
grupo de *General*, para que sus dos interruptores se vean al abrir.

**No es un diálogo ni una ruta de un router.** Con guardado automático y `Cerrar`
como única salida no hay estado al que navegar, así que `Escape` sigue valiendo
y `Cmd+,` sigue prometiendo lo que abre.

**El desplegable no es un `<select>` nativo.** La lista que despliega el
elemento nativo la pinta el sistema de ventanas, no la hoja de estilos, y salía
con los colores del escritorio en una pantalla hecha con tokens. A cambio, `Select`
repone a mano `combobox`, `listbox`, las flechas, Inicio, Fin, Intro, Escape, el
cierre al pulsar fuera y el foco de vuelta.

**El aviso de versiones se ofrece siempre.** Se descartó mostrarlo solo si nadie
gestiona la instalación: leer `sources.list.d` no es fiable, y el flatpak no
tiene esos ficheros. Avisar siempre y dejar apagarlo cuesta menos y no miente.

**El destino es un modo que se elige**, y bajo el sandbox no hay radios. El
motivo **no es privacidad** sino corrección: devolver una ruta que no se conoce
es devolver una mentira, y una opción atenuada contaría al usuario nuestros
problemas de empaquetado. Un documento que llega por el portal cae en la carpeta
elegida aunque se elija «junto al original», porque su ruta no es la del
original. La comprobación previa de la carpeta está en el
[ADR-0011](../adr/0011-destino-del-documento-firmado.md).

**Los ajustes se guardan al elegirlos**, por `PreferencesStore`, que pasa por
`remember_configuration`: el único sitio donde el borrado al apagar «Recordar mi
actividad» no se puede olvidar ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)).
El alcance de la traducción, en el
[ADR-0009](../adr/0009-catalogo-de-cadenas-propio-y-cinco-idiomas.md).

**La cabecera solo pierde lo de los documentos** mientras Preferencias está
delante; el documento sigue cargado detrás y vuelve al cerrar
([ADR-0007](../adr/0007-cabecera-unica-sin-barra-de-menus.md)).
