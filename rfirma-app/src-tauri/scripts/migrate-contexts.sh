#!/usr/bin/env bash
# Migración del backend a cinco contextos con capas visibles (#449). Se ejecuta
# desde rfirma-app/src-tauri/ y no hace nada si el árbol antiguo ya no existe.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f src/app/mod.rs ]; then
    echo "migrate-contexts: el árbol ya está migrado; nada que hacer"
    exit 0
fi

# ---------------------------------------------------------------------------
# 1. Tabla de movimientos: origen destino, relativos a src/. Los x/tests.rs
#    hermanos se arrastran solos.
# ---------------------------------------------------------------------------
MOVES='
pkcs11/certificate.rs identity/domain/certificate.rs
pkcs11/error.rs identity/domain/error.rs
app/certificates.rs identity/application/certificates.rs
memory/listed.rs identity/application/listed.rs
pkcs11/mod.rs identity/adapters/pkcs11/mod.rs
pkcs11/stores.rs identity/adapters/pkcs11/stores.rs
pkcs11/secret.rs identity/adapters/pkcs11/secret.rs
pkcs11/nss.rs identity/adapters/pkcs11/nss.rs
commands/identity.rs identity/adapters/tauri.rs
commands/views/identity.rs identity/adapters/views.rs

destination/mod.rs documents/domain/destination.rs
destination/naming.rs documents/domain/naming.rs
destination/error.rs documents/domain/error.rs
memory/handles.rs documents/domain/handles.rs
dropped.rs documents/domain/dropped.rs
app/documents.rs documents/application/documents.rs
app/in_hand.rs documents/application/in_hand.rs
app/recents.rs documents/application/recents.rs
app/rubric.rs documents/application/rubric.rs
memory/opened.rs documents/application/opened.rs
destination/portal.rs documents/adapters/portal.rs
memory/recents.rs documents/adapters/recents_store.rs
rubric/normalize.rs documents/adapters/rubric/normalize.rs
rubric/store.rs documents/adapters/rubric/store.rs
rubric/error.rs documents/adapters/rubric/error.rs
rubric/mod.rs documents/adapters/rubric/mod.rs
commands/documents.rs documents/adapters/tauri.rs
commands/rubric.rs documents/adapters/tauri_rubric.rs
commands/views/documents.rs documents/adapters/views.rs

signing/config.rs signing/domain/config.rs
signing/placement.rs signing/domain/placement.rs
signing/admissibility.rs signing/domain/admissibility.rs
signing/layer2_text.rs signing/domain/layer2_text.rs
signing/properties.rs signing/domain/properties.rs
signing/session_seal.rs signing/domain/session_seal.rs
signing/language.rs signing/domain/language.rs
signing/mod.rs signing/domain/mod.rs
app/cycle.rs signing/application/cycle.rs
app/signing/mod.rs signing/application/session.rs
app/preview.rs signing/application/preview.rs
app/configuration.rs signing/application/configuration.rs
memory/configuration.rs signing/application/configuration_memory.rs
memory/state.rs signing/application/state.rs
app/filtering.rs signing/application/filtering.rs
app/policies.rs signing/application/policies.rs
ffi.rs signing/adapters/ffi.rs
isolate.rs signing/adapters/isolate.rs
app/engines.rs signing/adapters/engines.rs
memory/store.rs signing/adapters/store.rs
memory/error.rs signing/adapters/memory_error.rs
commands/signing.rs signing/adapters/tauri.rs
commands/orders.rs signing/adapters/orders.rs
commands/views/signing.rs signing/adapters/views.rs

