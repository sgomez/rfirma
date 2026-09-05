# Pruebas manuales del protocolo `afirma://`

La segunda puerta manual del [ADR-0014](adr/0014-gradas-de-prueba-y-puerta-de-calidad.md),
junto al validador oficial. Se ejecuta **una vez por etiqueta `v*`**, por una persona.

## Qué entra aquí, y qué no

> Entra en la lista **sólo** lo que necesita un navegador de verdad o una sede de verdad, y por
> tanto no lo puede tener el CI. Si una comprobación cabe en la grada A, B o C, va ahí, **aunque
> sea incómoda**. Añadir una fila obliga a justificar **en la propia fila** por qué no cabe en
> ninguna de las tres.

Todo lo que sí corre solo está en las gradas: el códec y la conversación en `src/protocol/`, el
**cliente de canal** —el propio, en Rust— en `tests/channel_client.rs` (puerto `:0`), el **banco
de conformidad** —el `autoscript.js` publicado del tag `v1.9.2`, bajo Node— en
`tests/conformance_bench.rs`, y el paseo completo en `tests/native_cycle.rs` (grada C).

## La lista

### 1. La CA local, vista por los dos navegadores

Con la CA local recién instalada, abrir el servidor local en **Firefox** y en **Chrome**: la
página carga **sin aviso** en los dos.

*Por qué no cabe en una grada*: los bits de confianza del `nssdb` no bastan de oráculo. El
[#326](https://github.com/sgomez/rfirma/issues/326) midió que `vfychain` da el veredicto
**invertido** respecto a Firefox, así que sólo el navegador de verdad responde.

### 2. El permiso de red local

Conceder el permiso desde una sede y comprobar que el trámite sigue. Después **denegarlo**, y
comprobar qué enseña rfirma.

*Por qué no cabe en una grada*: el permiso lo concede una persona en una interfaz del navegador,
y el [#309](https://github.com/sgomez/rfirma/issues/309) midió que **rfirma no puede
diagnosticar la denegación** —ocurre entera dentro del navegador—.

### 3. El `afirma://` disparado desde una página, en los tres canales

Desde una página que lance `afirma://websocket?…`, comprobar que rfirma arranca y recibe la URL
entera en **flatpak**, en **`.deb`** y en **`.rpm`**. Incluye la convivencia con AutoFirma
instalada ([#325](https://github.com/sgomez/rfirma/issues/325)): qué aplicación gana.

Con **dos manejadores del esquema instalados** —rfirma y AutoFirma— se ensaya además la elección
explícita ([#358](https://github.com/sgomez/rfirma/issues/358)): con la entrada
`x-scheme-handler/afirma=rfirma.desktop;` escrita por rfirma en `[Default Applications]` del
`~/.config/mimeapps.list`, la invocación va **directa a rfirma y no sale el selector**, ni en esta
ni en las siguientes; **borrando esa entrada** del fichero, el selector vuelve a salir en cada
invocación. Si en `~/.config/` hay un `gnome-mimeapps.list` con una entrada para el esquema, manda
él y el ensayo mide otra cosa: hay que mirarlo antes.

*Por qué no cabe en una grada*: es el registro `x-scheme-handler` del escritorio sobre un paquete
instalado, no código. Que la entrada quede escrita donde el escritorio la lee sí lo miden las
gradas A y B (`src/desktop/choice.rs`); lo que ninguna puede medir es la conducta del escritorio
al leerla (TD-65).

### 4. Un trámite completo contra una sede real

`selectcert` y después `sign` contra una sede electrónica en producción, con la firma volviendo a
la sede y la sede dándola por buena.

*Por qué no cabe en una grada*: es la grada D llevada al extremo —un tercero que puede estar
caído— y además es el único sitio donde se comprueba que nuestra lectura de `autoscript.js` y la
sede coinciden de verdad.

### 5. La cancelación

Cancelar desde la ventana del trámite y comprobar que la sede recibe `err-11:=AF500001` y lo
trata como cancelación, no como error.

*Por qué no cabe en una grada, hoy*: el formato lo comprueban los fixtures de la grada A, y que
el cliente lo interprete como cancelación **sí** lo puede medir ya el banco de conformidad. Lo
que falta es el otro extremo: hasta que exista el trámite que se cancela desde la ventana
([#362](https://github.com/sgomez/rfirma/issues/362),
[#363](https://github.com/sgomez/rfirma/issues/363)) no hay nada que cancelar. **Es la próxima
fila que sale de esta lista**, y sale al banco, no a otra grada.

## Condición de salida

Esta lista **no es permanente**. El día que exista un arnés de navegador sin cabeza capaz de
**conceder el permiso de red local** y de **sembrar el `nssdb`** de forma reproducible, las filas
1 y 2 bajan a una grada nueva. La 5 no espera a eso: baja al **banco de conformidad** en cuanto
haya un trámite que cancelar. Este fichero se queda entonces con las que necesitan una sede de
verdad —la 3 y la 4—. Está escrito aquí y no en el ADR porque es estado de la lista, y lo lee
quien la ejecuta.
