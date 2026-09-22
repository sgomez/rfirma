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
comprobaciones pueden juzgar el mismo trámite.
_Avoid_: errand (en la interfaz), ejecución, corrida

**Familia de trámite**:
El camino por el que un guion llega al cliente: el eco v4 en crudo, el canal de servicio o una
operación de punta a punta por la sede publicada. La declara el guion, no la comprobación.

**Condición**:
Lo que un guion de la sede mide durante el trámite y emite con un nombre propio de ese guion, no
con el de una comprobación. La comprobación nombra la condición que espera; si no llega, su
resultado es NO OBSERVABLE.

**Manifiesto**:
Lo que la sede publica de sí misma: sus modos y sus guiones, cada uno con su sede, su familia de
trámite y sus condiciones, y lo que solo usa el banco de la aplicación. El catálogo se valida contra
él al arrancar.

**Saludo**:
La comprobación que abre una familia de trámite: si falla, no se corre ninguna comprobación de su
familia, sea del conjunto que sea. La familia de punta a punta tiene dos: uno sin ventana, que abre
el tramo `ninguna`, y una firma de verdad, que abre el tramo `clic`.

**Comprobación**:
Una entrada del catálogo: una exigencia del protocolo, cómo se provoca y cómo se juzga. Se
presenta con **qué se exige** —lo que debe pasar—, su **fuente** —dónde lo hace el código de
AutoFirma 1.9.2—, lo que hay que hacer **antes de empezar** o **durante la prueba**, lo que **te
preguntaremos** al acabar y, una vez corrida, **qué pasó** y su **resultado**. Declara su
**asistencia** y su **almacén**.
_Avoid_: check, entrada (en la interfaz), enunciado, cita, aviso, observación

**Asistencia**:
Qué necesita una comprobación de la persona que está delante. Es una lista cerrada:
- **ninguna**: no aparece ninguna ventana.
- **clic**: aparece el selector de certificado o el consentimiento, solo como medio para llegar a
  lo que se mide.
- **persona**: el diálogo es lo que se mide, o alguien tiene que juzgar lo que vio.
Las comprobaciones de una tanda se corren en **tramos** por asistencia, en ese orden, y la cola se
detiene entre tramo y tramo hasta que la persona dice que está. Una comprobación `ninguna` que
agota su espera es un fallo de la suite, no del cliente: queda PENDIENTE.
_Avoid_: desatendida (para una comprobación), interactiva, manual

**Expectativa**:
Lo que una comprobación conducida espera de su trámite, declarado en el catálogo con un vocabulario
cerrado. Por el cable, una sola de cuatro: un **código** (`SAF_NN`, cualquier SAF, `CANCEL`,
`SAVE_OK`, `OK` o `MEMORY_ERROR`); que el trámite **se complete**, con lo que tenga que traer lo que
vuelve (un prefijo, un OID, unos bytes, una longitud); una **condición** de la sede; o que **nadie
responda**. Si lo que se mide es un diálogo, la **persona** dice qué vio y qué resultado sostiene su
sí. Una comprobación nueva con una expectativa conocida no toca código.
_Avoid_: veredicto esperado, arnés (para cómo se juzga)

**Almacén**:
Dónde encuentra el cliente sus certificados en un trámite. Es una lista cerrada:
- **rsa**: un único certificado RSA de pruebas, sin PIN. Es el de omisión.
- **ec**: un único certificado de curva elíptica de pruebas, sin PIN.
- **token**: el token PKCS#11 de pruebas, con su PIN y varios certificados.
Es condición de lanzamiento: la suite prepara un perfil aislado por almacén, igual para cualquier
cliente.
_Avoid_: keystore, perfil (para el almacén)

**Conjunto**:
Las comprobaciones que miden decisiones del mismo componente del protocolo: un canal, el
analizador de la petición o una operación, con el capítulo del manual que lo recoge. Cada
comprobación tiene **un solo** conjunto, el del componente que decide; el código SAF o la versión
que observa nunca deciden dónde va.
_Avoid_: grupo, suite (para un conjunto)

**Resultado**:
El juicio de una comprobación, siempre respecto a lo que exige el protocolo y nunca respecto a lo
que se sabe del cliente. Es una lista cerrada:
- **CONFORME**: se comporta como exige el protocolo.
- **NO CONFORME**: no se comporta así, incluido cuando es AutoFirma quien falla por un bug suyo.
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

**Registro**:
Las líneas que deja una comprobación al correr, marcadas por quién las emite: la **sede**, el
**cliente** o la **suite**.
_Avoid_: conductor, sujeto, arnés

### Validación de la suite

**Referencia**:
Lo que se sabe de cómo responde AutoFirma 1.9.2 por sus bugs: las comprobaciones cuyo resultado
explica una ficha `BUG-NN` del anexo A1, casi siempre porque sale NO CONFORME, cada una con su
resultado y su causa. Es un dato de una versión concreta de AutoFirma, no de las comprobaciones, y
vive aparte del catálogo. Solo la usa la **validación**.
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
Una exigencia en la que rFirma no hace lo que AutoFirma porque un ADR suyo lo decidió. La suite no
la conoce: sale NO CONFORME sin causa, y el porqué lo cuenta el ADR.
_Avoid_: excepción, falso negativo
