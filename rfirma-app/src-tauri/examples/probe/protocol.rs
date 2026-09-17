//! El catálogo de condiciones del protocolo y el arnés que las verifica contra el sujeto.

use crate::dossier::{Dossier, ProtocolConditionDefinition, ProtocolState, ProtocolVerdict};
use crate::Probe;

pub(crate) const KNOWN_CONDITIONS: &[ProtocolConditionDefinition] = &[
    ProtocolConditionDefinition {
        id: "v4_ports_negotiation",
        chapter: "05",
        citation: "ProtocolInvocationLauncher.java:233-236, 976-990; AfirmaWebSocketServerManager.java:64-90",
        statement: "El cliente JavaScript selecciona 3 puertos aleatorios únicos (ports=p1,p2,p3) y el servidor enlaza el primer puerto libre de la lista.",
    },
    ProtocolConditionDefinition {
        id: "v4_echo_greeting",
        chapter: "05",
        citation: "AfirmaWebSocketServerV4.java:81-83",
        statement: "La petición de eco echo=-idsession=<idSession>@EOF recibe la respuesta de texto OK.",
    },
    ProtocolConditionDefinition {
        id: "v4_channel_credential_present",
        chapter: "05",
        citation: "AfirmaWebSocketServerV4.java:72-78",
        statement: "El mensaje que incluye el identificador de sesión coincidente con el negociado en el arranque es aceptado para su procesamiento.",
    },
    ProtocolConditionDefinition {
        id: "v4_channel_credential_absent",
        chapter: "05",
        citation: "AfirmaWebSocketServerV4.java:72-78; ProtocolInvocationLauncherErrorManager.java:77, 134",
        statement: "Si el mensaje no incluye el identificador de sesión o este difiere del negociado en el arranque, el servidor responde con SAF_46.",
    },
    ProtocolConditionDefinition {
        id: "v4_channel_credential_malformed",
        chapter: "05",
        citation: "ProtocolInvocationLauncher.java:992-1008; AfirmaWebSocketServerV4.java:72-78",
        statement: "Si idsession en la URL de arranque contiene caracteres no alfanuméricos, se anula y el servidor no rechaza los mensajes con SAF_46.",
    },
    ProtocolConditionDefinition {
        id: "v4_single_client",
        chapter: "05",
        citation: "AfirmaWebSocketServer.java:70, 73-79, 82-91",
        statement: "La primera conexión se fija en wsClient; el cierre de conexiones secundarias no finaliza la aplicación, que sólo termina cuando se cierra el cliente principal.",
    },
    ProtocolConditionDefinition {
        id: "v3_ports_default_fixed",
        chapter: "05",
        citation: "ProtocolInvocationLauncher.java:87, 233-236; AfirmaWebSocketServer.java:51-56",
        statement: "En versión 3, la ausencia de ports en la URL de arranque asigna por defecto el puerto fijo 63117 (DEFAULT_WEBSOCKET_PORT).",
    },
    ProtocolConditionDefinition {
        id: "v3_echo_greeting",
        chapter: "05",
        citation: "AfirmaWebSocketServer.java:33, 103-105",
        statement: "En versión 3, cualquier mensaje que empiece por echo= responde inmediatamente OK sin exigir @EOF ni identificador de sesión.",
    },
    ProtocolConditionDefinition {
        id: "v3_channel_credential_ignored",
        chapter: "05",
        citation: "AfirmaWebSocketServer.java:99-115",
        statement: "En versión 3, el servidor no valida el identificador de sesión en los mensajes entrantes y no emite SAF_46.",
    },
    ProtocolConditionDefinition {
        id: "v3_single_client",
        chapter: "05",
        citation: "AfirmaWebSocketServer.java:82-91",
        statement: "En versión 3, el cierre de una conexión secundaria no termina la aplicación; el canal principal sigue activo.",
    },
    ProtocolConditionDefinition {
        id: "operation_supported_protocol_version",
        chapter: "14",
        citation: "ProtocolVersion.java:60-62; ProtocolInvocationLauncher.java:62, 907-915",
        statement: "Las operaciones aceptan versiones de protocolo menores o iguales a 4 (MAX_PROTOCOL_VERSION_SUPPORTED).",
    },
    ProtocolConditionDefinition {
        id: "operation_unsupported_protocol_version_rejected",
        chapter: "14",
        citation: "ProtocolVersion.java:60-62; ProtocolInvocationLauncherSign.java:133-140",
        statement: "Una versión de protocolo solicitada superior a 4 (p. ej. ver=5) es rechazada con el código SAF_21.",
    },
    ProtocolConditionDefinition {
        id: "operation_minimum_client_version_satisfied",
        chapter: "14",
        citation: "Version.java:120-169; ProtocolInvocationLauncherSign.java:143-150",
        statement: "Si mcv solicita una versión igual o inferior a la versión del aplicativo (p. ej. mcv=1.0.0), la operación continúa sin error de versión.",
    },
    ProtocolConditionDefinition {
        id: "operation_minimum_client_version_unsatisfied_rejected",
        chapter: "14",
        citation: "Version.java:120-169; ProtocolInvocationLauncherSign.java:143-150; ProtocolInvocationLauncherErrorManager.java:72",
        statement: "Si mcv solicita una versión superior a la de la aplicación (p. ej. mcv=99.0.0), la operación es rechazada con SAF_41.",
    },
    ProtocolConditionDefinition {
        id: "websocket_channel_supported_versions_accepted",
        chapter: "14",
        citation: "AfirmaWebSocketServerManager.java:36, 100-107",
        statement: "El canal WebSocket admite las versiones de protocolo 3 y 4 (SUPPORTED_PROTOCOL_VERSIONS).",
    },
    ProtocolConditionDefinition {
        id: "websocket_channel_unsupported_versions_rejected",
        chapter: "14",
        citation: "AfirmaWebSocketServerManager.java:36, 100-107; ProtocolInvocationLauncher.java:240-245",
        statement: "Versiones de protocolo para WebSocket distintas de 3 y 4 (p. ej. v=1 o v=99) son rechazadas con SAF_21.",
    },
    ProtocolConditionDefinition {
        id: "service_channel_version_range",
        chapter: "14",
        citation: "ServiceInvocationManager.java:42-45, 212-220; ProtocolInvocationLauncher.java:281-288",
        statement: "El canal socket HTTP (afirma://service) sólo admite las versiones 1, 2 y 3; la versión 4 se rechaza con SAF_21.",
    },
    ProtocolConditionDefinition {
        id: "websocket_handshake_session_query_parameter",
        chapter: "14",
        citation: "AfirmaWebSocketServerV4.java:49-74; 14-versiones.md:377-380",
        statement: "El manual afirma que en versión 4 se exige idsession en la query string del handshake HTTP de WebSocket (ws.getResourceDescriptor()) y se rechaza con 1008 si no coincide.",
    },
    ProtocolConditionDefinition {
        id: "unsupported_protocol_uri_rejected",
        chapter: "15",
        citation: "ProtocolInvocationLauncher.java:172-178",
        statement: "Una URI que no empieza estrictamente por afirma:// se rechaza con SAF_02.",
    },
    ProtocolConditionDefinition {
        id: "unsupported_operation_rejected",
        chapter: "15",
        citation: "ProtocolInvocationLauncher.java:837-843",
        statement: "Una operación no soportada (host desconocido como unknownop) se rechaza con SAF_04.",
    },
    ProtocolConditionDefinition {
        id: "sign_missing_or_invalid_operation_rejected",
        chapter: "15",
        citation: "ProtocolInvocationLauncherSign.java:733",
        statement: "En afirma://sign?, si el parámetro op falta o no es reconocido, se rechaza con SAF_04.",
    },
    ProtocolConditionDefinition {
        id: "sign_unsupported_format_rejected",
        chapter: "15",
        citation: "ProtocolInvocationLauncherSign.java:269",
        statement: "Si el formato de firma especificado en format no está soportado, se rechaza con SAF_06.",
    },
    ProtocolConditionDefinition {
        id: "local_access_blocked",
        chapter: "15",
        citation: "UrlParameters.java:279-281; ProtocolInvocationLauncher.java:735",
        statement: "Si stservlet o rtservlet apuntan a direcciones locales prohibidas, se rechaza con SAF_13.",
    },
    ProtocolConditionDefinition {
        id: "invalid_parameters_syntax_rejected",
        chapter: "15",
        citation: "ProtocolInvocationLauncher.java:741",
        statement: "Parámetros de entrada con sintaxis inválida se rechazan con SAF_03.",
    },
];

