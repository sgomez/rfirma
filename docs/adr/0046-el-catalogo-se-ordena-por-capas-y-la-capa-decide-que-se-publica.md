# El catálogo se ordena por capas constructivas y la capa decide qué se publica

Completa al [ADR-0045](0045-la-verdad-de-una-pantalla-es-su-codigo-y-sus-historias-de-storybook.md):
la verdad de una pantalla son su código y sus historias; este ADR decide cómo se ordenan esas
historias y cuáles llegan a Claude Design.

Hasta ahora las categorías de Storybook mezclaban tres criterios —por ventana, por dominio y por
pieza— y una historia solo llegaba a Claude Design si su componente estaba en
`design-sync.entry.ts` **y** su título en el `titleMap` de `.design-sync/config.json`: dos listas
manuales cuya intersección nadie decidía, con descarte silencioso para lo no mapeado. El proyecto
remoto recibía páginas enteras recortadas por la captura y le faltaban justo las piezas con las que
se diseña un recorrido nuevo.

- **El catálogo se ordena en cuatro capas constructivas** —primitivos, dominio, flujos y
  pantallas—, definidas en el glosario (`CONTEXT.md`, «Sistema constructivo de la interfaz»). El
  primer segmento del `title` de cada historia declara su capa; el segundo, su dominio en las capas
  publicables; la numeración de recorrido solo existe dentro de las pantallas.
- **La capa decide la publicación.** Primitivos, dominio y flujos se publican en Claude Design;
  las pantallas son referencia local. No se publican páginas: el kit es material para componer
  recorridos nuevos, no un archivo de pantalla ya decidida.
- **La selección se deriva, no se copia.** Una receta regenera la entrada y el `titleMap` desde
  los títulos, y se corre antes de cada `/design-sync`; el CI no la verifica, porque solo
  `/design-sync` consume esos ficheros. Lo que una tarjeta
  necesita saber de sí misma —viewport, modo de tarjeta, historia principal— se declara en su
  historia, y las tablas de catálogo de la cabecera que lee Claude Design se generan de la misma
  fuente. Un título sin mapear en una capa publicable es un rojo, no un descarte en silencio.

Se consideró adoptar Feature-Sliced Design, también como estructura de carpetas de `src/`, y
Atomic Design como vocabulario. Se descartaron: la propiedad que FSD impone con capas de
importación ya la dan los puertos y las guardas estructurales, y mover las zonas por contexto
costaría el renombre de todo lo que las cita; de Atomic Design, su frontera molécula/organismo es
justo el tipo de debate que una guarda no puede zanjar. Las capas son una convención del catálogo,
no de los imports.
