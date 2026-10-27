//! Spike desechable de GlitchTip: el cliente de sentry, el extractor del pánico y el eco del evento de la ventana.

use std::sync::Arc;

pub(crate) fn spike_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = std::env::var("RFIRMA_GLITCHTIP_DSN").ok()?;
    let mut options = sentry::ClientOptions::new();
    options.release = sentry::release_name!();
    options.server_name = None;
    options.default_integrations = false;
    options.integrations = vec![
        Arc::new(sentry::integrations::contexts::ContextIntegration::default()),
        Arc::new(sentry::integrations::panic::PanicIntegration::new().add_extractor(panic_event)),
        Arc::new(sentry::integrations::backtrace::ProcessStacktraceIntegration),
    ];
    options.before_send = Some(Arc::new(|mut event| {
        event.server_name = None;
        for image in &mut event.debug_meta.to_mut().images {
            if let sentry::protocol::DebugImage::Symbolic(symbolic) = image {
                symbolic.name = std::path::Path::new(&symbolic.name)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
        }
        eprintln!(
            "SPIKE-RUST-EVENT {}",
            serde_json::to_string(&event).unwrap_or_default()
        );
        Some(event)
    }));
    Some(sentry::init((dsn, options)))
}

fn panic_event(info: &std::panic::PanicHookInfo<'_>) -> Option<sentry::protocol::Event<'static>> {
    let location = info
        .location()
        .map(|at| format!("{}:{}:{}", at.file(), at.line(), at.column()))
        .unwrap_or_default();
    Some(sentry::protocol::Event {
        exception: vec![sentry::protocol::Exception {
            ty: "panic".into(),
            value: Some(location),
            mechanism: Some(sentry::protocol::Mechanism {
                ty: "panic".into(),
                handled: Some(false),
                ..Default::default()
            }),
            stacktrace: sentry::integrations::backtrace::current_stacktrace().map(|mut trace| {
                if let Some(machinery) = trace.frames.iter().position(|frame| {
                    frame.function.as_deref().is_some_and(|name| {
                        name.starts_with("core::panicking::")
                            || name.starts_with("std::panicking::")
                            || name.ends_with("rust_begin_unwind")
                    })
                }) {
                    trace.frames.truncate(machinery);
                }
                trace
            }),
            ..Default::default()
        }]
        .into(),
        level: sentry::Level::Fatal,
        ..Default::default()
    })
}

#[tauri::command]
pub(crate) fn spike_log(event: String) {
    eprintln!("SPIKE-JS-EVENT {event}");
}