protocol/mod.rs site/domain/protocol/mod.rs
protocol/codes.rs site/domain/protocol/codes.rs
protocol/filters.rs site/domain/protocol/filters.rs
protocol/launch.rs site/domain/protocol/launch.rs
protocol/message.rs site/domain/protocol/message.rs
protocol/operation.rs site/domain/protocol/operation.rs
protocol/parameters.rs site/domain/protocol/parameters.rs
protocol/refusal.rs site/domain/protocol/refusal.rs
protocol/url.rs site/domain/protocol/url.rs
protocol/version.rs site/domain/protocol/version.rs
protocol/visible.rs site/domain/protocol/visible.rs
trust/mod.rs site/domain/trust.rs
trust/error.rs site/domain/trust_error.rs
tls/error.rs site/domain/tls_error.rs
app/errand/mod.rs site/application/errand/mod.rs
app/errand/desk.rs site/application/errand/desk.rs
app/errand/outcome.rs site/application/errand/outcome.rs
app/errand/replies.rs site/application/errand/replies.rs
app/errand/request.rs site/application/errand/request.rs
app/errand/state.rs site/application/errand/state.rs
app/errand/ports.rs site/ports.rs
app/startup/mod.rs site/application/startup/mod.rs
app/startup/channel.rs site/application/startup/channel.rs
app/startup/repair.rs site/application/startup/repair.rs
app/site.rs site/application/site.rs
app/signing/site.rs site/application/session.rs
app/frontier.rs site/application/frontier.rs
app/trust.rs site/application/trust.rs
channel/mod.rs site/adapters/channel/mod.rs
channel/bind.rs site/adapters/channel/bind.rs
channel/conversation.rs site/adapters/channel/conversation.rs
channel/error.rs site/adapters/channel/error.rs
channel/reply.rs site/adapters/channel/reply.rs
channel/server.rs site/adapters/channel/server.rs
tls/authority.rs site/adapters/tls/authority.rs
tls/server.rs site/adapters/tls/server.rs
tls/store.rs site/adapters/tls/store.rs
tls/mod.rs site/adapters/tls/mod.rs
trust/nss.rs site/adapters/nss.rs
app/codec.rs site/adapters/codec.rs
app/transport.rs site/adapters/transport.rs
commands/site.rs site/adapters/tauri.rs
commands/site_window.rs site/adapters/window.rs
commands/views_site.rs site/adapters/views.rs

desktop/error.rs desktop/domain/error.rs
app/handlers.rs desktop/application/handlers.rs
app/invocation.rs desktop/application/invocation.rs
app/version.rs desktop/application/version.rs
desktop/mod.rs desktop/adapters/channel.rs
desktop/choice.rs desktop/adapters/choice.rs
paths.rs desktop/adapters/paths.rs
releases.rs desktop/adapters/releases.rs
commands/desktop.rs desktop/adapters/tauri.rs
commands/views/desktop.rs desktop/adapters/views.rs

app/fixtures.rs fixtures.rs
app/tests.rs tests.rs
memory/tests.rs tests/memory.rs
'

tests_sibling_of() {
    case "$1" in
        */mod.rs) echo "${1%/mod.rs}/tests.rs" ;;
        *.rs) echo "${1%.rs}/tests.rs" ;;
    esac
}

move() {
    mkdir -p "src/$(dirname "$2")"
    git mv "src/$1" "src/$2"
}

echo "$MOVES" | while read -r origin destination; do
    [ -n "$origin" ] || continue
    sibling="$(tests_sibling_of "$origin")"
    if [ -f "src/$sibling" ] && [ "$origin" != "app/tests.rs" ] && [ "$origin" != "memory/tests.rs" ]; then
        move "$sibling" "$(tests_sibling_of "$destination")"
    fi
    move "$origin" "$destination"
done

# Los antiguos ficheros de reparto que desaparecen: sus cuerpos se funden en lib.rs.
OLD_APP_MOD=$(mktemp); cp src/app/mod.rs "$OLD_APP_MOD"
OLD_MEMORY_MOD=$(mktemp); cp src/memory/mod.rs "$OLD_MEMORY_MOD"
OLD_COMMANDS_MOD=$(mktemp); cp src/commands/mod.rs "$OLD_COMMANDS_MOD"
git rm -q src/app/mod.rs src/memory/mod.rs src/commands/mod.rs src/commands/views.rs

