#!/usr/bin/env bash
#
# EL ACCESO DE PUBLICACION AL VPS, DE PRINCIPIO A FIN.
#
# Hermano de `setup-signing-key.sh`. Aquel da de alta la clave que FIRMA; este
# da de alta el camino por el que los bytes ya firmados LLEGAN al anfitrion.
# Tampoco lo ejecuta ningun CI: lo ejecuta una persona, una sola vez.
#
# QUE PRODUCE:
#   - una clave ed25519 de despliegue, sin frase de paso, atada en el
#     `authorized_keys` del VPS a `command="rrsync /srv/rfirma-repo",restrict`;
#   - el secreto `PUBLISH_SSH_KEY` en el entorno `release`;
#   - las variables `PUBLISH_SSH_USER`, `PUBLISH_SSH_HOST` y
#     `PUBLISH_SSH_KNOWN_HOSTS`, que son publicas a proposito.
#
# POR QUE SIN FRASE DE PASO: la usa `publish.yml` sin nadie delante. Lo que
# sustituye a la frase es la orden forzada: esa clave no da consola, solo sabe
# escribir por rsync en un directorio que ya sirve ficheros publicos. Si se
# filtra, lo que gana quien la tenga es eso, y nada mas.
#
# POR QUE `rrsync` Y NO UN `command=` A MANO: rrsync encierra la orden en el
# directorio y lleva su propia lista de opciones admitidas. `publish-tree.sh`
# esta escrito contra esa lista —por eso no usa `--filter`— y
# `publish-tree.test.sh` levanta el MISMO rrsync detras de un ssh de mentira.
#
# QUE NO PUEDE HACER ESTE GUION: crear el usuario de sistema en el VPS y
# montar `/srv/rfirma-repo` en la aplicacion de Coolify. Los dos son pasos
# manuales y estan en sus etapas, con las ordenes exactas.
#
# Uso: packaging/setup-publish-access.sh
#
# Lo generado por la habilidad /wizard: todo lo que hay encima del marcador
# STAGES es su biblioteca y no se edita a mano.
set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────
# Wizard library: delightful, consistent UX, identical across every wizard.
# ──────────────────────────────────────────────────────────────────────────

if [[ -t 1 ]] && command -v tput >/dev/null 2>&1 && [[ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]]; then
  BOLD=$(tput bold); DIM=$(tput dim); RESET=$(tput sgr0)
  BLUE=$(tput setaf 4); GREEN=$(tput setaf 2); YELLOW=$(tput setaf 3); RED=$(tput setaf 1)
else
  BOLD=""; DIM=""; RESET=""; BLUE=""; GREEN=""; YELLOW=""; RED=""
fi

# Author sets this at the top of the stages section.
TOTAL_STAGES=0

_STAGE_INDEX=0
ENV_FILE="${ENV_FILE:-.env}"
WRITTEN_ENV=()    # KEYs written to ENV_FILE this run
WRITTEN_SECRET=() # secret NAMEs set this run
SKIPPED=()        # things we couldn't do (e.g. gh missing)

# _clear wipes the terminal so only the current step is on screen. No-op when
# output isn't a terminal, so piped logs stay readable.
_clear() {
  [[ -t 1 ]] || return 0
  if command -v tput >/dev/null 2>&1; then tput clear; else printf '\033[2J\033[3J\033[H'; fi
}

# banner "Title" shows the opening frame: what this wizard does.
banner() {
  _clear
  printf '\n%s%s  %s%s\n' "$BOLD" "$BLUE" "$1" "$RESET"
  printf '%s  %s stages%s\n\n' "$DIM" "$TOTAL_STAGES" "$RESET"
  printf '%s  You drive the browser; this wizard tells you exactly what to do and\n' "$DIM"
  printf '  captures the values you copy back. Stop any time with Ctrl-C and re-run\n'
  printf '  later, since it remembers values already saved.%s\n' "$RESET"
  pause "Ready to start?"
}

# stage "Name" clears the screen, then announces a stage and shows progress.
# Clearing keeps only the current step on screen.
stage() {
  _clear
  _STAGE_INDEX=$((_STAGE_INDEX + 1))
  printf '\n%s%s▸ Stage %s/%s · %s%s\n' \
    "$BOLD" "$BLUE" "$_STAGE_INDEX" "$TOTAL_STAGES" "$1" "$RESET"
}

