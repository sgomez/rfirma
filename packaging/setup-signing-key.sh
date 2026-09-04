#!/usr/bin/env bash
#
# LA CLAVE DE FIRMA DE rFIRMA, DE PRINCIPIO A FIN.
#
# Este guion no lo ejecuta ningun CI: lo ejecuta UNA PERSONA, una sola vez, en
# su equipo. Es la parte del ADR-0015 que un agente no puede hacer —generar la
# clave, guardarla fuera de linea y dar de alta los secretos— escrita para que
# no haya que reconstruirla de memoria dentro de dos anos, ni cuando toque
# revocar.
#
# QUE PRODUCE:
#   - una clave MAESTRA, solo de certificacion, que se queda FUERA DE LINEA;
#   - una SUBCLAVE de firma, que es lo unico que baja al CI;
#   - un certificado de revocacion, que se guarda con la maestra;
#   - `rfirma.asc`, la clave publica que sirve la landing y que consumen el
#     `Signed-By` de apt y el `gpgkey` de dnf;
#   - los dos secretos y la variable del entorno `release`:
#     GPG_SIGNING_SUBKEY, GPG_SIGNING_PASSPHRASE y GPG_FINGERPRINT.
#
# POR QUE UNA SOLA CLAVE: firma las Releases, el repositorio ostree, el indice
# de apt, el de dnf y cada `.rpm`. Dos raices de confianza para el mismo
# enunciado —«esto lo hizo rFirma»— son peor seguridad, no mejor (ID-169).
#
# POR QUE LA MAESTRA NO BAJA AL CI: si se filtra la subclave, se revoca, se
# emite otra bajo la misma maestra y la huella que la gente anadio a su
# `Signed-By` SIGUE VALIENDO. Con la maestra dentro, una filtracion obliga a
# todo el mundo a volver a anadir el repositorio a mano, que en un cliente de
# firma electronica es el peor final posible de un incidente.
#
# Uso: packaging/setup-signing-key.sh
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

TOTAL_STAGES=6

# Los secretos de este guion NO son valores que se peguen de un navegador: los
# produce gpg aqui mismo. Por eso no se escribe ni una linea en `.env` —un
# fichero con la subclave privada dentro del arbol de trabajo es exactamente lo
# que se esta intentando evitar— y por eso hace falta un ayudante que la
# biblioteca no trae: los secretos van al ENTORNO `release`, no al repositorio,
# que es lo que hace que un job sin `environment: release` no pueda verlos.
ENV_FILE="$(mktemp -u)"   # nunca se escribe; solo evita tocar un .env real

ENTORNO=release

set_env_secret() {
  local name="$1" fichero="$2"
  if gh secret set "$name" --env "$ENTORNO" < "$fichero" >/dev/null 2>&1; then
    WRITTEN_SECRET+=("$name")
    printf '  %s✓ puesto%s el secreto %s en el entorno %s\n' "$GREEN" "$RESET" "$name" "$ENTORNO"
    return
  fi
  SKIPPED+=("secreto $name (a mano: gh secret set $name --env $ENTORNO < $fichero)")
  warn "no se pudo poner el secreto $name"
}

banner "La clave de firma de rFirma"

# ── 1 ─────────────────────────────────────────────────────────────────────
stage "Antes de empezar"
say "Vas a crear la clave que firma TODO lo verificable por humanos de rFirma:"
say "las Releases, el repositorio ostree, el indice de apt, el de dnf y cada"
say ".rpm. Una sola clave, y su maestra no volvera a tocar un sistema"
say "conectado (ADR-0015)."
printf '\n'
step "Ten a mano el gestor de contrasenas: la frase de paso se guarda ahi."
step "Ten a mano una unidad extraible o un directorio cifrado para la maestra."
printf '\n'
for herramienta in gpg gh; do
  if ! command -v "$herramienta" >/dev/null 2>&1; then
    warn "falta $herramienta, y sin el no se puede seguir"
    exit 1
  fi
done
if ! gh auth status >/dev/null 2>&1; then
  warn "gh no esta autenticado: ejecuta 'gh auth login' y vuelve"
  exit 1
fi
note "gpg y gh listos"
pause "Enter para continuar"