# ---------------------------------------------------------------------------
# 2. Reescritura de caminos de crate. Cada regla es «viejo nuevo» sin `crate::`;
#    los largos van antes que los cortos, y el resultado se escribe con `%%`
#    como separador para que ninguna regla posterior lo vuelva a tocar.
# ---------------------------------------------------------------------------
commands_of() {
    sed -n "/^pub use $1::{/,/^};/p" "$OLD_COMMANDS_MOD" \
        | tr ',' '\n' | tr -d ' {};' | sed -n 's/^pub use .*//; /^[a-z][a-z_]*$/p'
}

RULES=$(
    for name in $(commands_of identity); do echo "commands::$name identity::adapters::tauri::$name"; done
    for name in $(commands_of documents); do echo "commands::$name documents::adapters::tauri::$name"; done
    for name in $(commands_of signing); do echo "commands::$name signing::adapters::tauri::$name"; done
    for name in $(commands_of site); do echo "commands::$name site::adapters::tauri::$name"; done
    for name in $(commands_of desktop); do echo "commands::$name desktop::adapters::tauri::$name"; done
    cat <<'EOF'
pkcs11::certificate identity::domain::certificate
pkcs11::error identity::domain::error
pkcs11::nss::NssHost identity::ports::NssHost
pkcs11::NssHost identity::ports::NssHost
app::certificates identity::application::certificates
memory::listed identity::application::listed
memory::ListedCertificates identity::application::listed::ListedCertificates
commands::views::identity identity::adapters::views
commands::views::CertificateView identity::adapters::views::CertificateView
commands::views::SecretView identity::adapters::views::SecretView
commands::views::store_name identity::adapters::views::store_name
commands::CertificateView identity::adapters::views::CertificateView
commands::SecretView identity::adapters::views::SecretView
commands::identity identity::adapters::tauri
destination::portal documents::adapters::portal
destination::PortalDocument documents::adapters::portal::PortalDocument
destination::the_original_folder_can_be_offered documents::adapters::portal::the_original_folder_can_be_offered
destination::naming documents::domain::naming
destination::error documents::domain::error
memory::handles documents::domain::handles
app::documents documents::application::documents
app::in_hand documents::application::in_hand
app::recents documents::application::recents
app::rubric documents::application::rubric
memory::opened documents::application::opened
memory::OpenedDocuments documents::application::opened::OpenedDocuments
memory::Remembrance documents::application::opened::Remembrance
memory::recents documents::adapters::recents_store
memory::Badge documents::adapters::recents_store::Badge
memory::Placement documents::adapters::recents_store::Placement
memory::RecentDocument documents::adapters::recents_store::RecentDocument
memory::Recents documents::adapters::recents_store::Recents
memory::ShownBadge documents::adapters::recents_store::ShownBadge
memory::CAPACITY documents::adapters::recents_store::CAPACITY
commands::documents documents::adapters::tauri
commands::rubric documents::adapters::tauri_rubric
commands::RubricChoiceView documents::adapters::tauri_rubric::RubricChoiceView
commands::RubricView documents::adapters::tauri_rubric::RubricView
commands::views::documents documents::adapters::views
commands::views::DestinationView documents::adapters::views::DestinationView
commands::views::DroppedDocumentView documents::adapters::views::DroppedDocumentView
commands::views::OpenedDocumentView documents::adapters::views::OpenedDocumentView
commands::views::RecentDocumentView documents::adapters::views::RecentDocumentView
commands::views::SignedDocumentView documents::adapters::views::SignedDocumentView
commands::DestinationView documents::adapters::views::DestinationView
commands::DroppedDocumentView documents::adapters::views::DroppedDocumentView
commands::OpenedDocumentView documents::adapters::views::OpenedDocumentView
commands::RecentDocumentView documents::adapters::views::RecentDocumentView
commands::SignedDocumentView documents::adapters::views::SignedDocumentView
commands::dropped_document documents::application::documents::dropped_document
app::cycle signing::application::cycle
app::signing::site site::application::session
app::signing::SiteSigning site::application::session::SiteSigning
app::signing::SiteRefusal site::application::session::SiteRefusal
app::signing::SiteSignature site::application::session::SiteSignature
app::signing::begin_for_the_site site::application::session::begin_for_the_site
app::signing::finish_for_the_site site::application::session::finish_for_the_site
app::signing signing::application::session
app::preview signing::application::preview
app::configuration signing::application::configuration
memory::configuration signing::application::configuration_memory
memory::Configuration signing::application::configuration_memory::Configuration
memory::Theme signing::application::configuration_memory::Theme
memory::state signing::application::state
memory::BoxSize signing::application::state::BoxSize
memory::RememberedFields signing::application::state::RememberedFields
memory::State signing::application::state::State
memory::VersionCheck signing::application::state::VersionCheck
memory::VisibleSignatureMemory signing::application::state::VisibleSignatureMemory
app::filtering::FilterEngine signing::ports::FilterEngine
app::filtering signing::application::filtering
app::policies::PolicyEngine signing::ports::PolicyEngine
app::policies signing::application::policies
app::engines signing::adapters::engines
memory::store signing::adapters::store
memory::Damage signing::adapters::store::Damage
memory::JsonFile signing::adapters::store::JsonFile
memory::Loaded signing::adapters::store::Loaded
memory::Recovery signing::adapters::store::Recovery
memory::FORMAT_VERSION signing::adapters::store::FORMAT_VERSION
memory::error signing::adapters::memory_error
memory::MemoryError signing::adapters::memory_error::MemoryError
memory::Situation signing::adapters::memory_error::Situation
memory::Memory Memory
commands::signing signing::adapters::tauri
commands::orders signing::adapters::orders
commands::PlacementOrder signing::adapters::orders::PlacementOrder
commands::SigningOrder signing::adapters::orders::SigningOrder
commands::views::signing signing::adapters::views
commands::views::ConfigurationView signing::adapters::views::ConfigurationView
commands::views::PlacementView signing::adapters::views::PlacementView
commands::views::StatusView signing::adapters::views::StatusView
commands::ConfigurationView signing::adapters::views::ConfigurationView
commands::PlacementView signing::adapters::views::PlacementView
commands::SigningSession signing::application::session::SigningSession
trust::error site::domain::trust_error
trust::TrustError site::domain::trust_error::TrustError
trust::nss site::adapters::nss
trust::NssTrustStores site::adapters::nss::NssTrustStores
trust::TrustStores site::ports::TrustStores
tls::error site::domain::tls_error
tls::TlsError site::domain::tls_error::TlsError
app::errand::ports site::ports
app::errand site::application::errand
app::startup site::application::startup
app::site site::application::site
app::frontier site::application::frontier
app::trust site::application::trust
app::codec site::adapters::codec
app::transport site::adapters::transport
commands::site_window site::adapters::window
commands::attend_site_operation site::adapters::window::attend_site_operation
commands::open_the_site_window site::adapters::window::open_the_site_window
commands::publish_the_moment site::adapters::window::publish_the_moment
commands::SITE_ERRAND site::adapters::window::SITE_ERRAND
commands::SITE_WINDOW site::adapters::window::SITE_WINDOW
commands::site site::adapters::tauri
commands::views_site site::adapters::views
commands::NoCertificateView site::adapters::views::NoCertificateView
commands::NoChannelView site::adapters::views::NoChannelView
commands::RefusalSituationView site::adapters::views::RefusalSituationView
commands::SignatureRoundView site::adapters::views::SignatureRoundView
commands::SiteErrandView site::adapters::views::SiteErrandView
commands::SiteOutcomeView site::adapters::views::SiteOutcomeView
commands::SiteStageView site::adapters::views::SiteStageView
desktop::error desktop::domain::error
desktop::choice desktop::adapters::choice
app::handlers desktop::application::handlers
app::invocation desktop::application::invocation
app::version desktop::application::version
commands::desktop desktop::adapters::tauri
commands::views::desktop desktop::adapters::views
commands::views::NewVersionView desktop::adapters::views::NewVersionView
commands::views::UrlHandlerView desktop::adapters::views::UrlHandlerView
commands::views::UrlHandlersView desktop::adapters::views::UrlHandlersView
commands::NewVersionView desktop::adapters::views::NewVersionView
commands::UrlHandlerView desktop::adapters::views::UrlHandlerView
commands::UrlHandlersView desktop::adapters::views::UrlHandlersView
commands::PendingInvocation desktop::application::invocation::PendingInvocation
commands::second_invocation desktop::application::invocation::second_invocation
app::fixtures fixtures
app::Environment Environment
app::lock lock
app::chosen_folder chosen_folder
commands::views::Failure commands::Failure
pkcs11 identity::adapters::pkcs11
destination documents::domain::destination
dropped documents::domain::dropped
rubric documents::adapters::rubric
signing signing::domain
ffi signing::adapters::ffi
isolate signing::adapters::isolate
protocol site::domain::protocol
trust site::domain::trust
tls site::adapters::tls
channel site::adapters::channel
desktop desktop::adapters::channel
paths desktop::adapters::paths
releases desktop::adapters::releases
EOF
)

