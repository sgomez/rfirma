# Suite de conformidad

Glosario de la suite de conformidad, un bounded context aparte del de la aplicación rFirma: aquí
las palabras significan lo que tienen sentido para quien mide, aunque en `CONTEXT.md` de la raíz
signifiquen otra cosa. Define **qué es** cada término; lo decidido vive en los ADR (`CONTEXT-MAP.md`).

## Language

### Qué se mide

**Suite de conformidad**:
El catálogo de exigencias del protocolo `afirma://` y lo que las mide contra el **cliente** que se
le configure. Produce un **informe**, no una puerta: no bloquea ningún PR ni ninguna etiqueta
(ADR-0014) y se ejecuta en local, fuera del CI. **El protocolo es lo que pretende el código de
AutoFirma 1.9.2**: cada exigencia dice lo que ese código quiere responder, también donde un bug
suyo se lo impide (ADR-0026). **La suite no sabe qué cliente mide**: ni corre comprobaciones
distintas ni dice cosas distintas según quién responda.
_Avoid_: banco de conformidad, sondeo, arnés

**Cliente**:
La aplicación a prueba, AutoFirma o rFirma: un binario instalado. Cada **almacén** le da un perfil
aislado con su raíz de confianza. Saber cuál es sirve solo para lanzarlo, nunca para juzgarlo.
_Avoid_: sujeto, perfil, aplicación a prueba

**Sede**:
El lado web del protocolo, que invoca al cliente y habla con él. La suite la hace de dos formas
(ADR-0027):
- **Sede publicada**: el `autoscript.js` de AutoFirma 1.9.2 corriendo bajo Node, parcheado o no;
  habla como lo haría una sede electrónica real. Mide las operaciones de punta a punta.
- **Sede a mano**: mensajes del protocolo escritos en crudo, que la publicada nunca envía pero que
  cualquier sede podría enviar. Mide la gramática del protocolo.
_Avoid_: conductor, driver, cliente publicado, sonda

**Trámite**:
Una ejecución de un guion de la sede contra el cliente y lo que se observó en ella. Varias
comprobaciones pueden juzgar el mismo trámite: se guarda en el informe con su **clave** —modo,
guion, almacén y arnés— y las tandas no lo relanzan. Repetir a mano una
comprobación ya resuelta sí lanza el cliente, y el trámite nuevo sustituye al guardado. El de una comprobación con persona no tiene clave, porque lo
que viaja depende de lo que ella haga en el diálogo.
_Avoid_: errand (en la interfaz), ejecución, corrida

**Familia de trámite**:
El camino por el que un guion llega al cliente: el eco v4 en crudo, el canal de servicio, una
operación de punta a punta por la sede publicada o esa misma operación por servidor intermedio. La
declara el guion, no la comprobación.

**Condición**:
Lo que un guion de la sede mide durante el trámite y emite con un nombre propio de ese guion, no
con el de una comprobación. La comprobación nombra la condición que espera, o varias que tienen que
cumplirse todas. Si alguna llega, deciden ellas: una que falte es NO OBSERVABLE, salvo que otra
salga NO CONFORME. Si no llega ninguna y la sede recibió un código de error —un SAF, CANCEL o
MEMORY_ERROR—, el trámite no se completó, y es NO CONFORME.

**Manifiesto**:
Lo que la sede publica de sí misma: sus modos y sus guiones, cada uno con su sede, su familia de
trámite y sus condiciones, y lo que solo usa el banco de la aplicación. El catálogo se valida contra
él al arrancar.

**Saludo**:
La comprobación que abre una familia de trámite: si falla, no se corre ninguna comprobación de su
familia, sea del conjunto que sea. La familia de punta a punta tiene dos: uno sin ventana, que abre
el tramo `ninguna`, y una firma de verdad, que abre el tramo `clic`.

