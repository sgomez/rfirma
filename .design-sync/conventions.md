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

### Estilo: primitivos, clases `rf-*` y tokens `--rf-*`

Las pantallas se componen con primitivos: `Button`, `Card`, `Field`, `Badge`, `Dialog`, `Popover`, `Menu` (con `MenuItem`), `Select`, `Switch`, `ProgressBar`, `Stack` y `Row` (todos en `window.RFirma`). Una clase `rf-*` se escribe a mano solo si aún no tiene primitivo: el texto, `rf-input`, `rf-divider`, `rf-label` y poco más. No inventes clases ni escribas colores, radios o sombras literales: para tu maquetación propia, `var(--rf-*)`.

| Primitivo | Props |
| --- | --- |
| `Button` | `variant`: `primary` / `secondary` / `ghost`; `type="button"` por defecto |
| `Card` | `elevated` |
| `Field` | etiqueta, control y ayuda apilados |
| `Badge` | `variant`: `primary` |
| `Dialog` | `label`, `onClose`, `role`: `dialog` / `alertdialog` |
| `Popover` | `open`, `onClose`, `anchorRef` |
| `Stack` / `Row` | `gap`: `xs` / `md` en `Stack`, `xs` / `sm` en `Row` |

Clases a mano, solo las que no tienen primitivo:

| Familia | Clases |
| --- | --- |
| Superficies | `rf-surface`, `rf-section`, `rf-divider` |
| Texto | `rf-display`, `rf-heading`, `rf-headline`, `rf-title`, `rf-subtitle`, `rf-ui`, `rf-ui--strong`, `rf-body`, `rf-prose`, `rf-caption`, `rf-label`, `rf-hint`, `rf-text-muted`, `rf-text-primary` |
| Formularios | `rf-input` |

El texto elige rol, no números: cada clase de rol fija tamaño, peso e interlínea juntos. No escribas `font-size`, `font-weight` ni `line-height` literales.

Las clases de los primitivos (`rf-btn`, `rf-card`, `rf-field`, `rf-badge`, `rf-dialog`, `rf-scrim`, `rf-stack`, `rf-row`, `rf-gap-*`) las pone el primitivo: no las escribas.

Tokens: color `--rf-bg`, `--rf-surface`, `--rf-text`, `--rf-text-muted`, `--rf-primary`, `--rf-primary-hover`, `--rf-on-primary`, `--rf-accent`, `--rf-border-subtle`, `--rf-border-strong`; espacio `--rf-space-xs` … `--rf-space-2xl`; radio `--rf-radius-sm` / `-md` / `-lg` / `-xl` / `-pill`; sombra `--rf-shadow-card`, `--rf-shadow-elevated`; movimiento `--rf-duration-fast` / `-base` / `-slow`, `--rf-easing`; tipografía `--rf-<rol>-size` / `-weight` / `-leading` por rol, `--rf-ui-strong-weight` y `--rf-body-prose-leading`.

### Iconos

Componentes `*Icon` con prop `size` en px: `CheckCircleIcon`, `CrossCircleIcon`, `AlertIcon`, `InfoIcon`, `FileIcon`, `FolderIcon`, `PersonIcon`, `SearchIcon`, `CopyIcon`, `ExternalLinkIcon`, `CloseIcon`, `SpinnerIcon`, `ChevronDownIcon`, `ChevronRightIcon`, entre otros (todos en `window.RFirma`).

### Pantallas

`SedeView` es la ventana que abre una sede electrónica (520 × 420), con el momento del trámite en la prop `errand` y sus órdenes como props: no recibe ningún puerto. Úsala entera cuando el diseño sea esa ventana, con el `errand` del momento que toque; sus textos salen del catálogo. Las pantallas nuevas se componen con los primitivos de arriba, el texto con sus clases y los iconos.

### Catálogo publicado

Lo que hay en `window.RFirma`, por capa y título de historia.

<!-- design-sync:catalog:start -->
| Capa | Título | Componente |
| --- | --- | --- |
| Dominio | Actualizaciones / NewVersionStrip | `NewVersionStrip` |
| Dominio | Documentos / DocumentTabs | `DocumentTabs` |
| Dominio | Documentos / RecentRows | `RecentsSection` |
| Dominio | Errores / ErrorNotice | `ErrorNotice` |
| Dominio | Firma / CertificateCard | `CertificateCard` |
| Dominio | Firma / CertificateSelect | `CertificateSelect` |
| Flujos | Documentos / DocumentViewer | `DocumentViewer` |
| Flujos | Estado / WithdrawCertificateView | `WithdrawCertificateView` |
| Flujos | Firma / PlacementBlock | `PlacementBlock` |
| Flujos | Firma / SignAnywayDialog | `SignAnywayDialog` |
| Flujos | Firma / SignaturesDialog | `SignaturesDialog` |
| Flujos | Firma / SignaturesPanel | `SignaturesPanel` |
| Flujos | Firma / SigningPanel | `SigningPanel` |
| Flujos | Firma / SigningProgressDialog | `SigningProgressDialog` |
| Flujos | Firma / UnsealedPagesDialog | `UnsealedPagesDialog` |
| Flujos | Firma / VisibleSignatureFieldset | `VisibleSignatureFieldset` |
| Flujos | Sede / Confirmar | `SedeView` |
| Flujos | Sede / Consentimiento | `SedeView` |
| Flujos | Sede / Desenlace | `SedeView` |
| Flujos | Sede / Espera | `SedeView` |
| Flujos | Sede / Firmando | `SedeView` |
| Flujos | Sede / Marcar la firma | `SedeView` |
| Flujos | Sede / Sin certificado | `SedeView` |
| Flujos | Ventana principal / Header | `Header` |
| Flujos | Ventana principal / MainWindow | `MainWindow` |
| Primitivos | Badge | `Badge` |
| Primitivos | Button | `Button` |
| Primitivos | Card | `Card` |
| Primitivos | Dialog | `Dialog` |
| Primitivos | Field | `Field` |
| Primitivos | Menu | `Menu` |
| Primitivos | Popover | `Popover` |
| Primitivos | ProgressBar | `ProgressBar` |
| Primitivos | Row | `Row` |
| Primitivos | Select | `Select` |
| Primitivos | Stack | `Stack` |
| Primitivos | Switch | `Switch` |
<!-- design-sync:catalog:end -->

### Dónde mirar

- `styles.css` e `_ds_bundle.css`: todas las clases y tokens.
- `components/sede/SedeView/SedeView.prompt.md`: los estados de la ventana de sede.

### Ejemplo

```jsx
const { DesignRoot, Card, Stack, Row, Button, CheckCircleIcon } = window.RFirma;
<DesignRoot language="es" theme="light">
  <Card>
    <Stack gap="md" style={{ padding: "var(--rf-space-lg)" }}>
      <Row gap="xs">
        <CheckCircleIcon size={24} />
        <p className="rf-title">Documento firmado</p>
      </Row>
      <p className="rf-prose">La firma se ha guardado junto al documento.</p>
      <Row gap="sm">
        <Button variant="ghost">Ver firmas</Button>
        <Button variant="primary">Cerrar</Button>
      </Row>
    </Stack>
  </Card>
</DesignRoot>
```
