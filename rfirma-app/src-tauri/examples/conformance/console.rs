//! La sesión de la consola web: el sujeto y el informe elegidos, la cola de comprobaciones y el
//! único hilo que las corre; no sabe de HTTP.

use std::collections::{BTreeMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::spawn;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::baseline::Profile;
use crate::catalogue::Check;
use crate::checks::{
    the_greetings_already_failed, the_group_of, the_reason_behind_a_failed_greeting, Settlement,
};
use crate::comparison::{compare, Comparison};
use crate::dossier::{CheckState, Dossier, HeaderCoordinates, THE_DOSSIER_FILE};
use crate::livelog::{CheckLog, LiveLogSink, Provenance};
use crate::snapshot::{snapshot_of, Activity, ReportEntry};
use crate::subject::{resolve, the_deduced_coordinates, DeducedCoordinates, Subject};
use crate::transcript::{log_path_of, transcript_path_of};
use crate::Probe;

/// Lo que se pide correr desde la página.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Request {
    Check(String),
    Suite(String),
    Pending,
}

/// Las coordenadas con las que se crea un informe, tal y como llegan de la página.
#[derive(Debug, Deserialize)]
pub(crate) struct NewReport {
    name: String,
    subject_version: String,
    os: String,
    os_version: String,
    transport: String,
    store: String,
}

#[derive(Clone)]
pub(crate) struct Console {
    shared: Arc<Shared>,
}

/// Lo que el conductor y las comprobaciones ven de la sesión mientras corren.
#[derive(Clone)]
pub(crate) struct Witness {
    shared: Arc<Shared>,
}

struct Shared {
    catalogue: Vec<Check>,
    reports_dir: PathBuf,
    patience: Duration,
    session: Mutex<Session>,
    wake: Condvar,
    listeners: Mutex<Vec<Sender<String>>>,
}

#[derive(Default)]
struct Session {
    subject: Option<Subject>,
    subject_complaints: Vec<String>,
    resolving_subject: bool,
    report: Option<OpenReport>,
    queue: VecDeque<Queued>,
    running: Vec<String>,
    started: Option<Instant>,
    log: Option<CheckLog>,
    question: Option<Question>,
    driver: Option<u32>,
    aborting: bool,
    reasons: BTreeMap<String, String>,
}

struct OpenReport {
    name: String,
    dir: PathBuf,
    dossier: Dossier,
}

struct Queued {
    id: String,
    in_batch: bool,
}

struct Question {
    check: String,
    prompt: String,
    reply: Sender<Option<String>>,
}

impl Session {
    fn busy(&self) -> bool {
        !self.running.is_empty() || !self.queue.is_empty() || self.resolving_subject
    }

    fn is_queued_or_running(&self, id: &str) -> bool {
        self.running.iter().any(|running| running == id)
            || self.queue.iter().any(|queued| queued.id == id)
    }
}