**Comprobación**:
Una entrada del catálogo: una exigencia del protocolo, cómo se provoca, qué hace la persona y una
sola expectativa. Se presenta con **qué se exige** —lo que debe pasar—, su **fuente** —dónde lo hace
el código de AutoFirma 1.9.2—, lo que hay que hacer **antes de empezar** o **durante la prueba** y,
una vez corrida, **qué pasó** y su **resultado**. Declara su **acción** y su **almacén**; su
**asistencia** se deduce de la acción. Dos comprobaciones que provocan el mismo trámite, con la misma
acción, y esperan lo mismo son la misma, y el catálogo no arranca.
_Avoid_: check, entrada (en la interfaz), enunciado, cita, aviso, observación

**Acción**:
Lo que hace la persona durante el trámite, con una instrucción cerrada: nada, consentir —elegir el
certificado, pulsar en el consentimiento o cerrar el error que aparezca, solo para llegar a lo que
se mide— o una acción con
nombre: cancelar, elegir un fichero, guardar con lo propuesto, teclear un PIN erróneo, teclear la
contraseña, marcar el área o rechazar. Es entrada del protocolo, no juicio: la persona nunca dice
qué vio, y el resultado lo da siempre el trámite —el código, las condiciones, el disco o el
silencio—.
_Avoid_: pregunta, respuesta de la persona

**Asistencia**:
Qué necesita una comprobación de la persona que está delante, deducido de su acción. Es una lista
cerrada:
- **ninguna**: no aparece ninguna ventana.
- **clic**: aparece el selector de certificado, el consentimiento o un error que hay que cerrar,
  solo como medio para llegar a lo que se mide. La suite enciende en esos trámites el
  **consentimiento automático** del cliente, que solo existe en un rFirma compilado para la suite
  y solo actúa si no queda nada que decidir.
- **persona**: la persona hace en el diálogo una acción con nombre, y lo que haga cambia lo que
  viaja.
Las comprobaciones de una tanda se corren en **tramos** por asistencia, en ese orden, y la cola se
detiene entre tramo y tramo hasta que la persona dice que está. Una comprobación `ninguna` que
agota su espera es un fallo de la suite, no del cliente: queda PENDIENTE.
_Avoid_: desatendida (para una comprobación), interactiva, manual

**Expectativa**:
Lo que una comprobación conducida espera de su trámite, declarado en el catálogo con un vocabulario
cerrado. Una sola de tres: un **código** (`SAF_NN`, cualquier SAF, `CANCEL`, `SAVE_OK`, `OK` o
`MEMORY_ERROR`); que el trámite **se complete**, con las **condiciones** de la sede que tenga que
cumplir y lo que tenga que traer lo que vuelve (un prefijo, un OID, unos bytes, una longitud); o el
**silencio**, que nadie responda. Un código de error —un SAF, CANCEL o MEMORY_ERROR— donde se
esperaba el trámite completo es NO CONFORME.
Lo que ninguna sede puede ver no tiene expectativa: la comprobación se declara **no medible**, con su
motivo, y no se conduce. Una comprobación nueva con una expectativa conocida no toca código.
_Avoid_: veredicto esperado, arnés (para cómo se juzga)

**Matriz de happy paths**:
Lo que responde si está confirmado el camino feliz de una operación: las **celdas** operación ×
formato × canal × clave que se exigen, agrupadas en planos que recorren dos ejes y dejan los demás en
el origen. Una combinación que ningún plano recorre no se exige. El catálogo no arranca si una celda
exigida queda sin estado o si la comprobación que la cubre no la mide.
_Avoid_: cobertura, tabla de happy paths

**Celda**:
Una combinación de la **matriz de happy paths** con uno de tres estados: **cubierta** por una sola
comprobación que se conduce en su canal y con su almacén, espera que el trámite se complete y, si
la operación devuelve firma, la condición que la verifica con la clave del certificado; **no
aplica**, con su motivo en el manual o en una ficha `BUG-NN`; o **hueco** declarado, lo que falta
por medir.
_Avoid_: caso, combinación

