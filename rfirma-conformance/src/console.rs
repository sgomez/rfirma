//! La sesión de la consola web: el cliente y el informe elegidos, la cola de comprobaciones y el
//! único hilo que las corre; no sabe de HTTP.

use std::collections::{BTreeMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::spawn;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::catalogue::Check;
use crate::checks::{
    the_greetings_already_failed, the_group_of, the_reason_behind_a_failed_greeting, Settlement,
};
use crate::client::ClientKind;
use crate::client::{resolve, the_deduced_coordinates, Client, DeducedCoordinates};
use crate::comparison::{compare, Comparison};
use crate::livelog::{CheckLog, LiveLogSink, Provenance};
use crate::outcome::CheckState;
use crate::report::{HeaderCoordinates, Report, THE_REPORT_FILE};
use crate::report_view::report_view;
use crate::snapshot::{snapshot_of, Activity, ReportEntry};
use crate::transcript::{log_path_of, transcript_path_of};
use crate::validation::{
    read_the_reference, the_reference_dir, the_references_in, validate, Validation,
};
use crate::Probe;

/// Lo que se pide correr: una comprobación, un conjunto entero o sus pendientes, o todo lo pendiente.
#[derive(Debug, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub(crate) enum Request {
    Check {
        check: String,
    },
    Set {
        set: String,
        #[ts(optional)]
        pending: Option<bool>,
    },
    Pending(AllPending),
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum AllPending {
    Pending,
}

/// Una línea del registro en vivo y la comprobación que la dio.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct LiveLine {
    check: Option<String>,
    line: String,
}

/// Las coordenadas con las que se crea un informe, tal y como llegan de la página.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub(crate) struct NewReport {
    name: String,
    client_version: String,
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
    client: Option<Client>,
    client_complaints: Vec<String>,
    resolving_client: bool,
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
    report: Report,
}

struct Queued {
    id: String,
    in_batch: bool,
}

struct Question {
    check: String,
    prompt: String,
    kind: &'static str,
    reply: Sender<Option<String>>,
}