impl Console {
    pub(crate) fn new(catalogue: Vec<Check>, reports_dir: PathBuf, patience: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                catalogue,
                reports_dir,
                patience,
                session: Mutex::new(Session::default()),
                wake: Condvar::new(),
                listeners: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Arranca el hilo que corre, de una en una, lo que se vaya encolando.
    pub(crate) fn start_the_runner(&self) {
        let shared = Arc::clone(&self.shared);
        spawn(move || loop {
            run_the_next_group(&shared);
        });
    }

    /// Un canal que recibe ya enmarcado cada evento de la sesión, empezando por el estado actual.
    pub(crate) fn subscribe(&self) -> Receiver<String> {
        let (sender, receiver) = channel();
        let session = self.shared.lock();
        let _ = sender.send(event_frame("state", &self.shared.state_of(&session)));
        self.shared.listeners.lock().unwrap().push(sender);
        receiver
    }

    pub(crate) fn choose_subject(
        &self,
        profile: Profile,
        binary: Option<PathBuf>,
        trust_root: Option<PathBuf>,
    ) -> Result<(), String> {
        {
            let mut session = self.shared.lock();
            if session.busy() {
                return Err("no se cambia de sujeto con una tanda en marcha".to_owned());
            }
            session.resolving_subject = true;
            self.shared.publish(&session);
        }
        let resolved = resolve(profile, binary, trust_root);
        let mut session = self.shared.lock();
        session.resolving_subject = false;
        match resolved {
            Ok(subject) => {
                let stays = session.report.as_ref().is_some_and(|report| {
                    report.dossier.profile() == subject.profile
                        && report.dossier.subject() == subject.binary.display().to_string()
                });
                if !stays {
                    session.report = None;
                    session.reasons.clear();
                }
                session.subject = Some(subject);
                session.subject_complaints.clear();
            }
            Err(complaints) => {
                session.subject = None;
                session.report = None;
                session.subject_complaints = complaints;
            }
        }
        self.shared.publish(&session);
        Ok(())
    }

    pub(crate) fn the_deduced_coordinates(&self) -> Result<DeducedCoordinates, String> {
        let session = self.shared.lock();
        let subject = session.subject.as_ref().ok_or(NO_SUBJECT)?;
        Ok(the_deduced_coordinates(subject))
    }

    pub(crate) fn create_report(&self, new: NewReport) -> Result<(), String> {
        if self.shared.lock().subject.is_none() {
            return Err(NO_SUBJECT.to_owned());
        }
        let dir = self.shared.the_report_dir(&new.name)?;
        if dir.exists() {
            return Err(format!("ya hay un informe llamado «{}»", new.name));
        }
        if new.subject_version.trim().is_empty() {
            return Err(
                "falta la versión del sujeto: es la única que nadie puede deducir".to_owned(),
            );
        }
        let coordinates = HeaderCoordinates {
            os: new.os,
            os_version: new.os_version,
            subject_version: new.subject_version,
            transport: new.transport,
            store: new.store,
        };
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("{} no se pudo crear: {error}", dir.display()))?;
        self.open_report_at(new.name, dir.clone(), Some(coordinates))
            .inspect_err(|_| {
                let _ = std::fs::remove_dir_all(&dir);
            })
    }

    pub(crate) fn open_report(&self, name: String) -> Result<(), String> {
        let dir = self.shared.the_report_dir(&name)?;
        if !dir.join(THE_DOSSIER_FILE).is_file() {
            return Err(format!("no hay informe llamado «{name}»"));
        }
        self.open_report_at(name, dir, None)
    }

    fn open_report_at(
        &self,
        name: String,
        dir: PathBuf,
        coordinates: Option<HeaderCoordinates>,
    ) -> Result<(), String> {
        let mut session = self.shared.lock();
        if session.busy() {
            return Err("no se cambia de informe con una tanda en marcha".to_owned());
        }
        let subject = session.subject.as_ref().ok_or(NO_SUBJECT)?;
        let dossier = Dossier::open(
            &dir.join(THE_DOSSIER_FILE),
            &subject.binary.display().to_string(),
            subject.profile,
            &self.shared.catalogue,
            coordinates,
        )?;
        session.report = Some(OpenReport { name, dir, dossier });
        session.reasons.clear();
        self.shared.publish(&session);
        Ok(())
    }

    pub(crate) fn enqueue(&self, request: Request) -> Result<(), String> {
        let mut session = self.shared.lock();
        let report = session
            .report
            .as_ref()
            .ok_or("elige o crea un informe antes de correr nada")?;
        let (wanted, in_batch): (Vec<&Check>, bool) = match &request {
            Request::Check(id) => (
                vec![self
                    .shared
                    .catalogue
                    .iter()
                    .find(|check| &check.id == id)
                    .ok_or_else(|| format!("no conozco la comprobación «{id}»"))?],
                false,
            ),
            Request::Suite(suite) => (
                self.shared
                    .catalogue
                    .iter()
                    .filter(|check| &check.suite == suite)
                    .collect(),
                true,
            ),
            Request::Pending => (
                self.shared
                    .catalogue
                    .iter()
                    .filter(|check| report.dossier.state_of(&check.id) == Some(CheckState::Pending))
                    .collect(),
                true,
            ),
        };
        let ids: Vec<String> = wanted.iter().map(|check| check.id.clone()).collect();
        for id in ids {
            if !session.is_queued_or_running(&id) {
                session.queue.push_back(Queued { id, in_batch });
            }
        }
        self.shared.wake.notify_one();
        self.shared.publish(&session);
        Ok(())
    }