**Almacén**:
Dónde encuentra el cliente sus certificados en un trámite. Es una lista cerrada:
- **rsa**: un único certificado RSA de pruebas, sin PIN. Es el de omisión.
- **ec**: un único certificado de curva elíptica de pruebas, sin PIN.
- **token**: el token PKCS#11 de pruebas, con su PIN y varios certificados.
- **token_apart**: el mismo token sin registrar en el almacén del sistema, que tiene otro
  certificado: distingue el almacén que la sede nombra por su biblioteca del sistema.
- **ed25519**: un token propio sin registrar, con su PIN y un solo certificado, cuya clave es
  Ed25519: un tipo de clave con el que no se compone ningún algoritmo de firma.
- **several**: varios certificados de pruebas sin PIN, para los filtros, el almacén que nombra la
  sede y la fijación; el token queda alcanzable por su biblioteca, sin registrar.
- **expired**: uno vigente y uno caducado, sin PIN, para ver qué oculta la selección.
Es condición de lanzamiento: la suite prepara un perfil aislado por almacén, igual para cualquier
cliente.
_Avoid_: keystore, perfil (para el almacén)

**Conjunto**:
Las comprobaciones que miden decisiones del mismo componente del protocolo: un canal, el
analizador de la petición o una operación, con el capítulo del manual que lo recoge. Cada
comprobación tiene **un solo** conjunto, el del componente que decide; el código SAF o la versión
que observa nunca deciden dónde va.
_Avoid_: grupo, suite (para un conjunto)

**Punto de decisión**:
Dónde decide el original un código SAF: el **analizador** de la petición, el **canal** que la trae,
la operación **antes del certificado** (o sin pedir ninguno, como guardar y cargar), el **almacén**
al elegirlo, la firma **después del certificado**, o un servicio remoto: el **prefirmador** o el
**postfirmador** del lote y el **servidor trifásico**. Es una lista cerrada. Un mismo código puede
decidirse en varios; un NO CONFORME de SAF suele venir de dónde se decide, no de qué código es.
_Avoid_: origen, capa (para un punto de decisión)

**Tabla SAF**:
Cada código SAF que la sede puede recibir, cruzado con cada punto de decisión desde el que el
original lo emite. Cada fila está **cubierta** por comprobaciones que lo miden, es **no medible**
con su motivo, o es un **hueco** declarado, que nadie mide todavía. Es una vista sobre el catálogo,
no un conjunto: una comprobación no cambia de conjunto por estar en una fila, y toda la que espera
un `SAF_NN` está en una sola.
_Avoid_: matriz de errores, cobertura SAF (para la tabla)

**Resultado**:
El juicio de una comprobación, siempre respecto a lo que exige el protocolo y nunca respecto a lo
que se sabe del cliente. Es una lista cerrada:
- **CONFORME**: se comporta como exige el protocolo.
- **NO CONFORME**: no se comporta así, también cuando lo explica una **etiqueta**, que no cambia
  el resultado.
- **NO OBSERVABLE**: no se puede saber, porque la sede no llega a verlo o porque aún no se ha
  averiguado cómo medirlo. Lo que el instrumento no puede ver es NO OBSERVABLE para cualquier
  cliente.
- **PENDIENTE**: aún no se ha medido.
_Avoid_: veredicto, estado, SORPRESA, coincide, sin medida, verde

### Informes

**Informe**:
Lo que deja una tanda de comprobaciones contra un cliente: sus resultados, qué pasó y el registro
de cada una. Se ve igual lo esté corriendo o no, y desde cualquier ventana; solo se continúa con el
mismo cliente y la misma versión con que se creó.
_Avoid_: expediente, tanda (para el informe), dossier

**Huérfana**:
Una comprobación que el **informe** guarda y el catálogo ya no tiene, casi siempre porque se
renombró. Se ve aparte, no cuenta en ningún recuento y se poda la próxima vez que se escribe el
informe.
_Avoid_: obsoleta, retirada, fantasma