with_percent() { echo "$1" | sed 's/::/%%/g'; }

# Un guion de sed por prefijo: `crate::` en src/, `rfirma_lib::` en tests/ y
# los caminos a pelo de lib.rs.
SED_CRATE=$(mktemp); SED_LIB=$(mktemp); SED_ROOT=$(mktemp)
echo "$RULES" | while read -r old new; do
    [ -n "$old" ] || continue
    new_pct="$(with_percent "$new")"
    echo "s/crate::$old\\b/krate%%$new_pct/g" >> "$SED_CRATE"
    echo "s/rfirma_lib::$old\\b/rlib%%$new_pct/g" >> "$SED_LIB"
    case "$old" in
        *::*) echo "s/\\b$old\\b/krate%%$new_pct/g" >> "$SED_ROOT" ;;
        *) echo "s/\\b$old::/krate%%$new_pct%%/g" >> "$SED_ROOT" ;;
    esac
done
echo 's/krate%%/crate::/g; s/%%/::/g' >> "$SED_CRATE"
echo 's/rlib%%/rfirma_lib::/g; s/%%/::/g' >> "$SED_LIB"
echo 's/krate%%//g; s/%%/::/g' >> "$SED_ROOT"

find src -name '*.rs' ! -path 'src/lib.rs' -print0 | xargs -0 sed -i -f "$SED_CRATE"
find tests -name '*.rs' -print0 | xargs -0 sed -i -f "$SED_LIB"
sed -i -f "$SED_ROOT" "$OLD_APP_MOD" "$OLD_MEMORY_MOD"
# lib.rs nombra los módulos a pelo y sin `crate::`.
sed -i -f "$SED_ROOT" src/lib.rs

