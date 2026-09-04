# Ventana principal

La única ventana de rFirma. Aloja el recorrido completo de firmar un PDF, de
abrir el documento a guardarlo firmado, sin navegar a ninguna otra pantalla.

## Casos de uso que la usan

- Firmar un PDF en local — de principio a fin.

## Estructura

Una cabecera común y debajo la bandeja, el visor y el panel de firma.

```
┌─────────────────────────────────────────────────────────┐
│ cabecera                                                │  56 px
├─────────────────────────────────────────────────────────┤
│ franja de notificación (sólo si hay algo que notificar)  │  41 px
├──────────────┬───────────────────────┬──────────────────┤
│ bandeja      │ visor                 │ panel de firma   │
│ 300 px       │ flexible              │ 360 px           │
└──────────────┴───────────────────────┴──────────────────┘
```

**Sin documento la ventana es de dos columnas.** El panel de firma no está
montado: no hay nada que firmar, y una columna vacía con su borde izquierdo
solo dice que falta algo. Es lo que dice el `oculto` del estado 1 de la tabla
de abajo y lo que enseña el artboard del estado vacío.

```
┌─────────────────────────────────────────────────────────┐
│ cabecera                                                │  56 px
├──────────────┬──────────────────────────────────────────┤
│ bandeja      │ visor                                    │
│ 300 px       │ flexible                                 │
└──────────────┴──────────────────────────────────────────┘
```

No es un `display: none` ni el primer paso hacia una segunda pantalla: la
región no se monta, y en cuanto hay documento vuelve. Las tres regiones siguen
sin aparecer ni desaparecer **durante** el recorrido, que empieza con el
documento abierto.

- [Cabecera](cabecera.md) — identidad, estado del documento y menú principal.
- [Bandeja de documentos](bandeja-de-documentos.md) — qué documento se firma.
- [Visor de documento](visor-de-documento.md) — cómo va a quedar.
- [Panel de firma](panel-de-firma.md) — con qué y cómo se firma, y el botón que firma.

La acción principal de la ventana vive **al pie del panel de firma**: hay como
mucho un botón primario en pantalla.

Sobre la ventana pueden abrirse cuatro diálogos, que la oscurecen sin
desmontarla: [el del secreto del almacén](dialogo-pin.md),
[progreso de firma](dialogo-progreso-firma.md),
[preferencias](preferencias.md) y [acerca de](acerca-de.md).

**«Sobre» es literal, y no lo da el bundle.** Los cuatro se montan como
hermanos de la ventana, no dentro de ella, y `.rf-scrim` en el bundle es solo
el color del velo. Quien lo coloca —`position: fixed`, `inset: 0`, el diálogo
centrado y por encima del menú de la cabecera— es `rfirma-app/src/app.css`.
Sin esa regla los diálogos se pintan **en flujo**, detrás de la ventana y por
debajo del pliegue, con la banda oscura del alto de su contenido: ni
superposición, ni ventana atenuada, ni centrado.

## La franja de notificación

**Entre la cabecera y las tres regiones hay sitio para una franja**, a ancho
completo, con borde inferior de 1 px en `--rf-border-subtle` y fondo
`--rf-surface`. No está casi nunca: cuando no hay nada que notificar la franja
**no se monta** y las regiones suben.

Lleva icono, una frase, **una sola acción** secundaria y una `×` para
descartarla. Cuesta **41 px** de alto a una ventana cuyo mínimo son 560, y por
eso admite una frase y no un párrafo.

Lo que se decidió aquí **no es dónde va el aviso de versión, sino dónde notifica
rFirma**: la franja es el patrón de notificación de la ventana, y el aviso de
versión nueva —«Hay una versión nueva de rFirma: 0.4.1», con «Cómo actualizar»
llevando a [Acerca de](acerca-de.md)— es su primer inquilino.

Se descartaron dos colocaciones, juzgadas con la ventana **ocupada** —documento
cargado, panel con su pie de destino, nombre de fichero largo—, porque un aviso
que sólo se ve con la ventana vacía no decide nada:

| | Colocación | Por qué no |
| - | ---------- | ---------- |
| A | Insignia en el botón de menú | no se ve hasta abrir el menú, así que no notifica: recuerda algo que ya estaba, y para siempre |
| B | Línea en el pie | rFirma **no tiene barra de estado**: estrenaba un mueble entero para una frase que casi siempre no está |
| **C** | **Franja bajo la cabecera** | **elegida**: se ve sin abrir nada, es descartable y desaparece del todo cuando no hay nada que decir |

Lo que la franja **no** es: un sitio para errores del recorrido. El error de
firma se queda en el pie del panel, como dice más abajo, y los fallos de
Preferencias van dentro de su sección.

### Geometría

- Cabecera de 56 px, borde inferior de 1 px en `--rf-border-subtle`.
- Bandeja de 300 px fijos, borde derecho de 1 px en `--rf-border-subtle`.
- Visor flexible, sin ancho propio.
- Panel de 360 px fijos, borde izquierdo de 1 px en `--rf-border-subtle`.
- Bandeja, visor y panel sobre `--rf-surface`; solo el papel del documento
  fuerza `data-theme="light"` sobre `--rf-bg`.
