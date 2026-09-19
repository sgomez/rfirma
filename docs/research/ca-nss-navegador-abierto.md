# La CA local en NSS con el navegador abierto: medición

Medición del ticket [#653](https://github.com/sgomez/rfirma/issues/653) del mapa
[#652](https://github.com/sgomez/rfirma/issues/652). El [ADR-0005](../adr/0005-servidor-local-https-y-ca-en-los-almacenes-nss.md)
decide **qué** se escribe en los almacenes NSS y **cuándo**; lo que no dice en
ninguna parte es qué ocurre cuando el navegador tiene esa misma base abierta. La
consecuencia es de interfaz: si basta con escribir y avisar, es una línea de
texto; si hay que detectar el navegador vivo y pedir que lo cierre, es una
pantalla con reintento, en el asistente y en la retirada.

Esto es **la medición**, no un repaso de documentación.

Entorno: Ubuntu (kernel 7.0.0), NSS **3.120**, Firefox **155.0.1** del paquete
del sistema, Google Chrome **152.0.7977.64**, Flatpak con
`org.freedesktop.Platform//25.08`. Todos los perfiles del banco de pruebas son
desechables y viven en `/tmp`: **el perfil real del titular no se ha tocado en
ningún paso**, ni para leer. La CA del banco es una raíz de laboratorio generada
para la ocasión, con un certificado de servidor para `localhost`/`127.0.0.1` y un
servidor HTTPS en `127.0.0.1:8443` que devuelve una página reconocible. El
veredicto de cada paso lo da el **propio navegador vivo**, conducido por
Marionette (Firefox) y por CDP (Chrome), nunca `certutil -L`.

## Veredicto

**La escritura nunca falla ni se pierde: entra siempre, con el navegador vivo,
por la vía exacta de rFirma y también desde dentro del sandbox.** Lo que cambia
según el navegador es *cuándo se ve*, y la asimetría importante es esta:
**instalar se ve casi siempre en caliente; retirar no se ve nunca hasta que el
navegador se reinicia.**

| Operación, con el navegador vivo | Firefox 155 | Chrome 152 |
|---|---|---|
| ¿La escritura da error? | No, `exit 0` | No, `exit 0` |
| ¿Queda en `cert9.db`, visible a otro proceso? | Sí, al instante | Sí, al instante |
| **Instalar**, sitio que **aún no** ha fallado en esta sesión | **Se ve, sin reiniciar** | **Se ve, sin reiniciar** |
| **Instalar**, sitio que **ya** ha fallado en esta sesión | **NO se ve** hasta reiniciar | **Se ve, sin reiniciar** (t+0 s) |
| **Retirar**, sitio ya visitado | **NO se ve** hasta reiniciar | **NO se ve** hasta reiniciar |
| **Retirar**, origen nunca visitado | **NO se ve** hasta reiniciar | Se ve, sin reiniciar |
| Tras reiniciar el navegador | Coincide siempre con el disco | Coincide siempre con el disco |

Las tres casillas que mandan en la interfaz son las de la columna de Firefox:
instalar tras un fallo ya visto, y retirar en cualquier caso.

### Lo que esto corrige del ADR-0005

El ADR afirma hoy que «**Chrome nunca relee su `nssdb` en caliente**». **Medido,
es falso en Chrome 152**, y de la forma más favorable posible: en la misma
instancia viva, sobre `https://localhost:8443/` que acababa de dar
`ERR_CERT_AUTHORITY_INVALID`, la primera navegación posterior a la escritura ya
carga la página. Repetido en dos ejecuciones independientes, y con muestreo a
t+0, +3, +6 y +9 s: **confía desde la primera medición**, así que no es un
temporizador que se cruce por suerte.

Lo que el ADR sí acierta es Firefox, y por el motivo que cita: la caché de
verificación envenenada. Aquí queda separado el mecanismo, que el ADR no
separaba:

* **La base se relee.** Un Firefox vivo que **no** haya verificado todavía nada
  contra esa CA la encuentra en cuanto la necesita. Instalado con el navegador ya
  abierto y sin haber visitado el sitio, la primera visita **confía**.
* **Lo que no se invalida es lo ya resuelto.** Un certificado que Firefox ya
  cargó en su caché en memoria se queda con el veredicto que tuviera, y ni la
  instalación ni la retirada posterior lo mueven. Por eso la retirada no se ve
  ni siquiera en un **origen nuevo** (`https://127.0.0.1:8443/`, jamás visitado):
  el certificado de la CA ya estaba en memoria, y ahí sigue.

Chrome se comporta igual pero con una caché más estrecha: cachea el **resultado
por sitio**, no la CA, de modo que un origen nuevo tras la retirada sí deja de
confiar, y un origen ya resuelto no.

### La retirada es el caso peor, y es el que menos se espera

Merece decirse solo porque es contraintuitivo: **retirar la CA con el navegador
abierto no quita la confianza**. La base queda limpia —`certutil -L` ya no la
lista, `CERT_FindCertByDERCert` no la encuentra—, pero el navegador vivo sigue
aceptando el certificado hasta que se reinicia. Si la retirada se ofrece desde la
aplicación como «quitar esto de mi máquina», **la promesa no se cumple del todo
en el momento de pulsarla**, y la interfaz tiene que decirlo. Es la única de las
casillas donde callarse tiene consecuencia de seguridad, no de comodidad.

## La vía de rFirma, no la de `certutil`

El ADR-0005 registra la CA **por la API de NSS**, no por el binario. Para que la
medición valga hay que medir esa vía, así que la parte decisiva se repitió con la
secuencia exacta de `site/adapters/nss.rs` —`PK11_SetPasswordFunc` con una
función que devuelve nulo, `NSS_NoDB_Init(NULL)`, `SECMOD_OpenUserDB` con
`configDir='sql:<perfil>' certPrefix='' keyPrefix='' flags=readWrite`,
`CERT_NewTempCertificate`, `PK11_ImportCert`, `CERT_ChangeCertTrust`— contra un
perfil que Firefox tenía abierto en ese instante:

```
  .parentlock: BLOQUEADO por pid <PID> (visible=True)
  SECMOD_OpenUserDB: OK
  CERT_NewTempCertificate: OK
  PK11_ImportCert: OK
  CERT_ChangeCertTrust: OK
  relectura: presente, sslFlags=0x0018, bits de CA de confianza=True
```

`SECMOD_OpenUserDB` con `flags=readWrite` **no falla por que el navegador tenga
la base abierta**, y `certutil -L` desde fuera lista la CA con `C,,`. No hay
`SEC_ERROR_LEGACY_DATABASE`, ni base bloqueada, ni escritura silenciosamente
descartada. La retirada por la misma vía (`CERT_FindCertByDERCert` +
`SEC_DeletePermCertificate`) también devuelve `OK` con el navegador vivo.

## Bajo el sandbox del flatpak

Repetida la misma secuencia **desde dentro de un sandbox de Flatpak**, con
`--filesystem` sobre el directorio del perfil y el Firefox del anfitrión vivo
sobre ese mismo perfil:

```
  .parentlock: BLOQUEADO por pid 0 (visible=False)
  procesos visibles en /proc: 3
  SECMOD_OpenUserDB: OK
  PK11_ImportCert: OK
  CERT_ChangeCertTrust: OK
  relectura: presente, sslFlags=0x0018, bits de CA de confianza=True
```

Y desde el anfitrión, después: `certutil -L` lista `rFirma CA local  C,,`. **El
sandbox no cambia nada de lo anterior**: es un bind mount al mismo inodo, el
proceso es del mismo usuario, y el bloqueo POSIX y el de SQLite cruzan la
frontera sin enterarse. Confirma por otra vía lo que `rutas-de-firefox-y-nss.md`
§5.1 ya midió para la lectura.

Dos avisos que sí salen de aquí:

* **`/proc` dentro del sandbox solo ve el propio sandbox** (3 procesos frente a
  452 en el anfitrión). Flatpak no comparte el espacio de nombres de PID, así que
  **cualquier detección basada en buscar el proceso del navegador es imposible
  desde dentro**: no es cuestión de permisos, es que los procesos del anfitrión
  no existen ahí.
* El programa de prueba terminó con **SIGSEGV en el desmontaje** (`exit=139`)
  dentro del sandbox y con `exit=0` en el anfitrión, siempre **después** de que
  el trabajo estuviera hecho y releído. Es un arnés de `ctypes`, no el código de
  rFirma, así que no se afirma nada sobre este: queda como cosa a mirar si el
  backend empieza a caerse al cerrar NSS bajo flatpak.

## Detectar el navegador vivo sin preguntar a nadie

Medido fichero a fichero, con el navegador arriba y luego cerrado:

| Señal | ¿Sirve? |
|---|---|
| Bloqueo POSIX o `flock` sobre `cert9.db` | **No.** Libre en todo momento, con Firefox y con Chrome vivos: SQLite solo toma el bloqueo durante la transacción |
| `<perfil>/lock` (enlace simbólico a `<IP>:+<PID>`) | **No.** **Sobrevive al cierre**: sigue ahí, apuntando a un PID muerto. Falso positivo garantizado |
| `<perfil>/.parentlock` | **Sí.** `F_GETLK` devuelve `F_WRLCK` con Firefox vivo y `F_UNLCK` en cuanto cierra |
| Buscar el descriptor abierto en `/proc/*/fd` | Sirve en el anfitrión; **inservible bajo flatpak** (sin espacio de nombres de PID compartido) |
| `SingletonLock` del perfil de Chrome (`<host>-<PID>`) | Fuera de alcance: vive en `~/.config/google-chrome`, que el manifiesto no pide y no debe pedir |

**Para Firefox hay detector, y es barato:** `F_GETLK` sobre
`<perfil>/.parentlock`. No pide un permiso nuevo —el manifiesto ya tiene rw sobre
esos directorios—, no lee `/proc`, no lanza procesos, y **funciona igual dentro
del sandbox**: con el navegador vivo devuelve `WRLCK`, y con el navegador cerrado
devuelve `UNLCK`, comprobado en las dos caras de la frontera.

Con una condición que hay que respetar al programarlo: **la decisión se toma por
el tipo de bloqueo, nunca por el PID**. Desde dentro del sandbox el dueño del
bloqueo está en otro espacio de nombres y el núcleo devuelve `pid=0`, que no es
«nadie», es «no te lo puedo decir».

**Para Chrome no hay detector**, y no conviene fabricarlo: la base
(`~/.local/share/pki/nssdb`, la que Chrome 152 crea por su cuenta —confirmado en
esta medición— y de la que ya hablaba `ca-en-los-almacenes-de-confianza.md`) no
tiene fichero de bloqueo alguno. Da igual: Chrome es justo el navegador que **sí**
relee en caliente al instalar, que es el único momento en el que la aplicación
necesitaría avisar. **No hace falta detectarlo.**

## `cert8.db`

**No se mide, porque la pregunta ya estaba decidida.**
`rutas-de-firefox-y-nss.md` §6 la cierra: el formato antiguo *dbm* no se soporta,
porque el runtime del flatpak no trae `libnssdbm3.so` y aceptarlo «solo cambiaría
un descarte silencioso por un fallo al abrir». El descubrimiento de perfiles ya
exige `cert9.db`, y el ADR-0005 obliga al prefijo `sql:` explícito precisamente
para que la operación no caiga en el backend antiguo, donde falla en silencio
devolviendo éxito. Firefox 155 tampoco crea ni lee `cert8.db` en un perfil nuevo.
Ningún navegador que rFirma alcance usa ese formato, así que **la respuesta no
cambia porque el caso no existe**.

## Restos al retirar

Con el navegador cerrado, la retirada por la API deja la base limpia: el
certificado no se encuentra por DER y `certutil -L` no lo lista. En el perfil no
aparece ningún fichero nuevo ni queda rastro fuera de `cert9.db`.

Lo que **no** puede limpiar la aplicación por sí sola, medido o razonado a partir
de lo medido:

1. **La sesión viva del navegador.** Ya está dicho: hasta que se reinicie, sigue
   confiando. Es el resto principal y el único que la persona nota.
2. **Las excepciones de certificado que la persona haya añadido a mano**
   (`cert_override.txt` en Firefox). Si alguien pulsó «acepto el riesgo» antes de
   que la CA estuviera instalada, esa excepción es suya, está fuera de la base y
   sobrevive a la retirada. La aplicación no debe tocarla.
3. **Las bases de perfiles que la aplicación no vea en ese momento**: un perfil
   creado después, un Firefox de flatpak (`~/.var/app/...`, fuera de alcance por
   decisión), o un `HOME` distinto. La retirada limpia lo que el descubrimiento
   encuentre, y ni un perfil más.
4. **Instalaciones anteriores con otro apodo o de otra versión.** El ADR ya obliga
   a borrar **por huella y no por apodo**, y a llevarse las dos CA del solape; el
   residuo medido de AutoFirma en `ca-en-los-almacenes-de-confianza.md` §5 es el
   ejemplo de lo que pasa cuando se borra por apodo.

## Lo que esto significa para la interfaz

* **Instalar (asistente del primer arranque): basta con escribir y avisar.** No
  hace falta detectar nada, ni pedir que se cierre nada, ni una pantalla con
  reintento. El caso que falla —Firefox vivo que ya vio el error— es exactamente
  el que el ADR ya evita al instalar en el primer arranque y no a mitad de
  trámite. Una línea de texto, y que diga «reinicia el navegador» **solo** si de
  verdad se ha instalado algo, como ya hace `PendingNotice::after_installing()`.
* **Reparar desde el panel de estado: ahí sí conviene detectar.** «Volver a
  instalar la CA» se pulsa justo después de haber visto fallar algo, que es la
  casilla mala de Firefox. Con `F_GETLK` sobre `.parentlock` la aplicación puede
  decir, sin preguntar y sin falsos positivos, «hecho; reinicia Firefox para que
  surta efecto» en lugar de un «hecho» que la persona va a desmentir en tres
  segundos.
* **Retirar: la advertencia es obligatoria, y no depende de detectar nada.**
  Ningún navegador deja de confiar hasta reiniciarse, esté o no abierto en ese
  instante. El texto de la retirada tiene que decir que la confianza no
  desaparece hasta que se cierren los navegadores, y decirlo siempre.
* **Y sigue en pie lo que el ADR ya dice: rFirma no mata el navegador de nadie.**
  Nada de lo medido justifica un `pkill`, ni pedir permiso para reiniciar nada.

## Cómo reproducirlo

El banco de pruebas no se versiona: son cinco guiones desechables en `/tmp`
—servidor HTTPS con la CA de laboratorio, cliente Marionette, cliente CDP, sonda
de bloqueos y el arnés de `ctypes` sobre `libnss3`—. Lo que hay que conservar es
el **método**, porque es lo que hace la medición creíble y lo que habría que
repetir cuando cambie la versión de un navegador:

1. Perfil desechable en `/tmp`, nunca el real.
2. CA de laboratorio + certificado para `localhost` + servidor HTTPS local con
   una cadena reconocible en el cuerpo de la página.
3. El navegador se arranca **una vez** y se conduce vivo (`-marionette` /
   `--remote-debugging-port`); el veredicto es qué renderiza él, no lo que diga
   `certutil -L`.
4. Cada caso se prueba en las dos variantes que resultaron distinguir el
   mecanismo: **origen ya visitado** y **origen nunca visitado**. Sin esa
   distinción, Firefox y Chrome parecen comportarse igual y no lo hacen.