# say "..." prints a plain instruction line.
say()  { printf '  %s\n' "$1"; }
# step "..." is a numbered-feeling action the human takes in the browser.
step() { printf '  %s•%s %s\n' "$BLUE" "$RESET" "$1"; }
note() { printf '  %s%s%s\n' "$DIM" "$1" "$RESET"; }
warn() { printf '  %s⚠ %s%s\n' "$YELLOW" "$1" "$RESET"; }

# open_url URL opens it in the human's browser, cross-platform incl. WSL.
open_url() {
  local url="$1"
  printf '  %s↗ opening%s %s\n' "$GREEN" "$RESET" "$url"
  { if   command -v wslview     >/dev/null 2>&1; then wslview "$url"
    elif command -v explorer.exe >/dev/null 2>&1; then explorer.exe "$url"
    elif command -v xdg-open    >/dev/null 2>&1; then xdg-open "$url"
    elif command -v open        >/dev/null 2>&1; then open "$url"
    else warn "couldn't open a browser; visit it manually: $url"; fi
  } >/dev/null 2>&1 || warn "couldn't open a browser, so visit it manually: $url"
}

# pause "msg" waits for the human to confirm they've done the manual part.
pause() {
  printf '  %s%s%s ' "$DIM" "${1:-Press Enter to continue}" "$RESET"
  read -r _ || true
}

# confirm "question" is a y/N gate; returns success on yes.
confirm() {
  local reply=""
  printf '  %s? %s [y/N] ' "$YELLOW" "$1"
  read -r reply || true
  [[ "$reply" =~ ^[Yy] ]]
}

# _existing KEY: current value of KEY in ENV_FILE, if any.
_existing() {
  [[ -f "$ENV_FILE" ]] || return 1
  local line; line=$(grep -E "^${1}=" "$ENV_FILE" | tail -n1) || return 1
  printf '%s' "${line#*=}"
}

# ask KEY "Prompt" reads a value into $KEY. Offers the existing .env value as
# a default on re-runs (Enter keeps it). Visible input (non-secret).
ask() {
  local key="$1" prompt="$2" current input
  current=$(_existing "$key" || true)
  if [[ -n "$current" ]]; then
    printf '  %s%s%s %s[Enter keeps current]%s ' "$BOLD" "$prompt" "$RESET" "$DIM" "$RESET"
  else
    printf '  %s%s%s ' "$BOLD" "$prompt" "$RESET"
  fi
  read -r input || true
  [[ -z "$input" && -n "$current" ]] && input="$current"
  printf -v "$key" '%s' "$input"
}

# ask_secret KEY "Prompt" is like ask, but input is hidden.
ask_secret() {
  local key="$1" prompt="$2" current input
  current=$(_existing "$key" || true)
  if [[ -n "$current" ]]; then
    printf '  %s%s%s %s[Enter keeps current]%s ' "$BOLD" "$prompt" "$RESET" "$DIM" "$RESET"
  else
    printf '  %s%s%s ' "$BOLD" "$prompt" "$RESET"
  fi
  read -rs input || true
  printf '\n'
  [[ -z "$input" && -n "$current" ]] && input="$current"
  printf -v "$key" '%s' "$input"
}

# write_env KEY VALUE upserts KEY=VALUE into ENV_FILE (creates it; replaces
# any existing line). Idempotent.
write_env() {
  local key="$1" value="$2" tmp
  touch "$ENV_FILE"
  tmp=$(mktemp)
  grep -vE "^${key}=" "$ENV_FILE" > "$tmp" || true
  printf '%s=%s\n' "$key" "$value" >> "$tmp"
  mv "$tmp" "$ENV_FILE"
  WRITTEN_ENV+=("$key")
  printf '  %s✓ wrote%s %s → %s\n' "$GREEN" "$RESET" "$key" "$ENV_FILE"
}