# ---------------------------------------------------------------------------
# 3. Rutas de include_str! de las guardas textuales.
# ---------------------------------------------------------------------------
sed -i \
    -e 's#include_str!("../../ffi.rs")#include_str!("../../adapters/ffi.rs")#' \
    -e 's#include_str!("../../pkcs11/mod.rs")#include_str!("../../../identity/adapters/pkcs11/mod.rs")#' \
    src/signing/application/cycle/tests.rs
sed -i \
    -e 's#include_str!("mod.rs")#include_str!("../session.rs")#' \
    -e 's#"app/signing/mod.rs"#"signing/application/session.rs"#' \
    -e 's#"app/recents.rs"#"documents/application/recents.rs"#' \
    -e 's#include_str!("../recents.rs")#include_str!("../../../documents/application/recents.rs")#' \
    -e 's#"app/signing/site.rs"#"site/application/session.rs"#' \
    -e 's#include_str!("site.rs")#include_str!("../../../site/application/session.rs")#' \
    -e 's#"app/errand/\([a-z]*\).rs"#"site/application/errand/\1.rs"#' \
    -e 's#include_str!("../errand/\([a-z]*\).rs")#include_str!("../../../site/application/errand/\1.rs")#' \
    -e 's#"app/policies.rs"#"signing/application/policies.rs"#' \
    -e 's#"app/documents.rs"#"documents/application/documents.rs"#' \
    -e 's#include_str!("../documents.rs")#include_str!("../../../documents/application/documents.rs")#' \
    -e 's#"app/in_hand.rs"#"documents/application/in_hand.rs"#' \
    -e 's#include_str!("../in_hand.rs")#include_str!("../../../documents/application/in_hand.rs")#' \
    -e 's#"app/invocation.rs"#"desktop/application/invocation.rs"#' \
    -e 's#include_str!("../invocation.rs")#include_str!("../../../desktop/application/invocation.rs")#' \
    -e 's#"app/preview.rs"#"signing/application/preview.rs"#' \
    -e 's#"commands/mod.rs"#"lib.rs"#' \
    -e 's#include_str!("../../commands/mod.rs")#include_str!("../../../lib.rs")#' \
    -e 's#"commands/site_window.rs"#"site/adapters/window.rs"#' \
    -e 's#include_str!("../../commands/site_window.rs")#include_str!("../../../site/adapters/window.rs")#' \
    src/signing/application/session/tests.rs
