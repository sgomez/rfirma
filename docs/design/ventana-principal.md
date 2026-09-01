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
desmontarla: [PIN](dialogo-pin.md), [progreso de firma](dialogo-progreso-firma.md),
[preferencias](preferencias.md) y [acerca de](acerca-de.md).

**«Sobre» es literal, y no lo da el bundle.** Los cuatro se montan como
hermanos de la ventana, no dentro de ella, y `.rf-scrim` en el bundle es solo
el color del velo. Quien lo coloca —`position: fixed`, `inset: 0`, el diálogo
centrado y por encima del menú de la cabecera— es `rfirma-app/src/app.css`.
Sin esa regla los diálogos se pintan **en flujo**, detrás de la ventana y por
debajo del pliegue, con la banda oscura del alto de su contenido: ni
superposición, ni ventana atenuada, ni centrado.

### Geometría

- Cabecera de 56 px, borde inferior de 1 px en `--rf-border-subtle`.
- Bandeja de 300 px fijos, borde derecho de 1 px en `--rf-border-subtle`.
- Visor flexible, sin ancho propio.
- Panel de 360 px fijos, borde izquierdo de 1 px en `--rf-border-subtle`.
- Bandeja, visor y panel sobre `--rf-surface`; solo el papel del documento
  fuerza `data-theme="light"` sobre `--rf-bg`.

## La secuencia no es negociable

El orden lo fija la criptografía, no el diseño:

```
configurar la firma visible → prefirma → PIN → firma → postfirma → guardar
```

La apariencia del recuadro forma parte del PDF cuyo hash se firma, así que
tiene que estar decidida **antes** de la prefirma; y el PIN no puede pedirse
antes de saber qué se va a firmar. Se puede rediseñar la piel; no la secuencia.
Ver [ADR-0001](../adr/0001-firma-trifasica-clave-privada-solo-en-rust.md).

## Estados

Diez, y cada uno es una combinación de estados de las tres regiones. El
recorrido nunca cambia de pantalla.

| # | Estado | Bandeja | Visor | Panel |
| - | ------ | ------- | ----- | ----- |
| 1 | Vacío | sin recientes | zona de soltar grande | oculto |
| 2 | Documento cargado | documento seleccionado | documento | sin certificado; firma visible apagada |
| 3 | Cargando certificados | ídem | documento | esqueletos en la sección de certificado |
| 4 | Sin certificados | ídem | documento | vacío con salida: volver a buscar, otro módulo |
| 5 | Configurando la firma visible | ídem | recuadro seleccionado, con asa | completo, botón activo |
| 6 | Pidiendo PIN | ídem | atenuado | atenuado |
| 7 | PIN incorrecto | ídem | atenuado | atenuado |
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