# ── 2 ─────────────────────────────────────────────────────────────────────
stage "El anillo de la maestra"
say "La maestra no se mezcla con tu anillo personal: vive en un directorio"
say "suyo que puedes copiar entero a la unidad extraible y borrar de aqui."
ask ANILLO "Directorio para el anillo (Enter para \$HOME/rfirma-gpg-offline):"
ANILLO="${ANILLO:-$HOME/rfirma-gpg-offline}"
if [ -e "$ANILLO" ] && [ -n "$(ls -A "$ANILLO" 2>/dev/null)" ]; then
  warn "$ANILLO ya existe y no esta vacio"
  confirm "Seguir usando ese anillo (no se borra nada)?" || exit 1
fi
mkdir -p "$ANILLO"
chmod 700 "$ANILLO"
export GNUPGHOME="$ANILLO"
note "GNUPGHOME=$ANILLO durante el resto del guion"
pause "Enter para continuar"

# ── 3 ─────────────────────────────────────────────────────────────────────
stage "La clave maestra, solo de certificacion"
say "Identidad de la clave. Es publica y para siempre: pon un correo del"
say "proyecto, nunca uno personal."
ask UID_CLAVE "Identidad (por ejemplo: rFirma signing <rfirma@ejemplo.org>):"
if [ -z "${UID_CLAVE:-}" ]; then
  warn "sin identidad no hay clave"
  exit 1
fi
printf '\n'
say "La frase de paso protege la maestra Y la subclave que ira al CI: el"
say "secreto GPG_SIGNING_PASSPHRASE es esta misma. Guardala en el gestor"
say "ANTES de escribirla aqui."
ask_secret FRASE "Frase de paso:"
ask_secret FRASE_OTRA_VEZ "Otra vez:"
if [ -z "${FRASE:-}" ] || [ "$FRASE" != "$FRASE_OTRA_VEZ" ]; then
  warn "las dos frases no coinciden"
  exit 1
fi
printf '\n'
say "RSA de 4096 y no una curva: el .rpm firmado lo va a verificar el rpm de"
say "cualquier distribucion, y no todas traen soporte de ed25519."
confirm "Generar la maestra (certificacion, sin caducidad) y su subclave de firma (2 anos)?" || exit 1
FRASE_FICHERO="$ANILLO/passphrase"
umask 077
printf '%s' "$FRASE" > "$FRASE_FICHERO"
gpg --batch --pinentry-mode loopback --passphrase-file "$FRASE_FICHERO" \
  --quick-generate-key "$UID_CLAVE" rsa4096 cert never
HUELLA="$(gpg --batch --with-colons --list-keys "$UID_CLAVE" \
  | awk -F: '$1 == "fpr" { print $10; exit }')"
gpg --batch --pinentry-mode loopback --passphrase-file "$FRASE_FICHERO" \
  --quick-add-key "$HUELLA" rsa4096 sign 2y
note "maestra $HUELLA creada, con su subclave de firma"
pause "Enter para continuar"

# ── 4 ─────────────────────────────────────────────────────────────────────
stage "El certificado de revocacion y la copia de la maestra"
say "El certificado de revocacion es lo que te permite matar la clave el dia"
say "que la pierdas. Sin el, una clave perdida se queda viva para siempre."
REVOCACION="$ANILLO/rfirma-revocacion.asc"
# `gpg --gen-revoke` NO funciona en modo lote: responde «can't do this in batch
# mode» y sale con 2, ni siquiera con --command-fd. No hace falta generarlo,
# porque gpg ya deja uno preparado al crear la clave, en openpgp-revocs.d.
REVOCACION_AUTOMATICA="$GNUPGHOME/openpgp-revocs.d/$HUELLA.rev"
if [ -f "$REVOCACION_AUTOMATICA" ] && cp "$REVOCACION_AUTOMATICA" "$REVOCACION"; then
  note "certificado de revocacion en $REVOCACION"
  say "Es el que gpg dejo hecho al crear la maestra: revoca la clave entera."
  say "Ojo el dia que lo uses: gpg escribe la linea '-----BEGIN...' con dos"
  say "puntos delante, a proposito, para que no se importe por accidente."
  say "Hay que quitar ese ':' antes de 'gpg --import $REVOCACION'."