# set_secret NAME VALUE sets a GitHub Actions repo secret via gh. Falls back
# to a warning (and records it) if gh is unavailable or unauthenticated.
set_secret() {
  local name="$1" value="$2"
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    if printf '%s' "$value" | gh secret set "$name" >/dev/null 2>&1; then
      WRITTEN_SECRET+=("$name")
      printf '  %s✓ set%s GitHub secret %s\n' "$GREEN" "$RESET" "$name"
      return
    fi
  fi
  SKIPPED+=("GitHub secret $name (set it manually: gh secret set $name)")
  warn "skipped GitHub secret $name: gh not ready; set it later"
}

# set_var NAME VALUE sets a GitHub Actions repo variable (non-secret).
set_var() {
  local name="$1" value="$2"
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    if gh variable set "$name" --body "$value" >/dev/null 2>&1; then
      printf '  %s✓ set%s GitHub variable %s\n' "$GREEN" "$RESET" "$name"
      return
    fi
  fi
  SKIPPED+=("GitHub variable $name")
  warn "skipped GitHub variable $name, gh not ready; set it later"
}

# finish clears, then shows a closing summary of everything configured.
finish() {
  _clear
  printf '\n%s%s  ✓ Setup complete%s\n' "$BOLD" "$GREEN" "$RESET"
  (( ${#WRITTEN_ENV[@]} ))    && note "wrote ${#WRITTEN_ENV[@]} value(s) to $ENV_FILE: ${WRITTEN_ENV[*]}"
  (( ${#WRITTEN_SECRET[@]} )) && note "set ${#WRITTEN_SECRET[@]} GitHub secret(s): ${WRITTEN_SECRET[*]}"
  if (( ${#SKIPPED[@]} )); then
    printf '\n'; warn "still to do by hand:"
    for s in "${SKIPPED[@]}"; do note "  - $s"; done
  fi
  printf '\n'
}


# ──────────────────────────────────────────────────────────────────────────
# STAGES
# ──────────────────────────────────────────────────────────────────────────

TOTAL_STAGES=8

# Este guion no pega valores de un navegador: los produce `ssh-keygen` aqui
# mismo. Por eso no escribe ni una linea en `.env` —una clave privada dentro
# del arbol de trabajo es justo lo que se intenta evitar— y por eso hace falta
# el mismo ayudante que en `setup-signing-key.sh`: el SECRETO va al ENTORNO
# `release`, que es lo que impide que un job sin `environment: release` lo vea.
# Las tres VARIABLES son de repositorio y publicas a proposito.
ENV_FILE="$(mktemp -u)"   # nunca se escribe; solo evita tocar un .env real

ENTORNO=release
DIRECTORIO=/srv/rfirma-repo

set_env_secret() {
  local name="$1" fichero="$2"
  if gh secret set "$name" --env "$ENTORNO" < "$fichero" >/dev/null 2>&1; then
    WRITTEN_SECRET+=("$name")
    printf '  %s✓ puesto%s el secreto %s en el entorno %s\n' "$GREEN" "$RESET" "$name" "$ENTORNO"
    return 0
  fi
  SKIPPED+=("secreto $name (a mano: gh secret set $name --env $ENTORNO < $fichero)")
  warn "no se pudo poner el secreto $name"
  return 1
}

banner "El acceso de publicacion de rFirma"

# ── 1 ─────────────────────────────────────────────────────────────────────
stage "Antes de empezar"
say "Vas a abrir el camino por el que publish.yml deja los tres repositorios"
say "—ostree, apt y dnf— en el anfitrion que sirve rfirma.sgomez.me. La clave"
say "de firma ya no se toca: eso fue setup-signing-key.sh (ADR-0015)."
printf '\n'
step "Ten a mano el acceso de administracion al VPS (el que usas para sudo)."
step "Ten a mano el panel de Coolify: la ultima etapa se hace ahi."
printf '\n'
for herramienta in ssh ssh-keygen ssh-keyscan rsync gh; do
  if ! command -v "$herramienta" >/dev/null 2>&1; then
    warn "falta $herramienta, y sin el no se puede seguir"
    exit 1
  fi
done
if ! gh auth status >/dev/null 2>&1; then
  warn "gh no esta autenticado: ejecuta 'gh auth login' y vuelve"
  exit 1
fi
note "ssh, rsync y gh listos"
pause "Enter para continuar"

# ── 2 ─────────────────────────────────────────────────────────────────────
stage "El usuario y el directorio en el VPS"
say "Quien recibe la publicacion es un usuario de sistema suyo, dueno del"
say "directorio que ya sirve Caddy. No es tu usuario ni el de Coolify."
ask ANFITRION "Nombre o IP del VPS:"
if [ -z "${ANFITRION:-}" ]; then
  warn "sin anfitrion no hay nada que configurar"
  exit 1
fi
ask USUARIO "Usuario de publicacion (Enter para rfirma-publish):"
USUARIO="${USUARIO:-rfirma-publish}"
HOGAR="/var/lib/$USUARIO"
printf '\n'
say "Entra en el VPS y ejecuta esto, que es el aprovisionamiento humano del"
say "packaging/repo/README.md:"
printf '\n'
note "  sudo adduser --system --group --home $HOGAR --shell /bin/sh $USUARIO"
note "  sudo mkdir -p $DIRECTORIO"
note "  sudo chown $USUARIO:$USUARIO $DIRECTORIO"
note "  sudo chmod 755 $DIRECTORIO"
note "  command -v rrsync   # viene en el paquete rsync; sin el no hay publicacion"
printf '\n'
warn "si 'command -v rrsync' no dice nada, instala rsync antes de seguir"
printf '\n'
say "El '--shell /bin/sh' NO es un descuido y no abre ninguna puerta: sshd"
say "lanza la orden forzada A TRAVES del shell del usuario, asi que con el"
say "/usr/sbin/nologin que pone 'adduser --system' por su cuenta, lo que llega"
say "por la conexion es «This account is currently not available» en vez del"
say "protocolo de rsync. Quien cierra la puerta es command=+restrict, no el"
say "shell. Si el usuario ya existe con nologin: sudo chsh -s /bin/sh $USUARIO"
pause "Enter cuando el usuario y el directorio existan"

# ── 3 ─────────────────────────────────────────────────────────────────────
stage "La clave de despliegue"
say "ed25519 y SIN frase de paso: la usa publish.yml sin nadie delante. Lo que"
say "sustituye a la frase es la orden forzada de la etapa siguiente."
CLAVE_DIR="$(mktemp -d)"
chmod 700 "$CLAVE_DIR"
CLAVE="$CLAVE_DIR/publish"
ssh-keygen -t ed25519 -N '' -C "ci@rfirma" -f "$CLAVE" >/dev/null
LINEA="command=\"rrsync $DIRECTORIO\",restrict $(cat "$CLAVE.pub")"
note "clave nueva en $CLAVE (se borra al terminar)"
printf '\n'
say "Esta linea va EN EL VPS, al final de este fichero:"
printf '\n'
note "  $HOGAR/.ssh/authorized_keys"
printf '\n'
say "Es el authorized_keys del usuario $USUARIO —no el tuyo, ni el de root—,"
say "porque manda el del usuario con el que se conecta el CI. Si el fichero no"
say "existe, se crea. Si ya tiene claves, esta va debajo, en su propia linea:"
say "una clave por linea y sin cortarla, aunque aqui la veas dar la vuelta."
printf '\n%s\n\n' "$LINEA"
say "'restrict' quita pty, reenvio de puertos y agente; 'rrsync' encierra la"
say "orden en $DIRECTORIO. Con eso la clave del CI no da consola."
printf '\n'
say "No la copies todavia: la etapa siguiente la instala por ti si quieres."
pause "Enter para continuar"

# ── 4 ─────────────────────────────────────────────────────────────────────
stage "La linea en el authorized_keys"
say "Puedo instalarla por ssh con tu acceso de administracion, o la pegas tu."
if confirm "Instalarla ahora por ssh (te pedira tu sudo del VPS)?"; then
  YO="${USER:-$(id -un)}"
  ask ADMIN "Usuario con el que administras el VPS (Enter para $YO):"
  ADMIN="${ADMIN:-$YO}"
  # El `sed` de en medio borra las claves que dejo una ejecucion anterior de
  # este guion —son las que acaban en el comentario `ci@rfirma`— antes de
  # anadir la nueva. Sin el, cada relanzamiento dejaria viva una clave mas con
  # permiso de escritura en el directorio que se publica.
  if ssh -t "$ADMIN@$ANFITRION" "sudo install -d -m 700 -o '$USUARIO' -g '$USUARIO' '$HOGAR/.ssh' \
      && sudo touch '$HOGAR/.ssh/authorized_keys' \
      && sudo sed -i '/ ci@rfirma\$/d' '$HOGAR/.ssh/authorized_keys' \
      && printf '%s\n' '$LINEA' | sudo tee -a '$HOGAR/.ssh/authorized_keys' >/dev/null \
      && sudo chown '$USUARIO:$USUARIO' '$HOGAR/.ssh/authorized_keys' \
      && sudo chmod 600 '$HOGAR/.ssh/authorized_keys'"; then
    note "linea anadida a $HOGAR/.ssh/authorized_keys"
  else
    warn "no se pudo; pegala a mano y sigue"
    SKIPPED+=("la linea del authorized_keys de $USUARIO en $ANFITRION")
  fi
else
  say "Pegala tu en $HOGAR/.ssh/authorized_keys, con el directorio a 700, el"
  say "fichero a 600 y los dos de $USUARIO."
fi
pause "Enter cuando la linea este puesta"

# ── 5 ─────────────────────────────────────────────────────────────────────
stage "El anfitrion conocido"
say "publish.yml conecta con StrictHostKeyChecking=yes, asi que necesita saber"
say "de antemano la clave del anfitrion. Aceptar la que venga seria entregarle"
say "el arbol a quien conteste."
CONOCIDO="$CLAVE_DIR/known_hosts"
# El `grep` quita las lineas de comentario que ssh-keyscan intercala con la
# version del sshd. known_hosts las ignora, asi que no estorban, pero la
# variable de GitHub se lee con los ojos y ahi solo deberian estar las claves.
if ! ssh-keyscan "$ANFITRION" 2>/dev/null | grep -v '^#' > "$CONOCIDO" || [ ! -s "$CONOCIDO" ]; then
  warn "ssh-keyscan no obtuvo nada de $ANFITRION"
  exit 1
fi
printf '\n'
say "Huellas de lo que ha contestado $ANFITRION. Comparalas con lo que veas"
say "por consola en el VPS (ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub):"
printf '\n'
ssh-keygen -lf "$CONOCIDO" | while read -r linea; do note "  $linea"; done
printf '\n'
confirm "Son las del VPS?" || exit 1
pause "Enter para continuar"

# ── 6 ─────────────────────────────────────────────────────────────────────
stage "La prueba de verdad"
say "Antes de dar de alta ningun secreto: la clave nueva tiene que poder"
say "hablar por rsync y NO tiene que dar consola. Las dos cosas, o no sirve."
# BatchMode: sin el, un fallo de clave publica se convierte en una peticion de
# contrasena y el guion se queda colgado esperando a nadie.
SSH_PRUEBA="ssh -i $CLAVE -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$CONOCIDO"
BITACORA="$CLAVE_DIR/prueba.log"
printf '\n'
while true; do
if RSYNC_RSH="$SSH_PRUEBA" rsync --list-only "$USUARIO@$ANFITRION:" > "$BITACORA" 2>&1; then
  # Cuidado con leer de mas aqui: esto solo dice que rsync habla. Una clave SIN
  # restringir tambien pasa esta mitad, y ademas ve el disco entero. Quien lo
  # distingue es la comprobacion de la consola, ahi abajo.
  note "correcto: rsync habla con $ANFITRION"
  break
else
  warn "rsync no ha podido listar $DIRECTORIO con esa clave"
  printf '\n'
  say "Esto es lo que ha contestado $ANFITRION:"
  printf '\n'
  sed 's/^/    /' "$BITACORA"
  printf '\n'
  # Cada rama es un fallo distinto con un arreglo distinto: decirlos los tres a
  # la vez es lo que hacia inutil el aviso anterior.
  if grep -qi 'permission denied' "$BITACORA"; then
    say "El anfitrion no acepta la clave. Por orden de probabilidad:"
    step "la linea esta en el authorized_keys de otro usuario (tiene que ser el"
    note "  de $USUARIO: $HOGAR/.ssh/authorized_keys);"
    step "los permisos: el .ssh a 700 y el authorized_keys a 600, los dos de"
    note "  $USUARIO —sshd ignora en silencio un fichero demasiado abierto—;"
    step "la clave se pego cortada en dos lineas."
    note "  Compruebalo asi, desde tu acceso de administracion:"
    note "  sudo ls -la $HOGAR/.ssh && sudo wc -l $HOGAR/.ssh/authorized_keys"
  elif grep -qiE 'rrsync: command not found|command not found' "$BITACORA"; then
    say "La linea esta bien, pero rrsync no existe en el VPS. Instalalo:"
    note "  sudo apt install rsync    # trae /usr/bin/rrsync"
    note "Si ya esta instalado, la linea tiene que llamarlo por su ruta entera."
  elif grep -qiE 'chdir|no such file' "$BITACORA"; then
    say "Entra, pero no puede abrir $DIRECTORIO. O no existe, o no es de"
    say "$USUARIO. Las ordenes de la etapa 2 lo dejan como tiene que estar:"
    note "  sudo mkdir -p $DIRECTORIO && sudo chown $USUARIO:$USUARIO $DIRECTORIO"
  elif grep -qiE 'protocol version mismatch|is your shell clean' "$BITACORA"; then
    say "Algo escribe por la conexion antes que rsync, y rsync lee esa basura"
    say "donde esperaba su protocolo. Casi siempre es el shell del usuario:"
    say "'adduser --system' deja /usr/sbin/nologin, y sshd lanza la orden"
    say "forzada A TRAVES del shell, asi que lo que viaja es el aviso de"
    say "nologin. Miralo y arreglalo asi:"
    note "  sudo getent passwd $USUARIO      # el ultimo campo es el shell"
    note "  sudo chsh -s /bin/sh $USUARIO"
    say "Darle shell no le abre la puerta: command=+restrict sigue delante, y"
    say "la comprobacion de aqui al lado lo verifica."
    say "Si el shell ya era correcto, busca un motd o un .profile que imprima."
  elif grep -qi 'unexpectedly closed' "$BITACORA"; then
    say "La conexion se corta antes de hablar: casi siempre es que la orden"
    say "forzada no es rrsync, o que lleva argumentos que rrsync no admite."
    say "La linea tiene que ser EXACTAMENTE la de la etapa 3."
  else
    say "No reconozco ese fallo. Repitelo a mano con mas detalle:"
    note "  RSYNC_RSH='$SSH_PRUEBA -v' rsync --list-only $USUARIO@$ANFITRION:"
  fi
  printf '\n'
  say "Arreglalo en otra terminal y responde que si: se repite la prueba SOLA,"
  say "con esta misma clave. No hace falta volver a empezar."
  # `confirm ... && continue` a secas no vale: con `set -e`, la lista entera
  # devuelve 1 cuando la respuesta es que no, y eso aborta el guion.
  if confirm "Reintentar la prueba?"; then continue; fi
  confirm "Seguir sin la prueba (no recomendado)?" || exit 1
  break
fi
done
# La mitad que de verdad importa: pedirle algo que no sea rsync y ver si lo
# ejecuta. Con la orden forzada delante, sshd lanza rrsync con la peticion en
# SSH_ORIGINAL_COMMAND, rrsync ve que no es un rsync y muere.
#
# NO se comprueba con `ssh host true`, que es lo primero que se le ocurre a
# cualquiera: rrsync lleva ese caso especial escrito —«Allow checking
# connectivity with ssh <host> true»— y sale con 0. Con la linea BIEN puesta
# daria «da consola» siempre. Y tampoco se mira el codigo de salida, que
# depende de la version de rrsync: se mira si la marca vuelve. Si vuelve, algo
# ha ejecutado un echo de verdad, y eso solo puede ser un shell.
MARCA="rfirma-consola-$$"
while [ "$($SSH_PRUEBA "$USUARIO@$ANFITRION" "echo $MARCA" 2>&1 || true)" = "$MARCA" ]; do
  warn "ESA CLAVE DA CONSOLA: la linea no lleva command=\"rrsync ...\",restrict"
  printf '\n'
  say "El rsync de arriba tambien funciona asi, y ademas de mas: sin la orden"
  say "forzada, esa clave ve el disco entero del VPS, no solo $DIRECTORIO."
  say "Miralo, desde tu acceso de administracion:"
  printf '\n'
  note "  sudo cat $HOGAR/.ssh/authorized_keys"
  printf '\n'
  say "TODAS las lineas tienen que empezar por command=\"rrsync $DIRECTORIO\","
  say "restrict y solo despues ssh-ed25519. Los dos fallos de siempre:"
  step "se copio solo la parte de la clave y se quedo fuera el command=;"
  step "hay DOS lineas, una completa y otra desnuda de un intento anterior."
  note "  Basta una desnuda para que la cuenta de consola: borra esas."
  printf '\n'
  say "La linea buena es la de la etapa 3, que sigue siendo esta:"
  printf '\n%s\n\n' "$LINEA"
  confirm "Arreglado? Se vuelve a comprobar" || exit 1
done
note "correcto: la clave no da consola"
pause "Enter para continuar"

# ── 7 ─────────────────────────────────────────────────────────────────────
stage "El entorno release en GitHub"
say "La clave privada es un SECRETO y va al entorno 'release', el mismo que"
say "guarda la subclave de firma: solo la ve un job que lo declare."
say "Las otras tres son VARIABLES de repositorio, publicas a proposito: un"
say "usuario, un nombre de maquina y una clave publica de anfitrion."
if ! gh api "repos/{owner}/{repo}/environments/$ENTORNO" >/dev/null 2>&1; then
  if confirm "El entorno '$ENTORNO' no existe. Crearlo?"; then
    gh api --method PUT "repos/{owner}/{repo}/environments/$ENTORNO" >/dev/null
    note "entorno $ENTORNO creado"
  fi
fi
SECRETO_PUESTO=0
set_env_secret PUBLISH_SSH_KEY "$CLAVE" && SECRETO_PUESTO=1
set_var PUBLISH_SSH_USER "$USUARIO"
set_var PUBLISH_SSH_HOST "$ANFITRION"
set_var PUBLISH_SSH_KNOWN_HOSTS "$(cat "$CONOCIDO")"
pause "Enter para continuar"

# ── 8 ─────────────────────────────────────────────────────────────────────
stage "El montaje en Coolify, y la limpieza"
say "Coolify sirve la landing desde la imagen, pero los tres repositorios no"
say "estan dentro de ella: son el montaje del anfitrion. Sin el, /apt/, /rpm/"
say "y /rfirma.asc devuelven 404 y la landing sigue funcionando igual, que es"
say "el fallo silencioso que hay que evitar."
printf '\n'
step "En la aplicacion de rfirma.sgomez.me, Storages: anade un bind mount"
note "  $DIRECTORIO del anfitrion  ->  $DIRECTORIO del contenedor, READ ONLY."
note "  De solo lectura porque el contenedor solo lee: quien escribe es rrsync"
note "  desde fuera y con otro usuario."
printf '\n'
if [ "$SECRETO_PUESTO" -eq 1 ]; then
  note "La clave privada ya esta en el secreto: aqui no hace falta guardarla."
  note "Si algun dia hay que rehacerla, se vuelve a ejecutar este guion."
else
  warn "el secreto NO se dio de alta: la clave privada sigue en $CLAVE"
  warn "subela tu (gh secret set PUBLISH_SSH_KEY --env $ENTORNO < $CLAVE) y borrala"
fi
pause "Enter para terminar"

if [ "$SECRETO_PUESTO" -eq 1 ]; then
  find "$CLAVE_DIR" -type f -exec shred -u {} + 2>/dev/null || rm -rf "$CLAVE_DIR"
  rm -rf "$CLAVE_DIR"
fi

# La biblioteca da por hecho que el guion ha escrito algo en `.env`, y este no
# escribe nada a proposito. Su primera linea de resumen es una lista `&&` que
# evalua a falso cuando no hay nada escrito, y con `set -e` eso aborta el guion
# justo antes de contar lo que ha hecho.
set +e
finish