- **La ventana abre a 1280×720 y no baja de 1100×560.** Las dos columnas
  laterales suman 660 px fijos, así que lo que decide si el documento se lee es
  lo que sobra: a 1024 de ancho —la medida con la que arrancó— al visor le
  quedaban 364 px, menos de la mitad de una A4 al 100 %. El mínimo de **ancho**
  se fija en 1100 por lo mismo: por debajo, el visor deja de ser la región
  principal de la ventana y pasa a ser la más estrecha de las tres.

  El **alto** es otra historia y hasta la v0.3.0 estaba mal. Abría a 900 con un
  mínimo de 700, y ninguna de las dos cifras miraba la pantalla: en un portátil
  de 1366×768 la ventana nacía más alta que el escritorio, el pie del panel
  quedaba fuera, y el mínimo de 700 impedía encogerla lo bastante para
  recuperarlo. **La corrección no es añadir lógica de monitores**, es dejar de
  estorbar al gestor de ventanas, que ya sabe colocar: se pide un alto que quepa
  en un portátil y se baja el suelo hasta donde la ventana sigue siendo usable.
  El tamaño se sigue recordando entre sesiones, como estaba
  ([ADR-0010](../adr/0010-memoria-entre-sesiones.md)); lo que no se hace es
  imponerlo por encima de la pantalla.

## La secuencia no es negociable

El orden lo fija la criptografía, no el diseño:

```
configurar la firma visible → prefirma → firma → postfirma → guardar
```

La apariencia del recuadro forma parte del PDF cuyo hash se firma, así que
tiene que estar decidida **antes** de la prefirma. Se puede rediseñar la piel;
no la secuencia. Ver
[ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md).

**El secreto del almacén no es un eslabón de esa cadena** (ID-190). Hasta la
v0.3 la secuencia llevaba «PIN» entre la prefirma y la firma, con el argumento
de que no se puede pedir sin saber qué se va a firmar. No es cierto en general:
abrir la sesión del almacén es un requisito **del almacén**, no de la firma, y
según cuál sea cae en un sitio u otro.

- Sin necesidad de sesión, **no hay diálogo** y se firma directo.
- Un módulo PKCS#11 o un perfil de navegador con contraseña maestra abren sesión
  **para poder enumerar**, así que el diálogo sale en el estado 3, con la
  ventana todavía buscando y **sin lista de certificados detrás**.
- Un `.p12` instalado lista sin secreto y lo pide **al firmar** (ID-195), que es
  el único caso que se parece a lo que decía la v0.3.

Ver [dialogo-pin.md](dialogo-pin.md).

## Estados

Diez, y cada uno es una combinación de estados de las tres regiones. El
recorrido nunca cambia de pantalla.

| # | Estado | Bandeja | Visor | Panel |
| - | ------ | ------- | ----- | ----- |
| 1 | Vacío | sin recientes | zona de soltar grande | oculto |
| 2 | Documento cargado | documento seleccionado | documento | sin certificado; firma visible apagada |
| 3 | Cargando certificados | ídem | documento | esqueletos en la sección de certificado; **encima, el diálogo de secreto** si el almacén necesita sesión para listar |
| 4 | Sin certificados | ídem | documento | vacío con salida: volver a buscar, añadir un certificado |
| 5 | Configurando la firma visible | ídem | recuadro seleccionado, con asa | completo, botón activo |
| 6 | Pidiendo el secreto del almacén | ídem | atenuado | atenuado |
| 7 | Secreto incorrecto | ídem | atenuado | atenuado |
| 8 | Firmando | ídem | atenuado | atenuado |
| 9 | Firmado | insignia «Firmado» | documento firmado | resumen y acciones sobre el resultado |
| 10 | Error de firma | ídem | documento | aviso en el pie; el botón pasa a «Volver a intentarlo» |

El estado 5 es el nudo del recorrido: es donde se toma la decisión que el resto
del flujo se limita a ejecutar.

El error de firma **no abre un diálogo**: se queda en el pie del panel, con el
detalle técnico y el botón convertido en «Volver a intentarlo», porque se
reintenta sin volver a configurar nada.

## Componentes y tokens

Regiones separadas por `--rf-border-subtle`; bandeja y panel sobre
`--rf-surface`, visor sobre `--rf-surface` con el papel en `--rf-bg` forzando
`data-theme="light"`. Diálogos con `.rf-scrim` + `.rf-dialog`.

Tema claro y oscuro, decididos por el sistema operativo. El papel del documento
es siempre claro: el papel es papel.

## Decisiones

Se compararon cuatro estructuras y ganó la **bandeja con panel lateral**:

| | Variante | Por qué no |
| - | -------- | ---------- |
| A | Asistente por pasos | cuatro pantallas para firmar una vez |
| B | Documento al centro | sin bandeja, cambiar de documento no tiene sitio propio |
| C | Panel único | la previsualización queda como salida muerta, no se toca |
| **D** | **Bandeja** | **elegida**: reconoce el documento y reutiliza la última configuración |

La ganadora se quedó con la barra lateral derecha de B, así que el resultado no
es D pura: es D con el panel de B.

Validado en el canvas [Autofirma de escritorio en Rust](https://claude.ai/design/p/c0ddbfa7-0982-498f-8f8c-8e2f8f0c6132), página
**Recorrido de firma**, el 31/08/2026. Las variantes descartadas están en el
historial de la rama `prototype/firmar-pdf-local`.