const THE_V4_PROTOCOL: &str = "v4";
const THE_V3_PROTOCOL: &str = "v3";
const THE_SINGLE_SELECTION: &str = "selectcert";

impl Probe {
    pub(crate) fn run_protocol_lane(&self, dossier: &mut Dossier) {
        self.monitor
            .display_header(dossier.subject(), dossier.header());
        let total = KNOWN_CONDITIONS.len();
        let mut index = 0;
        self.run_protocol_errand(
            dossier,
            "protocol-v4",
            "protocol-v4",
            THE_V4_PROTOCOL,
            &mut index,
            total,
        );
        self.run_protocol_errand(
            dossier,
            "protocol-v4-malformed-id",
            "protocol-v4-malformed-id",
            THE_V4_PROTOCOL,
            &mut index,
            total,
        );
        self.run_protocol_errand(
            dossier,
            "protocol-v3",
            "protocol-v3",
            THE_V3_PROTOCOL,
            &mut index,
            total,
        );
        self.run_channel_rejection_errands(dossier, &mut index, total);
    }

    fn run_protocol_errand(
        &self,
        dossier: &mut Dossier,
        transcript_name: &str,
        script: &str,
        mode: &str,
        index: &mut usize,
        total: usize,
    ) {
        self.monitor
            .start_progress("Condición", *index + 1, total, transcript_name);
        let start = std::time::Instant::now();
        let outcome = self.run_errand(transcript_name, script, mode);
        let duration = start.elapsed();
        if !outcome.launched {
            return;
        }
        for condition in outcome.protocol_conditions {
            *index += 1;
            self.resolve_condition(
                dossier,
                &condition.id,
                condition.verdict,
                condition.observation,
                duration,
            );
        }
    }