sed -i 's#include_str!("../site.rs")#include_str!("../session.rs")#' src/site/application/session/tests.rs
sed -i 's#include_str!("../signing/mod.rs")#include_str!("../session.rs")#' src/signing/application/preview/tests.rs
sed -i 's#include_str!("../../tauri.conf.json")#include_str!("../../../../tauri.conf.json")#' src/desktop/adapters/channel/tests.rs

# ---------------------------------------------------------------------------
# 4. Cortes de bloque entero: los tres puertos y los dos repartos.
# ---------------------------------------------------------------------------
# Un bloque `pub trait X { … }` con su `///` inmediatamente encima.
trait_block() { awk -v name="$2" '
    /^\/\/\// { doc = doc $0 "\n"; next }
    $0 ~ "^pub trait " name " " { printing = 1; printf "%s", doc }
    { doc = "" }
    printing { print }
    printing && /^\}/ { exit }
' "$1"; }
drop_trait_block() {
    local file="$1" name="$2" tmp; tmp=$(mktemp)
    awk -v name="$2" '
        /^\/\/\// { doc = doc $0 "\n"; next }
        $0 ~ "^pub trait " name " " { dropping = 1; doc = ""; next }
        { printf "%s", doc; doc = "" }
        dropping && /^\}/ { dropping = 0; skip_blank = 1; next }
        dropping { next }
        skip_blank && /^$/ { skip_blank = 0; next }
        { skip_blank = 0; print }
    ' "$file" > "$tmp"
    mv "$tmp" "$file"
}

NSS=src/identity/adapters/pkcs11/nss.rs
{
    echo '//! Puertos del contexto de identidad.'
    echo
    echo 'use libloading::Library;'
    echo
    echo 'use crate::identity::adapters::pkcs11::nss::NssUnavailable;'
    echo
    trait_block "$NSS" NssHost
} > src/identity/ports.rs
drop_trait_block "$NSS" NssHost
sed -i 's/^use libloading::Library;$/use libloading::Library;\n\nuse crate::identity::ports::NssHost;/' "$NSS"

FILTERING=src/signing/application/filtering.rs
POLICIES=src/signing/application/policies.rs
{
    echo '//! Puertos del contexto de firma: los dos motores que presta el puente.'
    echo
    echo 'use crate::signing::adapters::ffi::BridgeError;'
    echo
    trait_block "$FILTERING" FilterEngine
    echo
    trait_block "$POLICIES" PolicyEngine
} > src/signing/ports.rs
drop_trait_block "$FILTERING" FilterEngine
drop_trait_block "$POLICIES" PolicyEngine
sed -i 's/^use crate::signing::adapters::ffi::BridgeError;$/use crate::signing::adapters::ffi::BridgeError;\nuse crate::signing::ports::FilterEngine;/' "$FILTERING"
sed -i 's/^use crate::signing::adapters::ffi::BridgeError;$/use crate::signing::adapters::ffi::BridgeError;\nuse crate::signing::ports::PolicyEngine;/' "$POLICIES"

