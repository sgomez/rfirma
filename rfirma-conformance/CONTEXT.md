# Suite de conformidad

Glosario de la suite de conformidad, un bounded context aparte del de la aplicación rFirma: aquí
las palabras significan lo que tienen sentido para quien mide, aunque en `CONTEXT.md` de la raíz
signifiquen otra cosa. Define **qué es** cada término; lo decidido vive en los ADR (`CONTEXT-MAP.md`).

## Language

### Qué se mide

**Suite de conformidad**:
El catálogo de exigencias del protocolo `afirma://` y lo que las mide contra el **cliente** que se
le configure. Produce un **informe**, no una puerta: no bloquea ningún PR ni ninguna etiqueta
(ADR-0014) y se ejecuta en local, fuera del CI. **El protocolo es lo que responde AutoFirma**: cada
exigencia dice lo que AutoFirma responde, salvo donde AutoFirma se contradice a sí mismo, y
entonces dice lo que debe responder. **La suite no sabe qué cliente mide**: ni corre comprobaciones
distintas ni dice cosas distintas según quién responda.
_Avoid_: banco de conformidad, sondeo, arnés

**Cliente**:
La aplicación a prueba, AutoFirma o rFirma: un binario instalado con su raíz de confianza. Saber
cuál es sirve solo para lanzarlo, nunca para juzgarlo.
_Avoid_: sujeto, perfil, aplicación a prueba

**Sede**:
El lado web del protocolo, que la suite interpreta con el `autoscript.js` publicado de AutoFirma
1.9.2 corriendo bajo Node: invoca al cliente y habla con él como lo haría una sede electrónica.
_Avoid_: conductor, driver, cliente publicado

**Comprobación**:
Una entrada del catálogo: una exigencia del protocolo, cómo se provoca y cómo se juzga. Se
presenta con **qué se exige** —lo que debe pasar—, su **fuente** —dónde lo hace el código de
AutoFirma 1.9.2—, lo que hay que hacer **antes de empezar** o **durante la prueba**, lo que **te
preguntaremos** al acabar y, una vez corrida, **qué pasó** y su **resultado**.
_Avoid_: check, entrada (en la interfaz), enunciado, cita, aviso, observación

**Conjunto**:
Un grupo de comprobaciones sobre la misma parte del protocolo, con el capítulo del manual que las
recoge. Su **saludo** es la comprobación que abre el conjunto: si falla, el resto no se corre.
_Avoid_: grupo, suite (para un conjunto)

**Resultado**:
El juicio de una comprobación, siempre respecto a lo que exige el protocolo y nunca respecto a lo
que se sabe del cliente. Es una lista cerrada:
- **CONFORME**: se comporta como exige el protocolo.
- **NO CONFORME**: no se comporta así, incluido cuando es AutoFirma quien falla por un bug suyo.
- **NO OBSERVABLE**: no se puede saber, porque la sede no llega a verlo o porque aún no se ha
  averiguado cómo medirlo. Lo que el instrumento no puede ver es NO OBSERVABLE para cualquier
  cliente.
- **PENDIENTE**: aún no se ha corrido.
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