    fn run_channel_rejection_errands(
        &self,
        dossier: &mut Dossier,
        index: &mut usize,
        total: usize,
    ) {
        *index += 1;
        self.monitor.start_progress(
            "Condición",
            *index,
            total,
            "websocket_channel_unsupported_versions_rejected",
        );
        let start = std::time::Instant::now();
        let outcome_v1 = self.run_errand("protocol-v1-rejection", THE_SINGLE_SELECTION, "v1");
        let duration = start.elapsed();
        if outcome_v1.launched {
            let (verdict, obs) = match outcome_v1.error_code.as_deref() {
                Some("SAF_21") => (ProtocolVerdict::Compliant, Some("SAF_21".to_owned())),
                Some(other) => (ProtocolVerdict::Discrepant, Some(other.to_owned())),
                None => (ProtocolVerdict::NotObservable, outcome_v1.error_type),
            };
            self.resolve_condition(
                dossier,
                "websocket_channel_unsupported_versions_rejected",
                verdict,
                obs,
                duration,
            );
        }

        *index += 1;
        self.monitor
            .start_progress("Condición", *index, total, "service_channel_version_range");
        let start = std::time::Instant::now();
        let outcome_service_v4 = self.run_errand(
            "protocol-service-v4-rejection",
            THE_SINGLE_SELECTION,
            "service-v4",
        );
        let duration = start.elapsed();
        if outcome_service_v4.launched {
            let (verdict, obs) = match outcome_service_v4.error_code.as_deref() {
                Some("SAF_21") => (ProtocolVerdict::Compliant, Some("SAF_21".to_owned())),
                Some(other) => (ProtocolVerdict::Discrepant, Some(other.to_owned())),
                None => (
                    ProtocolVerdict::NotObservable,
                    outcome_service_v4.error_type,
                ),
            };
            self.resolve_condition(
                dossier,
                "service_channel_version_range",
                verdict,
                obs,
                duration,
            );
        }

        *index += 1;
        self.monitor.start_progress(
            "Condición",
            *index,
            total,
            "unsupported_protocol_uri_rejected",
        );
        let start = std::time::Instant::now();
        let outcome_bad_uri = self.run_errand(
            "protocol-bad-uri-rejection",
            THE_SINGLE_SELECTION,
            "bad-uri",
        );
        let duration = start.elapsed();
        if outcome_bad_uri.launched {
            let (verdict, obs) = match outcome_bad_uri.error_code.as_deref() {
                Some("SAF_02") => (ProtocolVerdict::Compliant, Some("SAF_02".to_owned())),
                Some(other) => (ProtocolVerdict::Discrepant, Some(other.to_owned())),
                None => (ProtocolVerdict::NotObservable, outcome_bad_uri.error_type),
            };
            self.resolve_condition(
                dossier,
                "unsupported_protocol_uri_rejected",
                verdict,
                obs,
                duration,
            );
        }

        if dossier.protocol_state_of("v4_ports_negotiation")
            == Some(ProtocolState::Resolved(ProtocolVerdict::Compliant))
            && dossier.protocol_state_of("v3_ports_default_fixed")
                == Some(ProtocolState::Resolved(ProtocolVerdict::Compliant))
        {
            *index += 1;
            self.resolve_condition(
                dossier,
                "websocket_channel_supported_versions_accepted",
                ProtocolVerdict::Compliant,
                Some("v3 y v4 conectan con éxito".to_owned()),
                std::time::Duration::from_millis(0),
            );
        }
    }