    /// Vacía la cola; con `abort`, además corta la comprobación en curso y la deja pendiente.
    pub(crate) fn stop(&self, abort: bool) {
        let mut session = self.shared.lock();
        session.queue.clear();
        if abort && !session.running.is_empty() {
            session.aborting = true;
            if let Some(pid) = session.driver {
                kill(pid);
            }
            if let Some(question) = session.question.take() {
                let _ = question.reply.send(None);
            }
        }
        self.shared.publish(&session);
    }

    /// La respuesta de la persona a la pregunta en curso; `None` la descarta.
    pub(crate) fn answer(&self, answer: Option<String>) -> Result<(), String> {
        let mut session = self.shared.lock();
        let question = session
            .question
            .take()
            .ok_or("no hay ninguna pregunta en curso")?;
        let _ = question.reply.send(answer);
        self.shared.publish(&session);
        Ok(())
    }

    pub(crate) fn log_of(&self, check: &str) -> Result<String, String> {
        self.read_in_the_report(|dir| log_path_of(dir, check))
    }

    pub(crate) fn transcript_of(&self, check: &str) -> Result<String, String> {
        self.read_in_the_report(|dir| transcript_path_of(dir, check))
    }

    fn read_in_the_report(&self, path_in: impl Fn(&Path) -> PathBuf) -> Result<String, String> {
        let path = {
            let session = self.shared.lock();
            let report = session.report.as_ref().ok_or("no hay informe abierto")?;
            path_in(&report.dir)
        };
        if !path.is_file() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))
    }

    pub(crate) fn compare(&self, a: &str, b: &str) -> Result<Comparison, String> {
        let read =
            |name: &str| Dossier::read(&self.shared.the_report_dir(name)?.join(THE_DOSSIER_FILE));
        Ok(compare(&read(a)?, &read(b)?))
    }
}

const NO_SUBJECT: &str = "elige un sujeto primero";

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Session> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn the_report_dir(&self, name: &str) -> Result<PathBuf, String> {
        if is_a_report_name(name) {
            Ok(self.reports_dir.join(name))
        } else {
            Err(format!(
                "«{name}» no vale como nombre de informe: letras, cifras, «.», «-» y «_», sin \
                 empezar por punto"
            ))
        }
    }

    fn state_of(&self, session: &Session) -> serde_json::Value {
        let reports = reports_in(&self.reports_dir);
        let activity = Activity {
            running: &session.running,
            running_for: session
                .started
                .map(|started| started.elapsed())
                .unwrap_or_default(),
            queued: session
                .queue
                .iter()
                .map(|queued| queued.id.as_str())
                .collect(),
            question: session
                .question
                .as_ref()
                .map(|question| (question.check.as_str(), question.prompt.as_str())),
            reasons: Some(&session.reasons),
            resolving_subject: session.resolving_subject,
        };
        let snapshot = snapshot_of(
            &self.catalogue,
            session.subject.as_ref(),
            &session.subject_complaints,
            session
                .report
                .as_ref()
                .map(|report| (report.name.as_str(), &report.dossier)),
            &reports,
            &activity,
        );
        serde_json::to_value(snapshot).expect("el estado se serializa")
    }

    fn publish(&self, session: &Session) {
        self.broadcast(&event_frame("state", &self.state_of(session)));
    }

    fn broadcast(&self, frame: &str) {
        self.listeners
            .lock()
            .unwrap()
            .retain(|listener| listener.send(frame.to_owned()).is_ok());
    }
}

impl Witness {
    pub(crate) fn started_at(&self) -> Instant {
        self.shared.lock().started.unwrap_or_else(Instant::now)
    }

    /// Por donde llegan las líneas de la comprobación en curso: a su fichero y a la página.
    pub(crate) fn log_sink(&self) -> LiveLogSink {
        let (log, check) = {
            let session = self.shared.lock();
            (session.log.clone(), session.running.first().cloned())
        };
        let shared = Arc::clone(&self.shared);
        LiveLogSink::new(move |line| {
            if let Some(log) = &log {
                log.write(&line);
            }
            let payload = serde_json::json!({ "check": check, "line": line });
            shared.broadcast(&event_frame("log", &payload));
        })
    }