TRUST=src/site/domain/trust.rs
PORTS=src/site/ports.rs
{
    sed '1s/.*/\/\/! Puertos del contexto de sede: códec, transporte y almacenes de confianza (ADR-0017)./' "$PORTS" \
        | sed '/^#\[cfg(test)\]$/,$d' \
        | sed 's/^use std::sync::Arc;$/use std::path::Path;\nuse std::sync::Arc;/' \
        | sed 's/^use super::outcome::SiteOutcome;$/use crate::site::application::errand::outcome::SiteOutcome;/' \
        | sed 's/^use super::request::SiteRequest;$/use crate::site::application::errand::request::SiteRequest;/' \
        | sed 's/^use crate::site::domain::protocol::AfirmaUrl;$/use crate::site::domain::protocol::AfirmaUrl;\nuse crate::site::domain::trust_error::TrustError;/'
    trait_block "$TRUST" TrustStores
    echo
    echo '#[cfg(test)]'
    echo 'mod tests;'
} > "$PORTS.new"
mv "$PORTS.new" "$PORTS"
drop_trait_block "$TRUST" TrustStores

# Los repartos que cambian de sitio: `pub mod` fuera y `pub use` reapuntados.
sed -i \
    -e '/^pub mod certificate;$/d' -e '/^pub mod error;$/d' \
    -e 's/^pub use certificate::/pub use crate::identity::domain::certificate::/' \
    -e 's/^pub use error::/pub use crate::identity::domain::error::/' \
    -e 's/^pub use nss::{NssHost, /pub use nss::{/' \
    src/identity/adapters/pkcs11/mod.rs
sed -i \
    -e '/^pub mod error;$/d' -e '/^pub mod naming;$/d' -e '/^pub mod portal;$/d' \
    -e 's/^pub use error::/pub use super::error::/' \
    -e 's/^pub use naming::/pub use super::naming::/' \
    -e '/^pub use portal::/d' \
    src/documents/domain/destination.rs
sed -i \
    -e '/^pub mod error;$/d' -e '/^pub mod nss;$/d' \
    -e 's/^pub use error::/pub use super::trust_error::/' \
    -e '/^pub use nss::NssTrustStores;$/d' \
    "$TRUST"
sed -i \
    -e '/^pub mod error;$/d' \
    -e 's/^pub use error::/pub use crate::site::domain::tls_error::/' \
    src/site/adapters/tls/mod.rs
sed -i -e '/^pub mod choice;$/d' -e '/^pub mod error;$/d' src/desktop/adapters/channel.rs
sed -i \
    -e '/^pub mod ports;$/d' \
    -e 's/^pub use ports::/pub use crate::site::ports::/' \
    src/site/application/errand/mod.rs
sed -i -e '/^pub mod site;$/d' -e '/^pub use site::{/d' src/signing/application/session.rs
sed -i 's/^use super::error::/use crate::identity::domain::error::/' "$NSS"
sed -i 's/^use super::error::/use crate::site::domain::trust_error::/; s/^use super::TrustStores;/use crate::site::ports::TrustStores;/' src/site/adapters/nss.rs
sed -i 's/^use super::error::/use crate::site::domain::tls_error::/' src/site/adapters/tls/authority.rs src/site/adapters/tls/store.rs
sed -i 's/^use super::error::/use crate::signing::adapters::memory_error::/' src/signing/adapters/store.rs
sed -i 's/^use super::handles::mint;/use crate::documents::domain::handles::mint;/' src/identity/application/listed.rs src/documents/application/opened.rs
sed -i 's/^use super::recents::Recents;/use crate::documents::adapters::recents_store::Recents;/' src/signing/application/state.rs
sed -i 's/^use super::{content_type_for, Channel};/use super::channel::{content_type_for, Channel};/' src/desktop/adapters/choice.rs