else
  warn "no se encontro el certificado que gpg deja en openpgp-revocs.d"
  warn "hazlo a mano, de forma interactiva: gpg --gen-revoke $HUELLA"
  SKIPPED+=("certificado de revocacion (gpg --gen-revoke $HUELLA)")
fi
printf '\n'
step "Copia el directorio $ANILLO ENTERO a la unidad extraible o al medio cifrado."
step "Guarda ahi tambien el certificado de revocacion."
step "Anota la huella en el gestor de contrasenas: $HUELLA"
pause "Enter cuando la copia este hecha"

# ── 5 ─────────────────────────────────────────────────────────────────────
stage "La subclave para el CI, y la publica para la landing"
say "Al CI se le da SOLO la subclave: puede firmar, no puede certificar ni"
say "crear subclaves ni tocar la identidad."
SUBCLAVE="$ANILLO/subclave-ci.asc"
PUBLICA="$ANILLO/rfirma.asc"
gpg --batch --yes --pinentry-mode loopback --passphrase-file "$FRASE_FICHERO" \
  --armor --output "$SUBCLAVE" --export-secret-subkeys "$HUELLA"
gpg --batch --yes --armor --output "$PUBLICA" --export "$HUELLA"
note "subclave en $SUBCLAVE"
note "publica en $PUBLICA"
printf '\n'
say "Comprobacion: la subclave exportada NO puede llevar la maestra dentro."
# --export-secret-subkeys emite SIEMPRE el paquete de la clave primaria, pero
# vaciado: es un stub 'gnu-dummy' (protect mode 1002) sin material secreto. Por
# eso la comprobacion no puede ser «no hay ningun :secret key packet:» —eso
# saltaria en toda exportacion correcta—, sino la contraria: la primaria TIENE
# que estar ahi como stub. Si no aparece 'gnu-dummy', el fichero lleva la
# maestra de verdad.
if ! gpg --batch --list-packets "$SUBCLAVE" 2>/dev/null | grep -q 'gnu-dummy'; then
  warn "el fichero contiene la clave secreta MAESTRA: no lo subas a ninguna parte"
  exit 1
fi
note "correcto: la maestra solo esta como stub, dentro solo hay subclaves secretas"
pause "Enter para continuar"

# ── 6 ─────────────────────────────────────────────────────────────────────
stage "El entorno release en GitHub"
say "Los dos SECRETOS van al ENTORNO 'release', no al repositorio: es lo que"
say "hace que un job sin 'environment: release' —como build.yml, que es"
say "invocable— no pueda verlos jamas."
say "La huella NO es un secreto y va como variable de REPOSITORIO, a proposito:"
say "asi cualquier job puede contrastar contra ella la subclave que importa."
if ! gh api "repos/{owner}/{repo}/environments/$ENTORNO" >/dev/null 2>&1; then
  if confirm "El entorno '$ENTORNO' no existe. Crearlo?"; then
    gh api --method PUT "repos/{owner}/{repo}/environments/$ENTORNO" >/dev/null
    note "entorno $ENTORNO creado"
  fi
fi
set_env_secret GPG_SIGNING_SUBKEY "$SUBCLAVE"
set_env_secret GPG_SIGNING_PASSPHRASE "$FRASE_FICHERO"
set_var GPG_FINGERPRINT "$HUELLA"
printf '\n'
warn "queda por hacer a mano, y no es opcional:"
step "Anade un revisor humano al entorno '$ENTORNO' (Settings > Environments)."
step "Restringe quien puede empujar etiquetas v* (Settings > Rules > Tags)."
step "Publica $PUBLICA como https://rfirma.sgomez.me/rfirma.asc"
step "Escribe la huella $HUELLA en SECURITY.md y en packaging/repo/index.html."
step "Borra $ANILLO de este equipo cuando la copia fuera de linea este hecha."
pause "Enter para terminar"

shred -u "$FRASE_FICHERO" 2>/dev/null || rm -f "$FRASE_FICHERO"

# La biblioteca da por hecho que el guion ha escrito algo en `.env`, y este no
# escribe nada a proposito. Su primera linea de resumen es una lista `&&` que
# evalua a falso cuando no hay nada escrito, y con `set -e` eso aborta el guion
# justo antes de contar lo que ha hecho.
set +e
finish