**Registro**:
Las líneas que deja una comprobación al correr, marcadas por quién las emite: la **sede**, el
**cliente** o la **suite**.
_Avoid_: conductor, sujeto, arnés

### Validación de la suite

**Etiqueta**:
Lo que puede explicar el NO CONFORME de una comprobación, declarado en ella y enseñado igual en
cualquier informe, sea del cliente que sea. Es una lista cerrada, y cada una tiene un origen que se
comprueba:
- **autofirma:bug:1.9.2**: un **bug conocido** de AutoFirma 1.9.2, con su ficha `BUG-NN`.
- **autofirma:bug:master**: ese bug sigue en `master`; se deduce del registro, no se escribe.
- **rfirma:adr-NNNN**: una **desviación deliberada**, con el ADR que la decide y cita la comprobación.
- **manual:deprecated**: un **formato deprecado**.
No cambia el resultado. Un NO CONFORME con cualquier etiqueta cuenta como **explicado**, y sin
ninguna, **sin explicar**: esa es la cifra que mira quien ejecuta. El límite se acepta a sabiendas:
la suite no sabe qué cliente mide, así que no dice para cuál vale cada etiqueta, y con AutoFirma
una `rfirma:` da por explicado un fallo que para ese cliente no lo está. Quien ejecuta sabe qué ha
lanzado y lee la que le toca; lo que sí es por cliente es la **validación**.
_Avoid_: marca, fallo esperado, causa (fuera de la referencia)

**Bug conocido**:
Una ficha `BUG-NN` del anexo A1 por la que AutoFirma 1.9.2 incumple lo que exige una comprobación.
La comprobación lo declara con la etiqueta `autofirma:bug:1.9.2`, y el registro
`bugs/autofirma-1.9.2.toml` guarda su título y si sigue en `master`, corregido o corregido a
medias; si sigue, la comprobación lleva además `autofirma:bug:master`.
_Avoid_: fallo esperado (para el bug), causa (fuera de la referencia)

**Referencia**:
Lo que se sabe de cómo responde AutoFirma 1.9.2 por sus bugs: las comprobaciones cuyo resultado
explica una ficha `BUG-NN` del anexo A1, casi siempre porque sale NO CONFORME, cada una con su
resultado y su causa. Si sale NO CONFORME, su causa es el **bug conocido** que declara la
comprobación; una que sale CONFORME puede tener causa sin declarar bug. Es un dato de una versión
concreta de AutoFirma y vive aparte del catálogo. Solo la usa la **validación**.
_Avoid_: línea base, baseline, expectativa por perfil

**Validación**:
Cruzar un informe con la **referencia**, a petición. Un informe de AutoFirma 1.9.2 está
**validado** cuando no le queda ninguna comprobación medible PENDIENTE y cada resultado coincide
con la referencia: CONFORME, salvo las que ella lista. Cuando la suite está validada, sus
resultados sobre cualquier otro cliente se leen tal cual.
_Avoid_: verde, línea base

**Discrepancia**:
Un resultado de la tanda de referencia que no coincide con la referencia. Es un fallo de la suite o
de la referencia, que se investiga; nunca del cliente.
_Avoid_: sorpresa, regresión

**Desviación deliberada**:
Una exigencia en la que rFirma no hace lo que AutoFirma porque un ADR suyo lo decidió. La
comprobación lleva la etiqueta `rfirma:adr-NNNN` y el ADR cita su id: el NO CONFORME de rFirma
sale explicado, y el porqué lo cuenta el ADR.
_Avoid_: excepción, falso negativo

**Formato deprecado**:
Un formato que el manual de AutoFirma desaconseja y mantiene solo por retrocompatibilidad (CMS,
XMLDSig, ODF y OOXML), o un modo obsoleto dentro de un formato que AutoFirma marca como tal (la
XAdES explícita): AutoFirma lo soporta y rFirma no. La comprobación que lo mide lleva la etiqueta
`manual:deprecated`; su resultado no cambia, y su NO CONFORME cuenta como explicado.
_Avoid_: obsoleto, retirado