    fn resolve_condition(
        &self,
        dossier: &mut Dossier,
        id: &str,
        verdict: ProtocolVerdict,
        observation: Option<String>,
        duration: std::time::Duration,
    ) {
        let (badge, color) = match verdict {
            ProtocolVerdict::Compliant => ("[CONFORME]", crate::verdicts::GREEN),
            ProtocolVerdict::Discrepant => ("[DISCREPANCIA]", crate::verdicts::RED),
            ProtocolVerdict::NotObservable => ("[NO OBSERVABLE]", crate::verdicts::YELLOW),
        };
        let chapter = dossier
            .protocol_conditions()
            .find(|(k, _)| *k == id)
            .map(|(_, record)| {
                if record.chapter.starts_with('[') {
                    record.chapter.clone()
                } else if record.chapter.starts_with("Cap.") {
                    format!("[{}]", record.chapter)
                } else {
                    format!("[Cap. {}]", record.chapter)
                }
            })
            .unwrap_or_else(|| "[--]".to_owned());
        let display_name = format!("{chapter} {id}");
        self.monitor.finish_item(
            badge,
            color,
            &display_name,
            duration,
            observation.as_deref(),
        );
        let _ = dossier.resolve_protocol(id, verdict, observation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_conditions_have_valid_chapters_and_citations() {
        for condition in KNOWN_CONDITIONS {
            assert!(
                condition.chapter == "05" || condition.chapter == "14" || condition.chapter == "15",
                "capítulo inválido para {}",
                condition.id
            );
            assert!(
                condition.citation.contains(".java"),
                "la cita debe incluir código Java original para {}",
                condition.id
            );
            assert!(
                !condition.statement.is_empty(),
                "la afirmación no puede estar vacía para {}",
                condition.id
            );
        }
    }

    #[test]
    fn conditions_are_indexed_by_unique_ids() {
        let mut ids = std::collections::HashSet::new();
        for condition in KNOWN_CONDITIONS {
            assert!(ids.insert(condition.id), "ID duplicado: {}", condition.id);
        }
    }
}
