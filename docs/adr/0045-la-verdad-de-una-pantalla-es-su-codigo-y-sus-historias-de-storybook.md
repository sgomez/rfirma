# La verdad de una pantalla implementada es su código y sus historias de Storybook

Sustituye al [ADR-0033](0033-la-interfaz-se-prototipa-en-claude-design-y-manda-el-repositorio.md), que
queda marcado como sustituido por este.

El repositorio manda y **Claude Design sirve para explorar**, no para guardar la decisión: eso
sigue valiendo del ADR-0033. Lo que cambia es dónde vive una pantalla ya implementada. La copia
dibujada a mano (los artboards) se quedaba atrás cada vez que cambiaba el código, y rehacerla era
rehacer el dibujo.

- **La verdad de una pantalla implementada son su código y sus historias de Storybook.** Cada
  componente y cada pantalla-estado tiene su historia, junto a su componente (`*.stories.tsx`,
  CSF3), pintada con el catálogo i18n real, en los cinco idiomas y en claro y oscuro. Una historia
  por variante, no solo por estado.
- **El repositorio no guarda copia de artboards.** Cada pantalla migrada borra el suyo; al terminar
  la migración desaparecen también sus parciales y sus scripts.
- **Las fichas de `docs/design/` cuentan el flujo entre estados y el porqué de cada decisión**, y
  citan historias y claves i18n en lugar de copiar textos.
- **El sistema de diseño es código fuente normal**: sin sello del bundle. Claude Design lo recibe
  compilado con `/design-sync` desde Storybook, y se explora allí con los componentes reales. Los
  proyectos anteriores de Claude Design quedan congelados como histórico.

## Las historias son entrada de las pruebas

- Una única prueba de vitest, en jsdom, compone todas las historias: comprueba que se pintan sin
  errores y que axe no encuentra fallos estructurales. El contraste queda fuera, porque jsdom no lo
  mide. Una historia nueva queda cubierta sin escribir ningún test.
- El contraste lo revisa `just storybook-a11y`, bajo demanda y en navegador, sobre todas las
  historias en claro y en oscuro. No entra en el CI: la aplicación corre en WebKitGTK y WebView2, así
  que esa revisión comprueba reglas, no el motor real.
- Historias y tests comparten los dobles de los puertos, sin importar vitest: los espías son los de
  `storybook/test`. Las historias no simulan Tauri en ningún sitio.