    /// Un diagnóstico del arnés, con la misma procedencia en el fichero y en la página.
    pub(crate) fn harness(&self, text: &str) {
        self.log_sink()
            .push(Provenance::Harness, self.started_at().elapsed(), text);
    }

    /// Pregunta a la persona y espera; `None` si la descarta o se aborta la tanda.
    pub(crate) fn ask(&self, check: &str, prompt: &str) -> Option<String> {
        let (reply, answer) = channel();
        {
            let mut session = self.shared.lock();
            if session.aborting {
                return None;
            }
            session.question = Some(Question {
                check: check.to_owned(),
                prompt: prompt.to_owned(),
                reply,
            });
            self.shared.publish(&session);
        }
        self.harness(&format!("pregunta: {prompt}"));
        let answer = answer.recv().ok().flatten();
        self.harness(&format!(
            "respuesta: {}",
            answer.as_deref().unwrap_or("(descartada)")
        ));
        answer
    }

    pub(crate) fn driver_spawned(&self, pid: u32) {
        let mut session = self.shared.lock();
        if session.aborting {
            kill(pid);
        }
        session.driver = Some(pid);
    }

    pub(crate) fn driver_finished(&self) {
        self.shared.lock().driver = None;
    }

    pub(crate) fn aborted(&self) -> bool {
        self.shared.lock().aborting
    }
}

