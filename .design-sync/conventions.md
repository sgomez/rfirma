## Convenciones de rFirma

rFirma es una aplicación de escritorio de firma electrónica para España. Los textos van en castellano de España, de tú, cortos y sin jerga criptográfica.

### Envoltura obligatoria: `DesignRoot`

Todo diseño se monta dentro de `DesignRoot`. Aporta tres cosas:
- el catálogo de textos en el idioma pedido: los componentes no llevan ni una cadena escrita en línea, y sin él se pintan vacíos;
- la clase `rf-root`, de la que cuelgan la tipografía y el fondo;
- el tema, con `data-theme`.

```jsx
const { DesignRoot } = window.RFirma;
<DesignRoot language="es" theme="light">{/* pantalla */}</DesignRoot>
```

`language`: `es` | `ca` | `en` | `eu` | `gl`. `theme`: `light` | `dark`.

### Estilo: clases `rf-*` y tokens `--rf-*`

No hay componentes React para las piezas básicas. Botones, tarjetas y campos son clases CSS. No inventes clases ni escribas colores, radios o sombras literales: usa estas clases y, para tu maquetación propia, `var(--rf-*)`.

| Familia | Clases |
| --- | --- |
| Botones | `rf-btn` + `rf-btn--primary` / `--secondary` / `--ghost` / `--pill` / `--disabled` |
| Superficies | `rf-surface`, `rf-card` (`--elevated`, `--interactive`), `rf-dialog`, `rf-scrim`, `rf-section`, `rf-divider` |
| Texto | `rf-display`, `rf-heading`, `rf-title`, `rf-body`, `rf-prose`, `rf-label`, `rf-hint`, `rf-text-muted`, `rf-text-primary` |
| Formularios | `rf-field` (`--error`), `rf-input` |
| Maquetación | `rf-stack` (columna), `rf-row` (fila), `rf-gap-xs` / `-sm` / `-md` / `-lg` |
| Otros | `rf-badge` (`--primary`) |

Tokens: color `--rf-bg`, `--rf-surface`, `--rf-text`, `--rf-text-muted`, `--rf-primary`, `--rf-primary-hover`, `--rf-on-primary`, `--rf-accent`, `--rf-border-subtle`, `--rf-border-strong`; espacio `--rf-space-xs` … `--rf-space-2xl`; radio `--rf-radius-sm` / `-md` / `-lg` / `-xl` / `-pill`; sombra `--rf-shadow-card`, `--rf-shadow-elevated`; movimiento `--rf-duration-fast` / `-base` / `-slow`, `--rf-easing`.

### Iconos

Componentes `*Icon` con prop `size` en px: `CheckCircleIcon`, `CrossCircleIcon`, `AlertIcon`, `InfoIcon`, `FileIcon`, `FolderIcon`, `PersonIcon`, `SearchIcon`, `CopyIcon`, `ExternalLinkIcon`, `CloseIcon`, `SpinnerIcon`, `ChevronDownIcon`, `ChevronRightIcon`, entre otros (todos en `window.RFirma`).

### Pantallas

`SedeWindow` es la ventana que abre una sede electrónica (520 × 420). Recibe un puerto `errands` que la app conecta a Tauri, así que úsala como referencia de cómo se ve una ventana de rFirma, no como pieza para componer. Las pantallas nuevas se construyen con las clases de arriba y los iconos.

### Dónde mirar

- `styles.css` e `_ds_bundle.css`: todas las clases y tokens.
- `components/sede/SedeWindow/SedeWindow.prompt.md`: los estados de la ventana de sede.

### Ejemplo

```jsx
const { DesignRoot, CheckCircleIcon } = window.RFirma;
<DesignRoot language="es" theme="light">
  <div className="rf-card rf-stack rf-gap-md" style={{ padding: "var(--rf-space-lg)" }}>
    <div className="rf-row rf-gap-xs">
      <CheckCircleIcon size={24} />
      <p className="rf-title">Documento firmado</p>
    </div>
    <p className="rf-prose">La firma se ha guardado junto al documento.</p>
    <div className="rf-row rf-gap-sm">
      <button type="button" className="rf-btn rf-btn--ghost">Ver firmas</button>
      <button type="button" className="rf-btn rf-btn--primary">Cerrar</button>
    </div>
  </div>
</DesignRoot>
```
