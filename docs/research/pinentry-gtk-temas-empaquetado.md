# Diálogos de PIN en rFirma: pinentry, GTK nativo y entorno Flatpak

Investigación sobre la viabilidad técnica, garantías de seguridad, empaquetado
y comportamiento bajo sandbox de delegar la solicitud de secretos (PIN de token
PKCS#11 o contraseñas de almacenes) en `pinentry` frente a soluciones nativas
o internas en rFirma.

---

## Resumen y veredicto

1. **Garantías de seguridad de `pinentry`:** utiliza el protocolo Assuan sobre
   tuberías Unix anónimas (`pipe(2)`). Implementa memoria segura en `secmem.c`
   bloqueando páginas en RAM física con `mlock()` (evita volcado a *swap* en
   disco) y borrado en cuatro pasadas (`wipememory2` con `0xff`, `0xaa`, `0x55`,
   `0x00`) resistente a optimizaciones del compilador. Sin embargo, la captura de
   teclado (*grab*) tradicional de `pinentry-gtk-2` (`gdk_keyboard_grab`) no
   funciona en Wayland; en distribuciones modernas, `pinentry-gnome3` delega en
   `GcrPrompt` / `org.gnome.keyring.SystemPrompter` sobre D-Bus, donde GNOME
   Shell dibuja el diálogo modal directamente desde el compositor.
2. **Integración visual y tema del sistema:** el tema del sistema es válido y
   se ve bien sin necesidad de tematización personalizada. `pinentry-gnome3` se
   integra con el aspecto del escritorio GNOME (diálogo modal de sistema). Frente
   a esto, un diálogo GTK propio en Rust con `gtk-rs` adoptaría el tema GTK
   activo sin intermediarios de proceso, permitiendo memoria segura con
   `libc::mlock` y el crate `zeroize`, eliminando la transmisión en claro del
   PIN a través de tuberías IPC.
3. **Empaquetado y nombres de paquetes:**
   * En **Debian/Ubuntu (`.deb`)**: el paquete GTK moderno es `pinentry-gnome3`
     (en *main*). `pinentry-gtk2` está confinado a *universe* en Ubuntu y arrastra
     el stack obsoleto de GTK+ 2.x. Debe declararse como
     `Depends: pinentry-gnome3 | pinentry-x11 | pinentry`.
   * En **Fedora/openSUSE (`.rpm`)**: a partir de Fedora 39 y RHEL 10,
     `pinentry-gtk` **ya no existe** (fue retirado y el `.spec` declara
     `Obsoletes: pinentry-gtk`). Declarar dependencia de `pinentry-gtk` rompe la
     instalación en Fedora moderno. El paquete gráfico es `pinentry-gnome3` o
     `pinentry-qt`. Debe declararse `Requires: (pinentry-gnome3 or pinentry-gui or pinentry)`.
4. **Flatpak (`org.gnome.Platform:50`):**
   * `/usr/bin/pinentry-gnome3` y `/usr/libexec/gcr-prompter` **ya vienen
     incluidos** en el runtime `org.gnome.Platform:50`. No hace falta compilar
     ningún módulo adicional.
   * Invocar el del *host* mediante `flatpak-spawn --host` exigiría
     `--talk-name=org.freedesktop.Flatpak`, lo cual representa un escape total
     del sandbox prohibido en Flathub.
   * Usar el `pinentry-gnome3` del runtime requiere el permiso
     `--talk-name=org.gnome.keyring.SystemPrompter`. Sin él, falla de forma
     silenciosa cayendo a modo terminal *curses* (`No Gcr System Prompter available, falling back to curses`).
   * **El problema de portabilidad:** dentro del Flatpak solo existe el binario
     GNOME; en escritorios KDE Plasma, Sway o Hyprland donde no corre el servicio
     D-Bus de GNOME Keyring, `pinentry-gnome3` no puede abrir diálogo gráfico y
     falla.

---

## 1. Funcionamiento de pinentry y garantías de seguridad

### El protocolo Assuan

Pinentry se comunica mediante el protocolo Assuan (especificación de Libassuan,
mantenida por g10 Code / FSF para GnuPG). Es un protocolo textual orientado a
líneas diseñado para ejecutarse sobre tuberías anónimas creadas con `pipe(2)`
o pares de sockets Unix.

El ciclo de ejecución típico entre una aplicación cliente y pinentry sigue
esta secuencia:

```text
rFirma                                  pinentry-gnome3
  |                                            |
  |-- (arranque del proceso con stdin/stdout)->|
  |<-- "OK Pleased to meet you, process 1234" -|
  |-- "SETTITLE rFirma\n" -------------------->|
  |<-- "OK\n" ---------------------------------|
  |-- "SETDESC Inserte el PIN del token\n" --->|
  |<-- "OK\n" ---------------------------------|
  |-- "SETPROMPT PIN:\n" --------------------->|
  |<-- "OK\n" ---------------------------------|
  |-- "SETOK Continuar\n" -------------------->|
  |<-- "OK\n" ---------------------------------|
  |-- "SETCANCEL Cancelar\n" ------------------>|
  |<-- "OK\n" ---------------------------------|
  |-- "GETPIN\n" ----------------------------->| (Muestra interfaz modal)
  |                                            | (Usuario teclea y confirma)
  |<-- "D 1234\n" -----------------------------| (Dato devuelto)
  |<-- "OK\n" ---------------------------------|
  |-- "BYE\n" -------------------------------->|
  |<-- "OK closing connection\n" --------------|
```

* **Superficie de ataque en tránsito:** el canal es una tubería local entre
  padre e hijo. En Linux, el acceso a los descriptores está protegido por el
  aislamiento de procesos del kernel (salvo `ptrace` o inspección de memoria
  con privilegios equivalentes). No obstante, el PIN viaja en texto plano en la
  línea `D <secreto>\n`.
* **Manejo de errores y reintentos:** si el usuario introduce un PIN erróneo, el
  protocolo no mantiene un bucle interactivo dentro de la misma llamada
  `GETPIN`; el cliente debe comprobar el resultado contra el almacén PKCS#11 y,
  en caso de fallo (`CKR_PIN_INCORRECT`), emitir `SETERROR PIN incorrecto` y
  volver a llamar a `GETPIN`.

### Gestión de memoria segura (`secmem`)

El código fuente de pinentry (`secmem/secmem.c`, heredado de GnuPG / libgcrypt)
incorpora mecanismos estrictos para proteger los secretos en la memoria del
proceso:

1. **Reserva bloqueada con `mlock()`:**
   En `secmem.c:141`, el pool inicial (habitualmente 32 KiB) se somete a:
   ```c
   err = mlock(p, n);
   ```
   Esto asegura que las páginas de memoria física donde se almacena el búfer del
   PIN nunca sean volcadas a la partición o fichero de intercambio (*swap*) del
   sistema operativo. Si los límites de usuario (`RLIMIT_MEMLOCK`) impiden el
   bloqueo, emite un aviso a stderr (`can't lock memory: %s`) y continúa en
   memoria insegura.
2. **Borrado seguro de memoria (*wiping*):**
   Al invocar `secmem_free()` (`secmem.c:382`), el búfer se limpia en cuatro
   pasadas sucesivas con patrones alternos:
   ```c
   wipememory2(mb, 0xff, size);
   wipememory2(mb, 0xaa, size);
   wipememory2(mb, 0x55, size);
   wipememory2(mb, 0x00, size);
   ```
   La función `wipememory2` usa barreras de memoria para evitar que el
   optimizador de GCC/Clang suprima la escritura mediante eliminación de código
   muerto (*dead-store elimination*).
3. **Abandono de privilegios y volcado de memoria (*core dumps*):**
   Pinentry invoca `drop_privs()` tras inicializar la memoria segura
   (`pinentry.c:734`) y registra un manejador con `atexit(secmem_term)` para
   garantizar que toda la memoria sensible se limpie antes de que el proceso
   abandone la ejecución.

### Captura de teclado (*grab*): X11 frente a Wayland

Existe una diferencia arquitectónica fundamental entre las variantes de
pinentry respecto a la exclusividad de la entrada del usuario:

* **`pinentry-gtk-2` (X11 clásico):**
  Implementa `gdk_keyboard_grab()` y `gdk_pointer_grab()` en
  `gtk+-2/pinentry-gtk-2.c:171`. En servidores X11, esto bloquea que cualquier
  otra ventana de usuario o proceso en la sesión reciba pulsaciones de teclas
  (mitigando escuchas no privilegiadas tipo `XRecord`). Sin embargo, en X11
  conlleva riesgo de bloqueo completo del entorno (*lockup*) si el diálogo no
  responde; además, bajo Wayland esta llamada carece de efecto sobre la sesión
  global.
* **`pinentry-gnome3` (moderno / GNOME):**
  No realiza ninguna llamada a `gdk_keyboard_grab`. En su lugar, utiliza la
  API de GCR (`gnome3/pinentry-gnome3.c:134`):
  ```c
  prompt = GCR_PROMPT(gcr_system_prompt_open(pe->timeout ? pe->timeout : -1, NULL, &error));
  ```
  `gcr_system_prompt_open` establece una llamada D-Bus a
  `org.gnome.keyring.SystemPrompter`. En una sesión GNOME (Wayland o X11),
  este servicio está implementado directamente por **GNOME Shell**
  (`/usr/bin/gnome-shell`).
  Bajo Wayland, el propio compositor presenta un modal del sistema y dirige la
  entrada de forma exclusiva. La seguridad de captura de teclado no descansa en
  el cliente, sino en el compositor del sistema.

---

## 2. Integración visual con el tema del sistema y comparativa técnica

### El tema del sistema es válido

El requisito confirma que la apariencia predeterminada del sistema operativo y
del entorno de escritorio es aceptable. No es necesario aplicar estilos CSS a
medida:

* **`pinentry-gnome3`:** hereda directamente la estética del sistema en GNOME.
  El diálogo es visualmente idéntico a las solicitudes de autenticación de
  Polkit o del depósito de claves de GNOME (*GNOME Keyring*).
* **`pinentry-qt`:** en entornos KDE Plasma, se dibuja con el estilo nativo
  Breeze y controles Qt6.
* **`pinentry-gtk2`:** presenta un aspecto desactualizado debido a que GTK+ 2.x
  no dispone de motores de tema modernos compatibles con las variantes oscuras
  o esquemas de GNOME 4x/5x.

### Comparativa: pinentry externo vs. diálogo GTK nativo en Rust (`gtk-rs`)

Si en lugar de invocar el binario externo de pinentry se construyera un diálogo
nativo en Rust empleando `gtk-rs` (`gtk3` o `gtk4`):

| Característica | pinentry externo (`pinentry-gnome3`) | Diálogo GTK nativo en Rust (`gtk-rs`) |
|---|---|---|
| **Modelo de proceso** | Proceso hijo independiente vía `fork`/`exec` | Mismo proceso (hilo secundario o modal GTK) |
| **Protocolo de comunicación** | Assuan por tuberías de texto plano | Llamadas a funciones / canales mpsc en memoria |
| **Gestión de memoria (`mlock`)** | Sí, gestionada en `secmem.c` | Sí, mediante llamada directa a `libc::mlock` |
| **Borrado seguro (*zeroize*)** | Sí, con `wipememory2` en `secmem.c` | Sí, mediante los crates `zeroize` / `secrecy` |
| **Integración con tema** | Nativa de GNOME Shell (vía D-Bus) | Nativa de GTK3/GTK4 según el tema de la sesión |
| **Dependencia de servicios de sesión** | Requiere `org.gnome.keyring.SystemPrompter` | Autosuficiente, no depende de D-Bus de sesión |
| **Comportamiento en KDE / Sway** | Falla o cae a terminal si no hay prompter | Abre la ventana GTK con normalidad |
| **Riesgo de cuelgue por IPC** | Posible si el prompter D-Bus queda bloqueado | Inexistente (ciclo de vida en el ejecutable) |

La implementación en Rust con `gtk-rs` permite retener el secreto en una
estructura con `ZeroizeOnDrop` y `libc::mlock`, trasladándolo directamente a la
llamada `C_Login` de PKCS#11 sin emitir líneas de texto a través de descriptores
de tubería del sistema operativo.

---

## 3. Empaquetado y dependencias en distribuciones Linux

### Debian y Ubuntu (`.deb`)

En los repositorios oficiales de Debian (12 *bookworm*, 13 *trixie*) y Ubuntu
(22.04, 24.04, 26.04), el paquete fuente `pinentry` genera varios binarios:

| Paquete | Componente / Sección | Descripción y librerías clave |
|---|---|---|
| **`pinentry-gnome3`** | `main` | Usa `libgcr-base-3-1`, `libassuan9`, `libsecret-1-0`, `libglib2.0-0t64`. Provee `pinentry` y `pinentry-x11`. |
| **`pinentry-gtk2`** | `universe` (Ubuntu) / `optional` (Debian) | Usa `libgtk2.0-0t64`. **Desaconsejado:** toolkit obsoleto, paquete en universe. |
| **`pinentry-qt`** | `main` / `universe` | Usa Qt5/Qt6 para entornos KDE. Provee `pinentry-x11`. |
| **`pinentry-curses`** | `main` | Modo consola mediante ncurses. |

**Declaración en `packaging/deb/`:**
Para garantizar disponibilidad gráfica en cualquier escritorio sin forzar GTK2:
```control
Depends: pinentry-gnome3 | pinentry-x11 | pinentry
```
*Evitar terminantemente:* `Depends: pinentry-gtk2` o `pinentry-gtk` (este último
no existe como paquete binario en Debian).

### Fedora y openSUSE (`.rpm`)

#### Fedora (39 a rawhide)

Comprobado directamente en el archivo de empaquetado oficial
(`src.fedoraproject.org/rpms/pinentry/pinentry.spec`):

```rpm
%if 0%{?fedora} && 0%{?fedora} < 39 || 0%{?rhel} && 0%{?rhel} < 10
%bcond_without gtk2
%endif
...
%if ! %{with gtk2}
Obsoletes: %{name}-gtk < %{version}-%{release}
%endif
```

1. **`pinentry-gtk` ya no existe en Fedora:** fue suprimido a partir de
   Fedora 39 al retirar GTK2 de la distribución. El spec declara de forma
   explícita `Obsoletes: pinentry-gtk`. Si rFirma declarase `Requires: pinentry-gtk`,
   el paquete fallará al resolver dependencias en cualquier versión actual.
2. Los subpaquetes gráficos vigentes en Fedora son:
   * **`pinentry-gnome3`**: compilado con `pkgconfig(gcr-4)` y `pkgconfig(libsecret-1)`.
   * **`pinentry-qt`**: compilado con Qt6 (`Qt6Widgets`, `KF6WindowSystem`).
3. El paquete base `pinentry` contiene `/usr/bin/pinentry` (un script selector
   `pinentry-wrapper`) y la versión curses.

#### openSUSE (Leap y Tumbleweed)

Dispone de los paquetes `pinentry-gnome3`, `pinentry-qt6` y el script de
despacho alternativo `pinentry-gui` mediante `update-alternatives`.

**Declaración en `packaging/rpm/`:**
```spec
Requires: (pinentry-gnome3 or pinentry-gui or pinentry)
```

---

## 4. Funcionamiento bajo Flatpak (`org.gnome.Platform:50`)

### Disponibilidad en el runtime

Verificado mediante inspección directa del runtime `org.gnome.Platform//50`:

```bash
flatpak run --command=sh org.gnome.Platform//50 -c "which pinentry-gnome3; ls -l /usr/bin/pinentry"
# Salida:
# /usr/bin/pinentry-gnome3
# /usr/bin/pinentry -> pinentry-gnome3
```

* `/usr/bin/pinentry-gnome3` (versión 1.3.3) **ya forma parte del runtime base**
  de GNOME 50.
* No existe `pinentry-gtk` ni `pinentry-qt` en este runtime.
* Por consiguiente, **no es necesario compilar pinentry como módulo** en
  `me.sgomez.rfirma.yml`.

### Permisos y barrera D-Bus

Aunque el ejecutable está dentro del sandbox, `pinentry-gnome3` no puede dibujar
ventanas por sí mismo si no tiene acceso al servicio prompter.

Comprobación empírica sin permisos D-Bus específicos:
```bash
echo -e "BYE" | flatpak run --command=pinentry-gnome3 org.gnome.Platform//50
# Salida:
# No Gcr System Prompter available, falling back to curses
```

* **Permiso D-Bus imprescindible:**
  Para que `pinentry-gnome3` funcione dentro de Flatpak, debe añadirse a
  `finish-args` en `me.sgomez.rfirma.yml`:
  ```yaml
  finish-args:
    - --talk-name=org.gnome.keyring.SystemPrompter
  ```
* **Invocación al binario del host (`flatpak-spawn --host`):**
  * Para ejecutar un `pinentry` instalado en el sistema anfitrión se requeriría
    invocar `flatpak-spawn --host pinentry`.
  * Esto exige conceder el permiso `--talk-name=org.freedesktop.Flatpak`.
  * **Inviable por política de seguridad:** este permiso concede control total
    sobre el anfitrión (escape completo del contenedor). La normativa de
    publicación en Flathub **prohíbe tajantemente** este permiso en aplicaciones
    que no sean entornos de desarrollo (IDEs). Descartado por diseño.

### La trampa de los escritorios no-GNOME en Flatpak

El runtime `org.gnome.Platform` solo proporciona `pinentry-gnome3`. Si un
usuario instala el Flatpak de rFirma en un equipo que ejecute **KDE Plasma**,
**Sway**, **Hyprland** o **LXQt**:

1. En la sesión de usuario no existirá el servicio D-Bus
   `org.gnome.keyring.SystemPrompter` (KDE usa su propio demonio para KWallet).
2. Dentro del sandbox no existe `pinentry-qt`.
3. `pinentry-gnome3` comprueba la ausencia del prompter del sistema y cae a
   modo *curses*.
4. Al no haber una consola tty vinculada a la aplicación gráfica, el proceso
   se queda bloqueado o cancela la operación con error `GPG_ERR_PIN_ENTRY`.

Esta asimetría significa que confiar en `pinentry-gnome3` dentro de Flatpak solo
funciona de forma transparente en sesiones GNOME Shell.

---

## 5. Matriz de decisión y conclusiones

Comparación entre las tres opciones posibles para la entrada del secreto en
rFirma:

| Criterio | A) pinentry externo (`pinentry-gnome3`) | B) Diálogo GTK nativo en Rust (`gtk-rs`) | C) Diálogo en ventana React (`PinDialog.tsx` actual) |
|---|---|---|---|
| **Aspecto del tema del sistema** | Nativo de GNOME Shell / Polkit | Nativo GTK3 del sistema | Estilo propio rFirma (`docs/design/dialogo-pin.md`) |
| **Protección en memoria RAM** | Sí (`mlock` + borrado en 4 pasadas) | Sí (`libc::mlock` + `zeroize`) | Parcial (solo una vez que el PIN llega a Rust) |
| **Canal de transmisión** | Tubería IPC en claro (`pipe(2)`) | Memoria interna del proceso | IPC Tauri (`invoke`) entre Webview y Rust |
| **Compatibilidad con Wayland** | Completa en GNOME Shell; nula fuera | Completa (ventana modal Wayland) | Completa (se dibuja dentro de la app) |
| **Funcionamiento en Flatpak** | Requiere D-Bus; roto fuera de GNOME | Nativo sin permisos extra | Nativo sin permisos extra |
| **Impacto en empaquetado** | Debe declarar dependencias por distro | Sin dependencias extra (GTK3 ya está en Tauri) | Cero dependencias adicionales |
| **Mantenimiento en código** | Parsing de protocolo Assuan e IPC | Gestión de ventana GTK en hilo de Tauri | Código existente y probado en TypeScript |

### Conclusión técnica

1. Delegar en un `pinentry` externo resuelve la protección en memoria a bajo
   nivel pero introduce una fuerte fragilidad en Flatpak para usuarios en
   escritorios no-GNOME y requiere dependencias condicionadas por distribución en
   `.deb` y `.rpm` (donde `pinentry-gtk` está extinto o relegado a *universe*).
2. Si se busca un diálogo con aspecto 100% nativo del sistema GTK y las mismas
   garantías de memoria segura que pinentry (`mlock` y *zeroize*), la solución
   técnicamente robusta es un diálogo interno en Rust con `gtk-rs`, sin incurrir
   en procesos externos ni canales IPC Assuan.
3. Si prima la coherencia con el diseño de la aplicación y la simplicidad
   operativa, el componente actual `PinDialog.tsx` dentro de Tauri ofrece
   portabilidad total entre canales sin dependencias externas, requiriendo
   únicamente aplicar `ZeroizeOnDrop` y `mlock` al búfer en el momento en que el
   backend de Rust recibe el secreto.