fn run_the_next_group(shared: &Arc<Shared>) {
    let (group, probe, store) = {
        let mut session = shared.lock();
        loop {
            match take_the_next_group(shared, &mut session) {
                Some(next) => break next,
                None => {
                    session = shared
                        .wake
                        .wait(session)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            }
        }
    };
    let checks: Vec<&Check> = group
        .iter()
        .filter_map(|id| shared.catalogue.iter().find(|check| &check.id == id))
        .collect();
    let settled = catch_unwind(AssertUnwindSafe(|| probe.run_group(&checks, &store)))
        .unwrap_or_else(|panic| {
            let why = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
                .unwrap_or_default();
            probe.witness.harness(&format!("el arnés reventó: {why}"));
            checks
                .iter()
                .map(|check| Settlement::Pending {
                    id: check.id.clone(),
                    why: format!("el arnés reventó: {why}"),
                })
                .collect()
        });
    let mut session = shared.lock();
    settle(&mut session, &group, settled, &probe);
    session.running.clear();
    session.started = None;
    session.log = None;
    session.driver = None;
    session.aborting = false;
    shared.publish(&session);
}

/// Saca de la cola lo siguiente que hay que correr y lo marca en curso; lo que su saludo fallido
/// detiene lo deja pendiente sin correrlo.
fn take_the_next_group(
    shared: &Arc<Shared>,
    session: &mut Session,
) -> Option<(Vec<String>, Probe, String)> {
    while let Some(queued) = session.queue.pop_front() {
        let (Some(subject), Some(report)) = (&session.subject, &session.report) else {
            session.queue.clear();
            return None;
        };
        let Some(head) = shared.catalogue.iter().find(|check| check.id == queued.id) else {
            continue;
        };
        let stopping_greeting =
            the_greetings_already_failed(&shared.catalogue, |id| report.dossier.state_of(id))
                .get(head.suite.as_str())
                .filter(|greeting| **greeting != head.id)
                .map(|greeting| (*greeting).to_owned());
        let subject = subject.clone();
        let dir = report.dir.clone();
        let store = report.dossier.header().store.clone();
        if let Some(greeting) = stopping_greeting.filter(|_| queued.in_batch) {
            let why = the_reason_behind_a_failed_greeting(&greeting);
            session.reasons.insert(head.id.clone(), why);
            shared.publish(session);
            continue;
        }
        let queued_checks: Vec<&Check> = std::iter::once(head)
            .chain(
                session
                    .queue
                    .iter()
                    .filter_map(|other| shared.catalogue.iter().find(|check| check.id == other.id)),
            )
            .collect();
        let group: Vec<String> = the_group_of(head, &queued_checks)
            .into_iter()
            .map(|check| check.id.clone())
            .collect();
        session.queue.retain(|other| !group.contains(&other.id));
        let probe = Probe {
            subject: subject.launcher,
            trust_root: subject.trust_root,
            profile: subject.profile,
            report: dir.clone(),
            patience: shared.patience,
            witness: Witness {
                shared: Arc::clone(shared),
            },
        };
        session.log = CheckLog::open(&log_path_of(&dir, &group[0])).ok();
        session.running = group.clone();
        session.started = Some(Instant::now());
        shared.publish(session);
        return Some((group, probe, store));
    }
    None
}

/// Apunta en el informe lo que dejó el grupo, y copia a cada miembro el registro y las tramas del
/// trámite que compartieron.
fn settle(session: &mut Session, group: &[String], settled: Vec<Settlement>, probe: &Probe) {
    let Some(report) = session.report.as_mut() else {
        return;
    };
    for settlement in settled {
        match settlement {
            Settlement::Resolved {
                id,
                verdict,
                observation,
                duration,
            } => {
                if let Err(complaint) = report.dossier.resolve(&id, verdict, observation, duration)
                {
                    probe.witness.harness(&complaint);
                }
                session.reasons.remove(&id);
            }
            Settlement::Pending { id, why } => {
                session.reasons.insert(id, why);
            }
        }
    }
    let head = &group[0];
    for member in &group[1..] {
        let _ = std::fs::copy(
            log_path_of(&report.dir, head),
            log_path_of(&report.dir, member),
        );
        let _ = std::fs::copy(
            transcript_path_of(&report.dir, head),
            transcript_path_of(&report.dir, member),
        );
    }
}

/// Los informes del directorio, cada uno con su sujeto y su perfil, o la queja si no se lee.
fn reports_in(dir: &Path) -> Vec<ReportEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut reports: Vec<ReportEntry> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(THE_DOSSIER_FILE).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_a_report_name(name))
        .map(
            |name| match Dossier::read(&dir.join(&name).join(THE_DOSSIER_FILE)) {
                Ok(dossier) => ReportEntry {
                    profile: Some(dossier.profile().name()),
                    subject: Some(dossier.subject().to_owned()),
                    subject_version: Some(dossier.header().subject_version.clone()),
                    date: Some(dossier.header().date.clone()),
                    complaint: None,
                    name,
                },
                Err(complaint) => ReportEntry {
                    profile: None,
                    subject: None,
                    subject_version: None,
                    date: None,
                    complaint: Some(complaint),
                    name,
                },
            },
        )
        .collect();
    reports.sort_by(|a, b| a.name.cmp(&b.name));
    reports
}

/// Un nombre que es un solo directorio bajo el de informes, y nunca una ruta que salga de él.
fn is_a_report_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn event_frame(kind: &str, payload: &serde_json::Value) -> String {
    format!("event: {kind}\ndata: {payload}\n\n")
}

fn kill(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_name_is_one_directory_and_never_a_way_out() {
        assert!(is_a_report_name("autofirma-1.9.2-ubuntu"));
        assert!(is_a_report_name("rfirma_0.10.0"));
        assert!(!is_a_report_name(""));
        assert!(!is_a_report_name(".."));
        assert!(!is_a_report_name(".oculto"));
        assert!(!is_a_report_name("a/b"));
        assert!(!is_a_report_name("con espacio"));
    }

    #[test]
    fn the_reports_directory_lists_each_report_by_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("b-tanda")).unwrap();
        std::fs::write(dir.path().join("b-tanda").join(THE_DOSSIER_FILE), "{").unwrap();
        std::fs::create_dir(dir.path().join("sin-expediente")).unwrap();

        let reports = reports_in(dir.path());

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].name, "b-tanda");
        assert!(reports[0]
            .complaint
            .as_deref()
            .unwrap()
            .contains("no es un expediente"));
    }

    #[test]
    fn an_event_is_framed_for_server_sent_events() {
        assert_eq!(
            event_frame("log", &serde_json::json!({"line": "x"})),
            "event: log\ndata: {\"line\":\"x\"}\n\n"
        );
    }
}