impl Session {
    fn busy(&self) -> bool {
        !self.running.is_empty() || !self.queue.is_empty() || self.resolving_client
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

    pub(crate) fn choose_client(
        &self,
        kind: ClientKind,
        binary: Option<PathBuf>,
        trust_root: Option<PathBuf>,
    ) -> Result<(), String> {
        {
            let mut session = self.shared.lock();
            if session.busy() {
                return Err("no se cambia de cliente con un informe corriendo".to_owned());
            }
            session.resolving_client = true;
            self.shared.publish(&session);
        }
        let resolved = resolve(kind, binary, trust_root);
        let mut session = self.shared.lock();
        session.resolving_client = false;
        match resolved {
            Ok(client) => {
                session.client = Some(client);
                session.client_complaints.clear();
            }
            Err(complaints) => {
                session.client = None;
                session.client_complaints = complaints;
            }
        }
        self.shared.publish(&session);
        Ok(())
    }

    pub(crate) fn the_deduced_coordinates(&self) -> Result<DeducedCoordinates, String> {
        let session = self.shared.lock();
        let client = session.client.as_ref().ok_or(NO_CLIENT)?;
        Ok(the_deduced_coordinates(client))
    }

    pub(crate) fn create_report(&self, new: NewReport) -> Result<(), String> {
        let dir = self.shared.the_report_dir(&new.name)?;
        if dir.exists() {
            return Err(format!("ya hay un informe llamado «{}»", new.name));
        }
        if new.client_version.trim().is_empty() {
            return Err(
                "falta la versión del cliente: es la única que nadie puede deducir".to_owned(),
            );
        }
        let coordinates = HeaderCoordinates {
            os: new.os,
            os_version: new.os_version,
            client_version: new.client_version,
            transport: new.transport,
            store: new.store,
        };
        let mut session = self.shared.lock();
        if session.busy() {
            return Err(NO_SWITCHING_REPORTS.to_owned());
        }
        let client = session.client.as_ref().ok_or(NO_CLIENT)?;
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("{} no se pudo crear: {error}", dir.display()))?;
        let report = Report::create(
            &dir.join(THE_REPORT_FILE),
            &client.binary.display().to_string(),
            client.kind,
            &self.shared.catalogue,
            coordinates,
        )
        .inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&dir);
        })?;
        self.shared.install(&mut session, new.name, dir, report);
        Ok(())
    }

    pub(crate) fn open_report(&self, name: String) -> Result<(), String> {
        let dir = self.shared.the_report_dir(&name)?;
        if !dir.join(THE_REPORT_FILE).is_file() {
            return Err(format!("no hay informe llamado «{name}»"));
        }
        let mut session = self.shared.lock();
        if session.busy() {
            return Err(NO_SWITCHING_REPORTS.to_owned());
        }
        let report = Report::open(&dir.join(THE_REPORT_FILE), &self.shared.catalogue)?;
        self.shared.install(&mut session, name, dir, report);
        Ok(())
    }

    pub(crate) fn enqueue(&self, request: Request) -> Result<(), String> {
        let mut session = self.shared.lock();
        let open = session
            .report
            .as_ref()
            .ok_or("elige o crea un informe antes de correr nada")?;
        let client = session.client.as_ref().ok_or(NO_CLIENT)?;
        if let Some(complaint) = open
            .report
            .refuses_to_continue_with(&client.binary.display().to_string(), client.kind)
        {
            return Err(complaint);
        }
        let is_pending =
            |check: &&Check| open.report.state_of(&check.id) == Some(CheckState::Pending);
        let (wanted, in_batch): (Vec<&Check>, bool) = match &request {
            Request::Check { check: id } => (
                vec![self
                    .shared
                    .catalogue
                    .iter()
                    .find(|check| &check.id == id)
                    .ok_or_else(|| format!("no conozco la comprobación «{id}»"))?],
                false,
            ),
            Request::Set { set, pending } => (
                self.shared
                    .catalogue
                    .iter()
                    .filter(|check| &check.set == set)
                    .filter(|check| !pending.unwrap_or(false) || is_pending(check))
                    .collect(),
                true,
            ),
            Request::Pending(AllPending::Pending) => (
                self.shared.catalogue.iter().filter(is_pending).collect(),
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

    /// Vacía la cola y corta la comprobación en curso, que queda pendiente.
    pub(crate) fn stop(&self) {
        let mut session = self.shared.lock();
        session.queue.clear();
        cut_the_running_check(&mut session);
        self.shared.publish(&session);
    }

    /// Corta la comprobación en curso, la deja pendiente y sigue con la cola.
    pub(crate) fn skip(&self) {
        let mut session = self.shared.lock();
        cut_the_running_check(&mut session);
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

    /// El registro de `check` en el informe `report`, o en el abierto si no se nombra ninguno.
    pub(crate) fn log_of(&self, report: Option<&str>, check: &str) -> Result<String, String> {
        self.read_in_the_report(report, |dir| log_path_of(dir, check))
    }

    /// Las tramas de `check` en el informe `report`, o en el abierto si no se nombra ninguno.
    pub(crate) fn transcript_of(
        &self,
        report: Option<&str>,
        check: &str,
    ) -> Result<String, String> {
        self.read_in_the_report(report, |dir| transcript_path_of(dir, check))
    }

    fn read_in_the_report(
        &self,
        report: Option<&str>,
        path_in: impl Fn(&Path) -> PathBuf,
    ) -> Result<String, String> {
        let dir = match report {
            Some(name) => self.shared.the_report_dir(name)?,
            None => {
                let session = self.shared.lock();
                let report = session.report.as_ref().ok_or("no hay informe abierto")?;
                report.dir.clone()
            }
        };
        let path = path_in(&dir);
        if !path.is_file() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))
    }

    /// La vista del informe `name`, sea o no el de la sesión, sin escribir nada.
    pub(crate) fn report_view(&self, name: &str) -> Result<serde_json::Value, String> {
        let report = self.open_to_see(name)?;
        serde_json::to_value(report_view(&report, &self.shared.catalogue))
            .map_err(|error| format!("el informe «{name}» no se pudo serializar: {error}"))
    }

    pub(crate) fn compare(&self, a: &str, b: &str) -> Result<Comparison, String> {
        Ok(compare(
            &self.open_to_see(a)?,
            &self.open_to_see(b)?,
            &self.shared.catalogue,
        ))
    }

    fn open_to_see(&self, name: &str) -> Result<Report, String> {
        let path = self.shared.the_report_dir(name)?.join(THE_REPORT_FILE);
        if !path.is_file() {
            return Err(format!("no hay informe llamado «{name}»"));
        }
        Report::open(&path, &self.shared.catalogue)
    }

    pub(crate) fn references(&self) -> Result<Vec<String>, String> {
        the_references_in(&the_reference_dir())
    }

    pub(crate) fn validate(&self, report: &str, reference: &str) -> Result<Validation, String> {
        let report = self.open_to_see(report)?;
        let reference = read_the_reference(&the_reference_dir(), reference)?;
        Ok(validate(&report, &self.shared.catalogue, &reference))
    }
}

const NO_CLIENT: &str = "elige un cliente primero";
const NO_SWITCHING_REPORTS: &str = "no se cambia de informe con otro corriendo";

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

    fn install(&self, session: &mut Session, name: String, dir: PathBuf, report: Report) {
        session.report = Some(OpenReport { name, dir, report });
        session.reasons.clear();
        self.publish(session);
    }

    fn state_of(&self, session: &Session) -> serde_json::Value {
        let reports = reports_in(&self.reports_dir);
        let activity = Activity {
            client: session.client.as_ref(),
            client_complaints: &session.client_complaints,
            resolving_client: session.resolving_client,
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
            question: session.question.as_ref().map(|question| {
                (
                    question.check.as_str(),
                    question.prompt.as_str(),
                    question.kind,
                )
            }),
            reasons: Some(&session.reasons),
        };
        let snapshot = snapshot_of(
            &self.catalogue,
            session
                .report
                .as_ref()
                .map(|open| (open.name.as_str(), &open.report)),
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
            let payload = serde_json::to_value(LiveLine {
                check: check.clone(),
                line,
            })
            .expect("la línea se serializa");
            shared.broadcast(&event_frame("log", &payload));
        })
    }

    /// Un diagnóstico del arnés, con la misma procedencia en el fichero y en la página.
    pub(crate) fn harness(&self, text: &str) {
        self.log_sink()
            .push(Provenance::Suite, self.started_at().elapsed(), text);
    }

    /// Pregunta a la persona y espera; `None` si la descarta o se aborta la pasada.
    pub(crate) fn ask(&self, check: &str, prompt: &str) -> Option<String> {
        self.put_to_the_person(check, prompt, "outcome")
    }

    /// Cuenta a la persona lo que va a pasar y espera a que dé paso; `false` si lo salta o se
    /// aborta la pasada.
    pub(crate) fn brief(&self, check: &str, briefing: &str) -> bool {
        self.put_to_the_person(check, briefing, "briefing")
            .is_some()
    }

    fn put_to_the_person(&self, check: &str, prompt: &str, kind: &'static str) -> Option<String> {
        let (reply, answer) = channel();
        {
            let mut session = self.shared.lock();
            if session.aborting {
                return None;
            }
            session.question = Some(Question {
                check: check.to_owned(),
                prompt: prompt.to_owned(),
                kind,
                reply,
            });
            self.shared.publish(&session);
        }
        let label = if kind == "briefing" {
            "aviso"
        } else {
            "pregunta"
        };
        self.harness(&format!("{label}: {prompt}"));
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
        let (Some(client), Some(open)) = (&session.client, &session.report) else {
            session.queue.clear();
            return None;
        };
        let Some(head) = shared.catalogue.iter().find(|check| check.id == queued.id) else {
            continue;
        };
        let stopping_greeting =
            the_greetings_already_failed(&shared.catalogue, |id| open.report.state_of(id))
                .get(head.set.as_str())
                .filter(|greeting| **greeting != head.id)
                .map(|greeting| (*greeting).to_owned());
        let client = client.clone();
        let dir = open.dir.clone();
        let store = open.report.header().store.clone();
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
            client: client.launcher,
            trust_root: client.trust_root,
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
    let Some(open) = session.report.as_mut() else {
        return;
    };
    for settlement in settled {
        match settlement {
            Settlement::Resolved {
                id,
                outcome,
                observation,
                duration,
            } => {
                if let Err(complaint) = open.report.resolve(&id, outcome, observation, duration) {
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
        let _ = std::fs::copy(log_path_of(&open.dir, head), log_path_of(&open.dir, member));
        let _ = std::fs::copy(
            transcript_path_of(&open.dir, head),
            transcript_path_of(&open.dir, member),
        );
    }
}

/// Los informes del directorio, cada uno con su cliente y su clase, o la queja si no se lee.
fn reports_in(dir: &Path) -> Vec<ReportEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut reports: Vec<ReportEntry> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(THE_REPORT_FILE).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_a_report_name(name))
        .map(
            |name| match Report::read(&dir.join(&name).join(THE_REPORT_FILE)) {
                Ok(report) => ReportEntry {
                    kind: Some(report.kind().name()),
                    client: Some(report.client().to_owned()),
                    client_version: Some(report.header().client_version.clone()),
                    date: Some(report.header().date.clone()),
                    complaint: None,
                    name,
                },
                Err(complaint) => ReportEntry {
                    kind: None,
                    client: None,
                    client_version: None,
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

fn cut_the_running_check(session: &mut Session) {
    if session.running.is_empty() {
        return;
    }
    session.aborting = true;
    if let Some(pid) = session.driver {
        kill(pid);
    }
    if let Some(question) = session.question.take() {
        let _ = question.reply.send(None);
    }
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
        std::fs::create_dir(dir.path().join("b-informe")).unwrap();
        std::fs::write(dir.path().join("b-informe").join(THE_REPORT_FILE), "{").unwrap();
        std::fs::create_dir(dir.path().join("sin-dossier")).unwrap();

        let reports = reports_in(dir.path());

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].name, "b-informe");
        assert!(reports[0]
            .complaint
            .as_deref()
            .unwrap()
            .contains("no es un informe"));
    }

    fn a_catalogue() -> Vec<Check> {
        crate::catalogue::the_catalogue_in(
            r#"
[[check]]
id = "a_greeting"
set = "saludo"
chapter = "14"
citation = "A.java:1"
statement = "Saluda."
drive = { mode = "v4", script = "protocol-v4" }
"#,
        )
        .unwrap()
    }

    fn a_report_in(dir: &Path, name: &str, kind: ClientKind, catalogue: &[Check]) {
        std::fs::create_dir(dir.join(name)).unwrap();
        Report::create(
            &dir.join(name).join(THE_REPORT_FILE),
            &format!("/usr/bin/{}", kind.name()),
            kind,
            catalogue,
            HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.0".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2".to_owned(),
            },
        )
        .unwrap();
    }

    fn the_shape_of(json: &serde_json::Value) -> serde_json::Value {
        match json {
            serde_json::Value::Object(fields) => fields
                .iter()
                .map(|(key, value)| (key.clone(), the_shape_of(value)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
            serde_json::Value::Array(items) => items.iter().map(the_shape_of).collect(),
            _ => serde_json::Value::Null,
        }
    }

    #[test]
    fn any_two_reports_of_different_clients_are_seen_alike_while_the_session_holds_another() {
        let dir = tempfile::tempdir().unwrap();
        let catalogue = a_catalogue();
        a_report_in(dir.path(), "a", ClientKind::Autofirma, &catalogue);
        a_report_in(dir.path(), "b", ClientKind::Rfirma, &catalogue);
        a_report_in(dir.path(), "c", ClientKind::Autofirma, &catalogue);
        let console = Console::new(catalogue, dir.path().to_owned(), Duration::ZERO);
        console.open_report("c".to_owned()).unwrap();

        let a = console.report_view("a").unwrap();
        let b = console.report_view("b").unwrap();

        assert_eq!(the_shape_of(&a), the_shape_of(&b));
        assert_eq!(
            (&a["kind"], &b["kind"]),
            (&"autofirma".into(), &"rfirma".into())
        );
        let state = console.subscribe().recv().unwrap();
        assert!(state.contains("\"report_name\":\"c\""));
    }

    #[test]
    fn seeing_a_report_that_does_not_exist_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let console = Console::new(a_catalogue(), dir.path().to_owned(), Duration::ZERO);

        assert_eq!(
            console.report_view("nadie").unwrap_err(),
            "no hay informe llamado «nadie»"
        );
    }

    #[test]
    fn an_event_is_framed_for_server_sent_events() {
        assert_eq!(
            event_frame("log", &serde_json::json!({"line": "x"})),
            "event: log\ndata: {\"line\":\"x\"}\n\n"
        );
    }
}