# ---------------------------------------------------------------------------
# 5. Los mod.rs nuevos: uno por contexto y uno por capa, desde el árbol.
# ---------------------------------------------------------------------------
layer_mod() {
    local dir="$1" header="$2"
    {
        echo "//! $header"
        echo
        for entry in "$dir"/*; do
            name="$(basename "$entry")"
            case "$name" in
                mod.rs|tests.rs|AGENTS.md) continue ;;
                *.rs) echo "pub mod ${name%.rs};" ;;
                *) [ -f "$entry/mod.rs" ] && echo "pub mod $name;" ;;
            esac
        done
    } > "$dir/mod.rs"
}

for context in identity documents signing site desktop; do
    [ -f "src/$context/domain/mod.rs" ] || layer_mod "src/$context/domain" "Dominio de \`$context\`: reglas puras, sin nada del crate fuera de esta carpeta."
    layer_mod "src/$context/application" "Casos de uso de \`$context\`."
    layer_mod "src/$context/adapters" "Adaptadores de \`$context\`: todo lo que toca el mundo, incluidas las órdenes y las vistas de Tauri."
    {
        echo "//! Contexto \`$context\` (ADR-0017)."
        echo
        echo 'pub mod adapters;'
        echo 'pub mod application;'
        echo 'pub mod domain;'
        [ -f "src/$context/ports.rs" ] && echo 'pub mod ports;'
    } > "src/$context/mod.rs"
done

# ---------------------------------------------------------------------------
# 6. lib.rs: los cinco contextos, `commands/` reducido a dos ficheros, y los
#    dos repartos fundidos (Environment y Memory).
# ---------------------------------------------------------------------------
LIB=$(mktemp)
{
    echo '//! Composición y arranque de la aplicación Tauri: junta las raíces de los cinco contextos.'
    echo
    echo 'pub mod desktop;'
    echo 'pub mod documents;'
    echo 'pub mod identity;'
    echo 'pub mod signing;'
    echo 'pub mod site;'
    echo
    echo 'pub mod commands {'
    echo '    pub mod failure;'
    echo
    echo '    #[cfg(test)]'
    echo '    mod guards;'
    echo
    echo '    pub use failure::Failure;'
    echo '}'
    echo
    echo '#[cfg(test)]'
    echo 'pub(crate) mod fixtures;'
    echo '#[cfg(test)]'
    echo 'mod tests;'
    echo
    echo 'use std::sync::Mutex;'
    echo
    echo 'use desktop::adapters::paths::Paths;'
    echo 'use documents::domain::destination::DestinationFolder;'
    echo 'use identity::application::listed::ListedCertificates;'
    echo 'use signing::adapters::memory_error::MemoryError;'
    echo 'use signing::adapters::store::{JsonFile, Loaded};'
    echo 'use signing::application::configuration_memory::Configuration;'
    echo 'use signing::application::state::{State, VersionCheck};'
    echo
    sed -n '/^\/\/\/ Variable de entorno/,/^pub const PKCS11_MODULE_VARIABLE/p' src/lib.rs
    echo
    echo '/// Nombre del evento emitido cuando se suelta un documento en la ventana.'
    echo 'pub const DOCUMENT_DROPPED: &str = "document-dropped";'
    echo
    sed -n '/^\/\/\/ Entorno de composición/,$p' "$OLD_APP_MOD" | sed '/^#\[cfg(test)\]$/,$d'
    sed -n '/^\/\/\/ Las dos memorias/,$p' "$OLD_MEMORY_MOD" | sed '/^#\[cfg(test)\]$/,$d'
    sed -n '/^\/\/\/ Punto de entrada compartido/,$p' src/lib.rs
} > "$LIB"
mv "$LIB" src/lib.rs
sed -i 's/\bcommands::DOCUMENT_DROPPED\b/DOCUMENT_DROPPED/g' src/lib.rs

# La orden de Tauri se registra por su ruta entera; `generate_handler!` la
# acepta igual que antes aceptaba `commands::nombre`.
sed -i 's/^use super::{chosen_folder, Environment};$/use crate::{chosen_folder, Environment};\n\nmod memory;/' src/tests.rs
sed -i 's/^use super::\*;$/use crate::*;/' src/tests/memory.rs

rm -f "$OLD_APP_MOD" "$OLD_MEMORY_MOD" "$OLD_COMMANDS_MOD" "$SED_CRATE" "$SED_LIB" "$SED_ROOT"
find src -type d -empty -delete

echo "migrate-contexts: árbol movido; ahora \`cargo check\` y arreglar solo lo que señale"
