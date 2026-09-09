# Prompt para Claude Design: landing de rFirma

Pégalo tal cual en el proyecto nuevo de Claude Design, con el sistema de diseño
«rFirma Design System» adjunto y los ficheros de referencia subidos. Después
de la primera versión, las correcciones van en tandas numeradas sobre el mismo
artboard, nunca en artboards nuevos.

---

Diseña la landing pública de **rFirma**, una aplicación de escritorio de firma
electrónica para la Administración española, alternativa independiente y de
código abierto a AutoFirma. Partes de tres referencias adjuntas:

- `index.html`: la landing actual. Reutiliza su contenido (secciones, textos,
  órdenes de instalación, enlaces, huella GPG) salvo lo que este prompt cambia.
- `rfirma-main-window.png`: captura real de la ventana principal. Solo para el
  hero.
- `artboards/*.dc.html`: las pantallas reales de la app, ya construidas con el
  sistema de diseño. Son la fuente de las maquetas de la demo; no uses capturas
  para ella.

## Sistema de diseño

Usa los foundations del proyecto «rFirma Design System» adjunto: tokens
`--rf-*`, tipografía Inter, radios, escala de espaciado, sombras y los
componentes `.rf-btn`, `.rf-card`, `.rf-badge`. **Solo tema claro.**

Única desviación permitida: el acento es el verde azulado de marca del icono,
`#067781`, con su familia ya definida en `index.html` (`--rf-brand`,
`--rf-brand-surface #f0fdfa`, `--rf-brand-border #99f6e4`,
`--rf-brand-text #0f766e`), en lugar del primario gris del sistema. Se usa con
medida: acción principal, iconos, subrayados, la insignia y un solo degradado de
fondo muy suave. Todo lo demás sigue monocromo, como manda el sistema.

## Tono

Sobrio, estilo Apple. Mucho aire, titulares grandes y cortos, una idea por
sección, frases sin adjetivos vacíos. **Ninguna cifra que no esté medida**: la
comparativa se hace solo con hechos. Nada de emojis como iconos: iconos de
línea simples y monocromos.

De la referencia visual adjunta (`referencia.jpg`) toma solo la **estructura**:
hero asimétrico con texto a la izquierda y tarjetas flotantes a la derecha,
secciones alternas texto/maqueta, fondos orgánicos. La ejecución es contenida:
un degradado teal muy suave, sin manchas de color ni arcoíris.

## Estructura de la página

1. **Franja de aviso fija superior**, fina, no descartable, encima de la
   navegación: «rFirma no es un producto oficial de la Administración Pública.
   Es un proyecto independiente de código abierto bajo licencia EUPL 1.2».
   Se repite en el pie. Este aviso no puede perderse en ningún punto del scroll.
2. **Navegación**: logo (SVG de `index.html`), insignia «Alfa», enlaces a las
   secciones, selector de idioma (castellano, català, euskara, galego, English;
   solo el castellano tiene contenido en este prototipo) y botón GitHub.
3. **Hero**: a la izquierda claim, subtítulo y dos acciones (primaria
   «Instalar», secundaria «Ver diferencias con AutoFirma»). A la derecha la
   captura real dentro de un marco con sombra elevada y dos o tres tarjetas
   flotantes pequeñas que resuman capacidades: certificado seleccionado, firma
   visible colocada, formato PAdES.
4. **«Así se firma»**, sección guiada por scroll: la maqueta de la app queda
   fija (sticky) mientras el texto lateral avanza, y en cada paso la maqueta
   cambia de estado con una transición suave. Secuencia, reconstruida desde
   los artboards indicados:
   1. Documento cargado: `EstadoDocumentoCargado.dc.html`.
   2. Elegir certificado: `EstadoElegirCertificado.dc.html`.
   3. Introducir PIN: `EstadoPin.dc.html`.
   4. Firmando: `EstadoFirmando.dc.html`.
   5. Firma completada: `EstadoExito.dc.html`.
   Animaciones de opacidad, desplazamiento corto y escala leve, con
   `prefers-reduced-motion` respetado (sin movimiento, solo cambio de estado).
5. **Cuatro pilares** en tarjetas con icono: rendimiento nativo (Rust + React,
   sin JVM), la clave privada nunca sale del sistema (almacenes del sistema y
   PKCS#11), criptografía oficial de clienteafirma compilada con GraalVM, firma
   visible arrastrando la rúbrica con previsualización fiel.
6. **Comparativa AutoFirma / rFirma** solo con hechos verificables:
   arquitectura, dónde se procesa la clave privada, colocación de la firma
   visible, canal de actualización, integración con el escritorio. Sin
   tiempos de arranque ni consumo de memoria.
7. **Instalación** con pestañas Flatpak, APT, DNF, Windows y macOS. Las dos
   últimas marcadas «En desarrollo» con el texto de `index.html`. Bloques de
   código con botón de copiar y las órdenes de `index.html` tal cual. Encima,
   el aviso de que los paquetes aún no están publicados, rediseñado como
   tarjeta discreta pero visible.
8. **Transparencia**: software libre bajo EUPL 1.2, los dos repositorios
   (`sgomez/rfirma` y `ctt-gob-es/clienteafirma`), la clave GPG y su huella
   con la orden para comprobarla.
9. **Pie**: logo, licencia, enlaces (GitHub, releases, clienteafirma, clave
   GPG) y el aviso de no oficialidad repetido.

## Idiomas

Diseña **solo en castellano**. El selector de idioma de la navegación existe
como componente pero no traduce nada: las cinco versiones se generarán después
fuera de esta herramienta. Deja todos los textos visibles en un único
diccionario JavaScript con claves en inglés (`hero.title`, `pillars.native.body`)
para que la traducción posterior sea mecánica.

## Entregable técnico

Un único fichero HTML con CSS y JS inline, sin frameworks ni dependencias
externas salvo la fuente Inter. Responsive desde 360 px. Accesible: contraste
AA, foco visible, área táctil mínima de 44 px, pestañas con roles ARIA. Sin
tema oscuro.

Se portará a **Astro** como sitio estático, así que organiza el fichero por
secciones claramente delimitadas y con nombres estables (un comentario
`<!-- section: hero -->` por sección) y evita estado global en el JS: cada
comportamiento (pestañas, copiar, demo con scroll, selector de idioma) en su
propia función aislada.
