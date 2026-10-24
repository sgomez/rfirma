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

use crate::catalogue::{Assistance, Check};
use crate::checks::{
    all_pending, the_greeting_that_stops, the_greetings_already_failed, the_group_of,
    the_reason_behind_a_failed_greeting, Settled, Settlement,
};
use crate::client::{
    prepare, resolve, the_binary_to_restore as the_binary_to_restore_of, the_deduced_coordinates,
    Candidate, Client, ClientKind, DeducedCoordinates, Profile, Store,
};
use crate::comparison::{compare, Comparison};
use crate::errand::{ErrandKey, ErrandRunner, NodeRunner, ObservedErrand};
use crate::hand::the_assistance_of;
use crate::livelog::{parse_log_line, CheckLog, LiveLogSink, LogLine, Provenance};
use crate::outcome::CheckState;
use crate::recipe::the_recipe_of;
use crate::report::{HeaderCoordinates, HeaderCorrection, Report, THE_REPORT_FILE};
use crate::report_view::report_view;
use crate::selection::{
    resolve as resolve_the_selection, the_sets_of, CatalogueSet, SelectionResolution,
};
use crate::snapshot::{snapshot_of, Activity, BatchView, PendingByTranches, ReportEntry};
use crate::transcript::{log_path_of, transcript_path_of};
use crate::validation::{
    read_the_reference, the_reference_dir, the_references_in, validate, Validation,
};
use crate::witness::Witness;
use crate::Probe;

/// Lo que se pide correr: una Selección, en blanco todo el alcance del informe, en un modo y, si se
/// nombran, solo de unos tramos.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub(crate) struct Request {
    selection: String,
    #[ts(optional)]
    mode: Option<RunMode>,
    #[ts(optional)]
    tranches: Option<Tranches>,
}

/// Cómo se corre una Selección: reutilizando los trámites guardados o relanzando el cliente.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum RunMode {
    /// Corre las PENDIENTE y vuelve a juzgar las resueltas con trámite, lanzándolo si se olvidó.
    #[default]
    Reuse,
    /// Abre el cliente para todo lo seleccionado y sustituye los trámites guardados.
    Relaunch,
}

/// Los tramos de una tanda: solo sin persona, el resto, o todos.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum Tranches {
    Unattended,
    Attended,
    All,
}

impl Tranches {
    fn take(self, assistance: Assistance) -> bool {
        match self {
            Self::Unattended => assistance == Assistance::None,
            Self::Attended => assistance != Assistance::None,
            Self::All => true,
        }
    }
}

/// Un fallo de la suite en una comprobación, que la página avisa aparte de su resultado.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct SuiteFailure {
    check: String,
    why: String,
}

/// Una línea del registro en vivo y la comprobación que la dio.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct LiveLine {
    check: String,
    provenance: Provenance,
    clock: String,
    text: String,
}

/// Las coordenadas con las que se crea un informe, tal y como llegan de la página.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub(crate) struct NewReport {
    name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    notes: String,
    client_version: String,
    os: String,
    os_version: String,
    scope: Option<String>,
}

#[derive(Clone)]
pub struct Console {
    shared: Arc<Shared>,
}

/// El testigo de verdad: cuenta en la página y en el registro lo que pasa.
struct ConsoleWitness {
    shared: Arc<Shared>,
}

struct Shared {
    catalogue: Vec<Check>,
    runner: Arc<dyn ErrandRunner>,
    reports_dir: PathBuf,
    patience: Duration,
    session: Mutex<Session>,
    wake: Condvar,
    listeners: Mutex<Vec<Sender<String>>>,
}

#[derive(Default)]
struct Session {
    client: Option<Client>,
    candidates: Vec<Candidate>,
    profiles: Vec<Profile>,
    client_complaints: Vec<String>,
    resolving_client: bool,
    reports: Vec<ReportEntry>,
    report: Option<OpenReport>,
    queue: VecDeque<Queued>,
    running: Vec<String>,
    started: Option<Instant>,
    batch: Option<Batch>,
    log: Option<CheckLog>,
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
    relaunch: bool,
}

/// La tanda desde que la cola estaba vacía: cuándo empezó y cuántas comprobaciones se le han pedido.
struct Batch {
    started: Instant,
    total: usize,
}

impl Session {
    fn busy(&self) -> bool {
        self.batch_is_running() || self.resolving_client
    }

    fn batch_is_running(&self) -> bool {
        !self.running.is_empty() || !self.queue.is_empty()
    }

    fn batch_view(&self) -> Option<BatchView> {
        let batch = self.batch.as_ref().filter(|_| self.batch_is_running())?;
        let left = self.queue.len() + self.running.len();
        Some(BatchView {
            total: batch.total,
            done: batch.total.saturating_sub(left),
            running: self.running.len(),
            elapsed_ms: batch.started.elapsed().as_millis(),
        })
    }

    fn is_queued_or_running(&self, id: &str) -> bool {
        self.running.iter().any(|running| running == id)
            || self.queue.iter().any(|queued| queued.id == id)
    }
}

impl Console {
    pub fn new(catalogue: Vec<Check>, reports_dir: PathBuf, patience: Duration) -> Self {
        Self::running_errands_with(catalogue, reports_dir, patience, Arc::new(NodeRunner))
    }

    fn running_errands_with(
        catalogue: Vec<Check>,
        reports_dir: PathBuf,
        patience: Duration,
        runner: Arc<dyn ErrandRunner>,
    ) -> Self {
        let session = Session {
            reports: reports_in(&reports_dir, None, &[]),
            ..Session::default()
        };
        Self {
            shared: Arc::new(Shared {
                catalogue,
                runner,
                reports_dir,
                patience,
                session: Mutex::new(session),
                wake: Condvar::new(),
                listeners: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Arranca el hilo que corre, de una en una, lo que se vaya encolando.
    pub fn start_the_runner(&self) {
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
                return Err("no se cambia de cliente con un informe en marcha".to_owned());
            }
            session.resolving_client = true;
            self.shared.publish(&session);
        }
        let candidates = the_candidates();
        let resolved = resolve(kind, binary, trust_root, &candidates);
        let mut session = self.shared.lock();
        session.resolving_client = false;
        session.candidates = candidates;
        session.profiles.clear();
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
        self.shared.list_the_reports(&mut session);
        self.shared.publish(&session);
        Ok(())
    }

    /// Busca los binarios que se pueden medir y los publica.
    pub fn discover_the_candidates(&self) {
        let candidates = the_candidates();
        let mut session = self.shared.lock();
        session.candidates = candidates;
        self.shared.list_the_reports(&mut session);
        self.shared.publish(&session);
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
            return Err("falta la versión del cliente: AutoFirma no la dice, escríbela".to_owned());
        }
        let coordinates = HeaderCoordinates {
            os: new.os,
            os_version: new.os_version,
            client_version: new.client_version,
            scope: self.a_scope_named_in_the_catalogue(new.scope)?,
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
            client.origin,
            &self.shared.catalogue,
            coordinates,
        )
        .inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&dir);
        })?;
        let mut report = report;
        if !new.title.is_empty() || !new.notes.is_empty() {
            let header = report.header();
            report
                .correct_header(
                    HeaderCorrection {
                        title: new.title,
                        notes: new.notes,
                        client_version: header.client_version.clone(),
                        os: header.os.clone(),
                        os_version: header.os_version.clone(),
                        scope: header.scope.clone(),
                    },
                    &self.shared.catalogue,
                )
                .inspect_err(|_| {
                    let _ = std::fs::remove_dir_all(&dir);
                })?;
        }
        self.shared.install(&mut session, new.name, dir, report);
        Ok(())
    }

    pub(crate) fn correct_header(&self, mut correction: HeaderCorrection) -> Result<(), String> {
        if correction.client_version.trim().is_empty() {
            return Err("falta la versión del cliente".to_owned());
        }
        correction.scope = self.a_scope_named_in_the_catalogue(correction.scope)?;
        let mut session = self.shared.lock();
        let open = session
            .report
            .as_mut()
            .ok_or("elige o abre un informe antes de corregir su cabecera")?;
        open.report
            .correct_header(correction, &self.shared.catalogue)?;
        self.shared.list_the_reports(&mut session);
        self.shared.publish(&session);
        Ok(())
    }

    /// El alcance `scope`, sin nada que el catálogo no tenga; uno en blanco es ninguno.
    fn a_scope_named_in_the_catalogue(
        &self,
        scope: Option<String>,
    ) -> Result<Option<String>, String> {
        let Some(scope) = scope.filter(|scope| !scope.trim().is_empty()) else {
            return Ok(None);
        };
        let unknown = resolve_the_selection(&scope, &self.shared.catalogue, None).unknown;
        if !unknown.is_empty() {
            return Err(format!(
                "el catálogo no tiene «{}»: corrige el alcance",
                unknown.join("», «")
            ));
        }
        Ok(Some(scope))
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
        let kind = report.kind();
        let restoring = the_client_to_restore(&session, &report);
        self.shared.install(&mut session, name, dir, report);
        match restoring {
            None => Ok(()),
            Some(Ok(binary)) => {
                drop(session);
                self.choose_client(kind, Some(binary), None)
            }
            Some(Err(complaint)) => {
                if session.client.is_none() {
                    session.client_complaints = vec![complaint];
                    self.shared.publish(&session);
                }
                Ok(())
            }
        }
    }

    /// Abre el último informe que se abrió en el directorio, y con él su cliente.
    pub fn reopen_the_last_report(&self) {
        let Ok(name) = std::fs::read_to_string(self.shared.reports_dir.join(THE_LAST_REPORT))
        else {
            return;
        };
        if let Err(complaint) = self.open_report(name.trim().to_owned()) {
            eprintln!("el último informe no se pudo reabrir: {complaint}");
        }
    }

    pub(crate) fn enqueue(&self, request: Request) -> Result<(), String> {
        let mut session = self.shared.lock();
        let open = session
            .report
            .as_ref()
            .ok_or("elige o crea un informe antes de ejecutar nada")?;
        let client = session.client.as_ref().ok_or(NO_CLIENT)?;
        let kind = client.kind;
        if let Some(complaint) = open
            .report
            .refuses_to_continue_with(client.kind, client.identity.version.as_deref())
        {
            return Err(complaint);
        }
        let selected = self.the_selected(&open.report, &request.selection)?;
        let relaunch = request.mode.unwrap_or_default() == RunMode::Relaunch;
        let in_batch = selected.len() > 1;
        let worth_running = |check: &Check| {
            relaunch
                || open.report.state_of(&check.id) == Some(CheckState::Pending)
                || ErrandKey::of(check).is_some()
        };
        let fresh: Vec<Queued> = selected
            .into_iter()
            .filter(|check| {
                request
                    .tranches
                    .is_none_or(|tranches| tranches.take(the_assistance_of(check, kind)))
            })
            .filter(|check| worth_running(check))
            .filter(|check| !session.is_queued_or_running(&check.id))
            .map(|check| Queued {
                id: check.id.clone(),
                in_batch,
                relaunch,
            })
            .collect();
        if fresh.is_empty() {
            return Err(
                "no hay nada que ejecutar en esa selección que no esté ya en la cola".to_owned(),
            );
        }
        if !session.batch_is_running() {
            session.batch = Some(Batch {
                started: Instant::now(),
                total: 0,
            });
        }
        if let Some(batch) = session.batch.as_mut() {
            batch.total += fresh.len();
        }
        session.queue.extend(fresh);
        sort_in_tranches(&mut session.queue, &self.shared.catalogue, kind);
        self.shared.wake.notify_one();
        self.shared.publish(&session);
        Ok(())
    }

    /// Deja pendiente lo que nombra `selection` en el informe abierto, sin los saludos que añade, y
    /// olvida sus trámites con un solo guardado.
    pub(crate) fn reset(&self, selection: &str) -> Result<(), String> {
        let mut session = self.shared.lock();
        let open = session
            .report
            .as_ref()
            .ok_or("elige o abre un informe antes de resetear nada")?;
        let named = self.the_named(&open.report, selection)?;
        if let Some(busy) = named
            .iter()
            .find(|check| session.is_queued_or_running(&check.id))
        {
            return Err(format!("«{}» está en la cola o en curso", busy.id));
        }
        let ids: Vec<String> = named.iter().map(|check| check.id.clone()).collect();
        let keys: Vec<ErrandKey> = named
            .iter()
            .filter_map(|check| ErrandKey::of(check))
            .collect();
        let open = session.report.as_mut().expect("el informe sigue abierto");
        open.report.forget(&ids, &keys)?;
        self.shared.publish(&session);
        Ok(())
    }

    /// Lo que resuelve `text` en `report`, con los saludos que añade; en blanco, todo su alcance.
    fn the_selected(&self, report: &Report, text: &str) -> Result<Vec<&Check>, String> {
        let ids = if text.trim().is_empty() {
            self.the_covered(report)
        } else {
            self.the_resolution_in(report, text)?.ids
        };
        Ok(self.the_checks_named(&ids))
    }

    /// Lo que nombra `text` en `report`, sin los saludos que añade; en blanco, todo su alcance.
    fn the_named(&self, report: &Report, text: &str) -> Result<Vec<&Check>, String> {
        let ids = if text.trim().is_empty() {
            self.the_covered(report)
        } else {
            let resolution = self.the_resolution_in(report, text)?;
            resolution
                .ids
                .into_iter()
                .filter(|id| !resolution.greetings_added.contains(id))
                .collect()
        };
        Ok(self.the_checks_named(&ids))
    }

    fn the_covered(&self, report: &Report) -> Vec<String> {
        self.shared
            .catalogue
            .iter()
            .filter(|check| report.covers(&check.id))
            .map(|check| check.id.clone())
            .collect()
    }

    /// Lo que resuelve `text` dentro del alcance de `report`, o por qué no vale como selección.
    fn the_resolution_in(
        &self,
        report: &Report,
        text: &str,
    ) -> Result<SelectionResolution, String> {
        let resolution = resolve_the_selection(
            text,
            &self.shared.catalogue,
            report.header().scope.as_deref(),
        );
        if !resolution.unknown.is_empty() {
            return Err(format!("no conozco «{}»", resolution.unknown.join("», «")));
        }
        if !resolution.out_of_scope.is_empty() {
            return Err(format!(
                "«{}» está fuera del alcance del informe: amplíalo si quieres medirlo",
                resolution.out_of_scope.join("», «")
            ));
        }
        Ok(resolution)
    }

    fn the_checks_named(&self, ids: &[String]) -> Vec<&Check> {
        self.shared
            .catalogue
            .iter()
            .filter(|check| ids.contains(&check.id))
            .collect()
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

    /// El registro de `check` en el informe `report`, o en el abierto si no se nombra ninguno.
    pub(crate) fn log_of(&self, report: Option<&str>, check: &str) -> Result<Vec<LogLine>, String> {
        let text = self.read_in_the_report(report, |dir| log_path_of(dir, check))?;
        Ok(text
            .lines()
            .filter(|line| !line.is_empty())
            .map(parse_log_line)
            .collect())
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

    /// Lo que resuelve `text` contra el catálogo y el alcance `scope`, sin tocar la sesión.
    pub(crate) fn resolve_selection(&self, text: &str, scope: Option<&str>) -> SelectionResolution {
        resolve_the_selection(text, &self.shared.catalogue, scope)
    }

    pub(crate) fn catalogue_sets(&self) -> Vec<CatalogueSet<'_>> {
        the_sets_of(&self.shared.catalogue)
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
        if report.header().scope.is_some() {
            return Err(
                "un informe con alcance no se valida: «validado» habla de la suite entera"
                    .to_owned(),
            );
        }
        let reference = read_the_reference(&the_reference_dir(), reference)?;
        Ok(validate(&report, &self.shared.catalogue, &reference))
    }
}

const NO_CLIENT: &str = "elige un cliente primero";
/// El fichero del directorio de informes que nombra el último abierto, que no es un informe.
const THE_LAST_REPORT: &str = ".last-report";
const NO_SWITCHING_REPORTS: &str = "no se cambia de informe con otro en marcha";

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
        if let Err(error) = std::fs::write(self.reports_dir.join(THE_LAST_REPORT), &name) {
            eprintln!("no se pudo apuntar «{name}» como el último informe: {error}");
        }
        session.report = Some(OpenReport { name, dir, report });
        session.reasons.clear();
        self.list_the_reports(session);
        self.publish(session);
    }

    /// Relee los informes del directorio, lo único que lo lee: publicar el estado no lo hace.
    fn list_the_reports(&self, session: &mut Session) {
        session.reports = reports_in(
            &self.reports_dir,
            session.client.as_ref(),
            &session.candidates,
        );
    }

    /// Las pendientes del informe abierto que tomaría cada petición de tramos.
    fn pending_by_tranches(&self, session: &Session) -> Option<PendingByTranches> {
        let open = session.report.as_ref()?;
        let kind = session.client.as_ref()?.kind;
        let pending: Vec<Assistance> = self
            .catalogue
            .iter()
            .filter(|check| open.report.covers(&check.id))
            .filter(|check| open.report.state_of(&check.id) == Some(CheckState::Pending))
            .map(|check| the_assistance_of(check, kind))
            .collect();
        let count = |tranches: Tranches| {
            pending
                .iter()
                .filter(|assistance| tranches.take(**assistance))
                .count()
        };
        Some(PendingByTranches {
            unattended: count(Tranches::Unattended),
            attended: count(Tranches::Attended),
            all: count(Tranches::All),
        })
    }

    fn state_of(&self, session: &Session) -> serde_json::Value {
        let activity = Activity {
            client: session.client.as_ref(),
            candidates: &session.candidates,
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
            reasons: Some(&session.reasons),
            batch: session.batch_view(),
            pending_by_tranches: self.pending_by_tranches(session),
        };
        let snapshot = snapshot_of(
            &self.catalogue,
            session
                .report
                .as_ref()
                .map(|open| (open.name.as_str(), &open.report)),
            &session.reports,
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

impl Witness for ConsoleWitness {
    fn started_at(&self) -> Instant {
        self.shared.lock().started.unwrap_or_else(Instant::now)
    }

    fn log_sink(&self) -> LiveLogSink {
        let (log, group) = {
            let session = self.shared.lock();
            (session.log.clone(), session.running.clone())
        };
        let shared = Arc::clone(&self.shared);
        LiveLogSink::new(move |provenance, line| {
            if let Some(log) = &log {
                log.write(&line);
            }
            for check in &group {
                let split = parse_log_line(&line);
                let payload = serde_json::to_value(LiveLine {
                    check: check.clone(),
                    provenance,
                    clock: split.clock,
                    text: split.text,
                })
                .expect("la línea se serializa");
                shared.broadcast(&event_frame("log", &payload));
            }
        })
    }

    fn suite_failure(&self, check: &str, why: &str) {
        self.harness(&format!("{check}: {why}"));
        let payload = serde_json::to_value(SuiteFailure {
            check: check.to_owned(),
            why: why.to_owned(),
        })
        .expect("el aviso se serializa");
        self.shared
            .broadcast(&event_frame("suite_failure", &payload));
    }

    fn driver_spawned(&self, pid: u32) {
        let mut session = self.shared.lock();
        if session.aborting {
            kill(pid);
        }
        session.driver = Some(pid);
    }

    fn driver_finished(&self) {
        self.shared.lock().driver = None;
    }

    fn aborted(&self) -> bool {
        self.shared.lock().aborting
    }

    fn a_dnie_is_in_the_reader(&self) -> bool {
        crate::card::a_dnie_is_in_the_reader()
    }
}

fn run_the_next_group(shared: &Arc<Shared>) {
    let Next {
        group,
        client,
        store,
        prepared,
        report,
        already,
    } = {
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
    let witness: Arc<dyn Witness> = Arc::new(ConsoleWitness {
        shared: Arc::clone(shared),
    });
    let profile = match prepared {
        Some(profile) => Ok(profile),
        None => the_profile_prepared_for(shared, &client, store, witness.as_ref()),
    };
    let settled = match profile {
        Ok(profile) => {
            let probe = Probe {
                client: client.binary.clone(),
                kind: client.kind,
                profile,
                report,
                patience: shared.patience,
                witness: Arc::clone(&witness),
                runner: Arc::clone(&shared.runner),
            };
            run_the_group_catching_panics(&probe, &checks, already.as_ref())
        }
        Err(why) => {
            for check in &checks {
                witness.suite_failure(&check.id, &why);
            }
            all_pending(&checks, &why).into()
        }
    };
    let mut session = shared.lock();
    let key = checks.first().and_then(|head| ErrandKey::of(head));
    settle(
        &mut session,
        &group,
        key,
        already,
        settled,
        witness.as_ref(),
    );
    session.running.clear();
    session.started = None;
    session.log = None;
    session.driver = None;
    session.aborting = false;
    shared.publish(&session);
}

/// El perfil de `store` para `client`, preparado ahora y guardado para el resto de la sesión.
fn the_profile_prepared_for(
    shared: &Shared,
    client: &Client,
    store: Store,
    witness: &dyn Witness,
) -> Result<Profile, String> {
    witness.harness(&format!(
        "preparo el perfil aislado del almacén {}",
        store.name()
    ));
    let profile = prepare(client, store).map_err(|complaints| {
        format!(
            "el perfil aislado del almacén {} no se pudo preparar: {}",
            store.name(),
            complaints.join(" · ")
        )
    })?;
    shared.lock().profiles.push(profile.clone());
    Ok(profile)
}

fn the_candidates() -> Vec<Candidate> {
    [ClientKind::Rfirma, ClientKind::Autofirma]
        .into_iter()
        .flat_map(|kind| the_recipe_of(kind).the_candidates())
        .collect()
}

fn run_the_group_catching_panics(
    probe: &Probe,
    checks: &[&Check],
    already: Option<&ObservedErrand>,
) -> Settled {
    catch_unwind(AssertUnwindSafe(|| probe.run_group(checks, already))).unwrap_or_else(|panic| {
        let why = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
            .unwrap_or_default();
        probe.witness.harness(&format!("el arnés reventó: {why}"));
        all_pending(checks, &format!("el arnés reventó: {why}")).into()
    })
}

/// Ordena la cola en tramos —ninguna, clic, persona— con los saludos delante de cada uno.
fn sort_in_tranches(queue: &mut VecDeque<Queued>, catalogue: &[Check], kind: ClientKind) {
    let key = |queued: &Queued| {
        catalogue
            .iter()
            .find(|check| check.id == queued.id)
            .map(|check| (the_assistance_of(check, kind), !check.greeting()))
            .unwrap_or_default()
    };
    queue.make_contiguous().sort_by_key(key);
}

/// Deja pendientes, sin correrlas, las de una tanda que detiene un saludo fallido de su familia.
fn stop_what_failed_greetings_stop(shared: &Shared, session: &mut Session) {
    let Some(open) = &session.report else {
        return;
    };
    let failed = the_greetings_already_failed(&shared.catalogue, |id| open.report.state_of(id));
    let mut stopped = Vec::new();
    session.queue.retain(|queued| {
        let greeting = shared
            .catalogue
            .iter()
            .find(|check| check.id == queued.id)
            .and_then(|check| the_greeting_that_stops(check, &failed));
        match greeting.filter(|_| queued.in_batch) {
            Some(greeting) => {
                stopped.push((
                    queued.id.clone(),
                    the_reason_behind_a_failed_greeting(greeting),
                ));
                false
            }
            None => true,
        }
    });
    session.reasons.extend(stopped);
}

/// Lo siguiente que corre: el grupo, con qué cliente y almacén, el perfil si ya está preparado, el
/// informe y el trámite ya observado que lo juzga.
struct Next {
    group: Vec<String>,
    client: Client,
    store: Store,
    prepared: Option<Profile>,
    report: PathBuf,
    already: Option<ObservedErrand>,
}

/// Saca de la cola lo siguiente que hay que correr y lo marca en curso.
fn take_the_next_group(shared: &Arc<Shared>, session: &mut Session) -> Option<Next> {
    stop_what_failed_greetings_stop(shared, session);
    while let Some(queued) = session.queue.pop_front() {
        let (Some(client), Some(open)) = (&session.client, &session.report) else {
            session.queue.clear();
            return None;
        };
        let Some(head) = shared.catalogue.iter().find(|check| check.id == queued.id) else {
            continue;
        };
        let client = client.clone();
        let dir = open.dir.clone();
        let queued_checks: Vec<&Check> = std::iter::once(head)
            .chain(
                session
                    .queue
                    .iter()
                    .filter(|other| other.relaunch == queued.relaunch)
                    .filter_map(|other| shared.catalogue.iter().find(|check| check.id == other.id)),
            )
            .collect();
        let group: Vec<String> = the_group_of(head, &queued_checks)
            .into_iter()
            .map(|check| check.id.clone())
            .collect();
        session.queue.retain(|other| !group.contains(&other.id));
        let already = ErrandKey::of(head)
            .filter(|_| !queued.relaunch)
            .and_then(|key| open.report.observed(&key).cloned());
        let store = head.store();
        let prepared = session
            .profiles
            .iter()
            .find(|profile| profile.store == store)
            .cloned();
        session.log = already
            .is_none()
            .then(|| CheckLog::open(&log_path_of(&dir, &group[0])).ok())
            .flatten();
        session.running = group.clone();
        session.started = Some(Instant::now());
        shared.publish(session);
        return Some(Next {
            group,
            client,
            store,
            prepared,
            report: dir,
            already,
        });
    }
    None
}

/// Apunta en el informe lo que dejó el grupo y el trámite nuevo con su clave, y copia a cada
/// miembro el registro y las tramas del trámite que compartieron.
fn settle(
    session: &mut Session,
    group: &[String],
    key: Option<ErrandKey>,
    already: Option<ObservedErrand>,
    settled: Settled,
    witness: &dyn Witness,
) {
    let Some(open) = session.report.as_mut() else {
        return;
    };
    if let (Some(key), Some(observed)) = (key, settled.observed) {
        if let Err(complaint) = open.report.observe(key, observed) {
            witness.harness(&complaint);
        }
    }
    for settlement in settled.settlements {
        match settlement {
            Settlement::Resolved {
                id,
                outcome,
                observation,
                duration,
            } => {
                if let Err(complaint) = open.report.resolve(&id, outcome, observation, duration) {
                    witness.harness(&complaint);
                }
                session.reasons.remove(&id);
            }
            Settlement::Pending { id, why } => {
                session.reasons.insert(id, why);
            }
        }
    }
    let source = already.map_or_else(|| group[0].clone(), |observed| observed.transcribed_in);
    for member in group.iter().filter(|member| **member != source) {
        let _ = std::fs::copy(
            log_path_of(&open.dir, &source),
            log_path_of(&open.dir, member),
        );
        let _ = std::fs::copy(
            transcript_path_of(&open.dir, &source),
            transcript_path_of(&open.dir, member),
        );
    }
}

/// Los informes del directorio, cada uno con su cliente, su clase y si `client` lo continúa —o, sin
/// cliente activo, si el suyo se restaura de entre los `candidates`—, o la queja si no se lee.
fn reports_in(dir: &Path, client: Option<&Client>, candidates: &[Candidate]) -> Vec<ReportEntry> {
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
                Ok(report) => {
                    let why_not = why_not_continue(&report, client, candidates);
                    ReportEntry {
                        kind: Some(report.kind().name()),
                        client: Some(report.client().to_owned()),
                        client_version: Some(report.header().client_version.clone()),
                        title: Some(report.header().title.clone()),
                        date: Some(report.header().date.clone()),
                        continuable: why_not.is_none(),
                        why_not,
                        complaint: None,
                        name,
                    }
                }
                Err(complaint) => ReportEntry {
                    kind: None,
                    client: None,
                    client_version: None,
                    title: None,
                    date: None,
                    continuable: false,
                    why_not: None,
                    complaint: Some(complaint),
                    name,
                },
            },
        )
        .collect();
    reports.sort_by(|a, b| a.name.cmp(&b.name));
    reports
}

fn why_not_continue(
    report: &Report,
    client: Option<&Client>,
    candidates: &[Candidate],
) -> Option<String> {
    match client {
        Some(client) => {
            report.refuses_to_continue_with(client.kind, client.identity.version.as_deref())
        }
        None => the_binary_to_restore(report, candidates)
            .is_none()
            .then(|| the_missing_client_of(report)),
    }
}

/// El binario con que se restaura el cliente de `report`, o por qué no se encuentra; `None` si el
/// cliente activo ya lo continúa.
fn the_client_to_restore(session: &Session, report: &Report) -> Option<Result<PathBuf, String>> {
    let continues = session.client.as_ref().is_some_and(|client| {
        report
            .refuses_to_continue_with(client.kind, client.identity.version.as_deref())
            .is_none()
    });
    (!continues).then(|| {
        the_binary_to_restore(report, &session.candidates)
            .ok_or_else(|| the_missing_client_of(report))
    })
}

fn the_binary_to_restore(report: &Report, candidates: &[Candidate]) -> Option<PathBuf> {
    the_binary_to_restore_of(
        report.kind(),
        report.origin(),
        Path::new(report.client()),
        &report.header().client_version,
        candidates,
    )
}

fn the_missing_client_of(report: &Report) -> String {
    format!(
        "su cliente, {} {}, no está entre los candidatos ni en {}",
        report.kind().name(),
        report.header().client_version,
        report.client()
    )
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
}

fn kill(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Origin;
    use crate::errand::fake::RecordedRunner;

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

        let reports = reports_in(dir.path(), None, &[]);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].name, "b-informe");
        assert!(reports[0]
            .complaint
            .as_deref()
            .unwrap()
            .contains("no es un informe"));
    }

    const THREE_TRANCHES: &str = r#"
[[check]]
id = "a_person_one"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Uno."

[check.drive]
mode = "v4"
script = "protocol-v4"
act.cancel = "Cancela."
expects.completes = {}

[[check]]
id = "a_click_one"
set = "errores"
chapter = "15"
citation = "A.java:2"
statement = "Dos."

[check.drive]
mode = "v4"
script = "protocol-v4"
act.consent = "Elige."
expects.completes = {}

[[check]]
id = "an_unattended_one"
set = "errores"
chapter = "15"
citation = "A.java:3"
statement = "Tres."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes = {}

[[check]]
id = "a_click_greeting"
set = "errores"
chapter = "15"
citation = "A.java:4"
statement = "Cuatro."

[check.drive]
mode = "v4"
script = "protocol-v4"
act.consent = "Elige."
expects.completes = {}
greeting = true
"#;

    #[test]
    fn the_queue_runs_in_tranches_with_each_greeting_ahead_of_its_tranche() {
        let catalogue = crate::catalogue::the_catalogue_in(THREE_TRANCHES).unwrap();
        let mut queue: VecDeque<Queued> = catalogue
            .iter()
            .map(|check| Queued {
                id: check.id.clone(),
                in_batch: true,
                relaunch: false,
            })
            .collect();

        sort_in_tranches(&mut queue, &catalogue, ClientKind::Autofirma);

        assert_eq!(
            queue
                .iter()
                .map(|queued| queued.id.as_str())
                .collect::<Vec<_>>(),
            [
                "an_unattended_one",
                "a_click_greeting",
                "a_click_one",
                "a_person_one"
            ]
        );
    }

    #[test]
    fn each_tranche_request_takes_its_own_assistances() {
        let taken = |tranches: Tranches| {
            [Assistance::None, Assistance::Click, Assistance::Person]
                .into_iter()
                .filter(|assistance| tranches.take(*assistance))
                .collect::<Vec<_>>()
        };
        assert_eq!(taken(Tranches::Unattended), [Assistance::None]);
        assert_eq!(
            taken(Tranches::Attended),
            [Assistance::Click, Assistance::Person]
        );
        assert_eq!(taken(Tranches::All).len(), 3);
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

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes = {}
"#,
        )
        .unwrap()
    }

    fn a_report_in(dir: &Path, name: &str, kind: ClientKind, catalogue: &[Check]) {
        std::fs::create_dir(dir.join(name)).unwrap();
        Report::create(
            &dir.join(name).join(THE_REPORT_FILE),
            &format!("/nowhere/{}", kind.name()),
            kind,
            Origin::Installed,
            catalogue,
            HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.0".to_owned(),
                scope: None,
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

    fn an_rfirma_at(binary: &str, version: &str) -> Client {
        Client {
            kind: ClientKind::Rfirma,
            origin: Origin::Given,
            binary: PathBuf::from(binary),
            library_dir: None,
            identity: crate::client::Identity {
                version: Some(version.to_owned()),
                channel: None,
            },
            trust_root: None,
        }
    }

    #[test]
    fn each_report_listed_says_whether_the_active_client_continues_it_and_why_not() {
        let dir = tempfile::tempdir().unwrap();
        let catalogue = a_catalogue();
        a_report_in(
            dir.path(),
            "of-autofirma",
            ClientKind::Autofirma,
            &catalogue,
        );
        a_report_in(dir.path(), "of-rfirma", ClientKind::Rfirma, &catalogue);
        let console = Console::new(catalogue, dir.path().to_owned(), Duration::ZERO);
        {
            let mut session = console.shared.lock();
            session.client = Some(an_rfirma_at("/elsewhere/rfirma", "1.0"));
            console.shared.list_the_reports(&mut session);
        }

        let state = console.shared.state_of(&console.shared.lock());

        let reports = &state["reports"];
        assert_eq!(reports[0]["name"], "of-autofirma");
        assert_eq!(reports[0]["continuable"], false);
        assert!(reports[0]["why_not"]
            .as_str()
            .unwrap()
            .contains("para autofirma y el cliente activo es rfirma"));
        assert_eq!(reports[1]["continuable"], true);
        assert_eq!(reports[1]["why_not"], serde_json::Value::Null);
    }

    #[test]
    fn the_last_report_opened_is_open_again_when_the_session_starts_anew() {
        let dir = tempfile::tempdir().unwrap();
        let catalogue = a_catalogue();
        a_report_in(dir.path(), "a", ClientKind::Rfirma, &catalogue);
        a_report_in(dir.path(), "b", ClientKind::Rfirma, &catalogue);
        let before = Console::new(catalogue.clone(), dir.path().to_owned(), Duration::ZERO);
        before.open_report("b".to_owned()).unwrap();

        let after = Console::new(catalogue, dir.path().to_owned(), Duration::ZERO);
        after.reopen_the_last_report();

        let state = after.subscribe().recv().unwrap();
        assert!(state.contains("\"report_name\":\"b\""), "{state}");
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
    fn a_selection_is_resolved_without_a_report_or_a_client() {
        let dir = tempfile::tempdir().unwrap();
        let console = Console::new(a_catalogue(), dir.path().to_owned(), Duration::ZERO);

        let resolution = console.resolve_selection("saludo, nadie", None);

        assert_eq!(resolution.ids, ["a_greeting"]);
        assert_eq!(resolution.unknown, ["nadie"]);
    }

    #[test]
    fn the_live_log_reaches_every_member_of_a_group_with_its_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let console = Console::new(a_catalogue(), dir.path().to_owned(), Duration::ZERO);
        console.shared.lock().running = vec!["first".to_owned(), "second".to_owned()];
        let frames = console.subscribe();
        frames.recv().unwrap();

        ConsoleWitness {
            shared: Arc::clone(&console.shared),
        }
        .harness("aviso");

        let lines: Vec<serde_json::Value> = frames
            .try_iter()
            .map(|frame| {
                let data = frame
                    .lines()
                    .find_map(|l| l.strip_prefix("data: "))
                    .unwrap();
                serde_json::from_str(data).unwrap()
            })
            .collect();
        let seen: Vec<_> = lines
            .iter()
            .map(|line| {
                (
                    line["check"].as_str().unwrap(),
                    line["provenance"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(seen, [("first", "suite"), ("second", "suite")]);
    }

    #[test]
    fn an_event_is_framed_for_server_sent_events() {
        assert_eq!(
            event_frame("log", &serde_json::json!({"line": "x"})),
            "event: log\ndata: {\"line\":\"x\"}\n\n"
        );
    }

    const A_BATCH_WITH_THREE_KEYS: &str = r#"
[[check]]
id = "a_rejection"
set = "errores"
chapter = "15"
citation = "A.java:1"
statement = "Se rechaza."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
expects.code = "SAF_03"

[[check]]
id = "its_twin"
set = "parametros"
chapter = "15"
citation = "A.java:2"
statement = "También."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
expects.code = "SAF_03"

[[check]]
id = "the_same_in_ec"
set = "errores"
chapter = "15"
citation = "A.java:3"
statement = "En curva elíptica."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
store = "ec"
expects.code = "SAF_03"

[[check]]
id = "an_echo"
set = "errores"
chapter = "05"
citation = "A.java:4"
statement = "Un eco."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects = "silence"

[[check]]
id = "the_same_with_a_click"
set = "errores"
chapter = "15"
citation = "A.java:5"
statement = "Lo mismo, en el tramo de clic."

[check.drive]
mode = "v4"
script = "signwithoutaformat"
act.consent = "Elige."
expects.code = "SAF_03"
"#;

    fn a_console_replaying(runner: &Arc<RecordedRunner>) -> (Console, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let catalogue = crate::catalogue::the_catalogue_in(A_BATCH_WITH_THREE_KEYS).unwrap();
        a_report_in(dir.path(), "a-report", ClientKind::Rfirma, &catalogue);
        let console = Console::running_errands_with(
            catalogue,
            dir.path().to_owned(),
            Duration::from_secs(1),
            Arc::clone(runner) as Arc<dyn ErrandRunner>,
        );
        {
            let mut session = console.shared.lock();
            session.client = Some(Client {
                kind: ClientKind::Rfirma,
                origin: crate::client::Origin::Installed,
                binary: PathBuf::from("/usr/bin/rfirma"),
                library_dir: None,
                identity: crate::client::Identity::default(),
                trust_root: None,
            });
            session.profiles = Store::ALL
                .into_iter()
                .map(|store| Profile {
                    store,
                    ..crate::client::a_profile_in(&dir.path().join(store.name()))
                })
                .collect();
        }
        console.open_report("a-report".to_owned()).unwrap();
        (console, dir)
    }

    fn a_rejecting_runner() -> Arc<RecordedRunner> {
        RecordedRunner::replaying(&[(
            "signwithoutaformat",
            crate::errand::the_recorded("a-rejection-with-saf-03"),
        )])
    }

    fn run_the_queue(console: &Console) {
        while !console.shared.lock().queue.is_empty() {
            run_the_next_group(&console.shared);
        }
    }

    fn the_state_of(console: &Console, id: &str) -> Option<CheckState> {
        let session = console.shared.lock();
        session.report.as_ref().unwrap().report.state_of(id)
    }

    fn reusing(selection: &str) -> Request {
        Request {
            selection: selection.to_owned(),
            mode: None,
            tranches: None,
        }
    }

    fn relaunching(selection: &str) -> Request {
        Request {
            mode: Some(RunMode::Relaunch),
            ..reusing(selection)
        }
    }

    /// La consola de `a_console_replaying` con AutoFirma, que no tiene mano: el clic es su tramo.
    fn a_console_of_autofirma_replaying(
        runner: &Arc<RecordedRunner>,
    ) -> (Console, tempfile::TempDir) {
        let (console, dir) = a_console_replaying(runner);
        let catalogue = console.shared.catalogue.clone();
        a_report_in(
            dir.path(),
            "of-autofirma",
            ClientKind::Autofirma,
            &catalogue,
        );
        if let Some(client) = console.shared.lock().client.as_mut() {
            client.kind = ClientKind::Autofirma;
            client.identity.version = Some("1.0".to_owned());
        }
        console.open_report("of-autofirma".to_owned()).unwrap();
        (console, dir)
    }

    fn the_published_state(console: &Console) -> serde_json::Value {
        console.shared.state_of(&console.shared.lock())
    }

    const COMPLIANT: Option<CheckState> =
        Some(CheckState::Resolved(crate::outcome::Outcome::Compliant));

    #[test]
    fn a_batch_launches_the_client_once_per_distinct_errand_key() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);

        console.enqueue(reusing("")).unwrap();
        run_the_queue(&console);

        assert_eq!(
            runner.runs(),
            [
                "v4/signwithoutaformat",
                "v4/signwithoutaformat",
                "v4/protocol-v4"
            ]
        );
        for id in [
            "a_rejection",
            "its_twin",
            "the_same_in_ec",
            "the_same_with_a_click",
        ] {
            assert_eq!(the_state_of(&console, id), COMPLIANT, "{id}");
        }
    }

    #[test]
    fn a_check_whose_errand_is_already_in_the_report_does_not_launch_the_client() {
        let runner = a_rejecting_runner();
        let (console, dir) = a_console_replaying(&runner);
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);

        console.enqueue(reusing("the_same_with_a_click")).unwrap();
        run_the_queue(&console);

        assert_eq!(runner.runs(), ["v4/signwithoutaformat"]);
        assert_eq!(the_state_of(&console, "the_same_with_a_click"), COMPLIANT);
        let transcripts = dir.path().join("a-report");
        assert_eq!(
            std::fs::read_to_string(transcript_path_of(&transcripts, "the_same_with_a_click"))
                .unwrap(),
            std::fs::read_to_string(transcript_path_of(&transcripts, "a_rejection")).unwrap()
        );
    }

    #[test]
    fn running_a_selection_judges_what_is_resolved_again_with_its_saved_errands() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("")).unwrap();
        run_the_queue(&console);

        console.enqueue(reusing("errores")).unwrap();
        run_the_queue(&console);

        assert_eq!(runner.runs().len(), 3);
        assert_eq!(the_state_of(&console, "a_rejection"), COMPLIANT);
    }

    #[test]
    fn relaunching_a_selection_opens_the_client_for_each_errand_and_replaces_the_saved_one() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("")).unwrap();
        run_the_queue(&console);

        console
            .enqueue(relaunching("a_rejection its_twin"))
            .unwrap();
        run_the_queue(&console);
        console.enqueue(reusing("the_same_with_a_click")).unwrap();
        run_the_queue(&console);

        assert_eq!(
            runner.runs(),
            [
                "v4/signwithoutaformat",
                "v4/signwithoutaformat",
                "v4/protocol-v4",
                "v4/signwithoutaformat"
            ]
        );
        assert_eq!(the_state_of(&console, "its_twin"), COMPLIANT);
    }

    #[test]
    fn relaunching_a_check_leaves_the_others_sharing_its_errand_untouched() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);

        console.enqueue(relaunching("a_rejection")).unwrap();
        run_the_queue(&console);

        assert_eq!(runner.runs().len(), 2);
        assert_eq!(
            the_state_of(&console, "its_twin"),
            Some(CheckState::Pending)
        );
    }

    #[test]
    fn a_selection_runs_only_the_tranches_asked_for() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_of_autofirma_replaying(&runner);

        console
            .enqueue(Request {
                tranches: Some(Tranches::Attended),
                ..reusing("errores")
            })
            .unwrap();

        let queued: Vec<String> = console
            .shared
            .lock()
            .queue
            .iter()
            .map(|queued| queued.id.clone())
            .collect();
        assert_eq!(queued, ["the_same_with_a_click"]);
    }

    #[test]
    fn a_report_is_continued_by_its_class_and_version_and_not_by_the_path_of_the_binary() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.shared.lock().client = Some(an_rfirma_at("/elsewhere/rfirma", "1.0"));
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);
        assert_eq!(the_state_of(&console, "a_rejection"), COMPLIANT);

        console.shared.lock().client = Some(an_rfirma_at("/usr/bin/rfirma", "2.0"));
        let refused = console.enqueue(reusing("its_twin")).unwrap_err();

        assert!(
            refused.contains("con la versión 1.0 y el cliente activo es la 2.0"),
            "{refused}"
        );
    }

    #[test]
    fn a_selection_already_in_the_queue_is_refused() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("errores")).unwrap();

        let refused = console.enqueue(reusing("a_rejection")).unwrap_err();

        assert!(refused.contains("ya en la cola"), "{refused}");
    }

    #[test]
    fn a_selection_naming_what_the_catalogue_lacks_is_refused_naming_it() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);

        let refused = console.enqueue(reusing("errores nowhere")).unwrap_err();
        let not_reset = console.reset("nowhere").unwrap_err();

        assert!(refused.contains("«nowhere»"), "{refused}");
        assert!(not_reset.contains("«nowhere»"), "{not_reset}");
        assert!(console.shared.lock().queue.is_empty());
    }

    #[test]
    fn the_errand_observed_in_a_session_is_reused_after_reopening_the_report() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);

        console.open_report("a-report".to_owned()).unwrap();
        console.enqueue(reusing("its_twin")).unwrap();
        run_the_queue(&console);

        assert_eq!(runner.runs().len(), 1);
    }

    #[test]
    fn a_reset_check_is_pending_again_and_runs_its_errand_anew() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);

        console.reset("a_rejection").unwrap();

        assert_eq!(
            the_state_of(&console, "a_rejection"),
            Some(CheckState::Pending)
        );
        console.enqueue(reusing("a_rejection")).unwrap();
        run_the_queue(&console);
        assert_eq!(runner.runs().len(), 2);
    }

    #[test]
    fn resetting_a_selection_leaves_the_checks_outside_that_shared_its_errand_with_their_result() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("")).unwrap();
        run_the_queue(&console);

        console.reset("errores").unwrap();

        for id in [
            "a_rejection",
            "the_same_in_ec",
            "an_echo",
            "the_same_with_a_click",
        ] {
            assert_eq!(
                the_state_of(&console, id),
                Some(CheckState::Pending),
                "{id}"
            );
        }
        assert_eq!(the_state_of(&console, "its_twin"), COMPLIANT);
        console.enqueue(reusing("its_twin")).unwrap();
        run_the_queue(&console);
        assert_eq!(runner.runs().len(), 4);
    }

    #[test]
    fn a_selection_with_a_check_in_the_queue_is_not_reset() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.enqueue(reusing("a_rejection")).unwrap();

        let refused = console.reset("errores").unwrap_err();

        assert!(refused.contains("«a_rejection»"), "{refused}");
    }

    #[test]
    fn the_state_counts_the_batch_and_the_pending_checks_of_each_tranche() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_of_autofirma_replaying(&runner);
        let pending_before = the_published_state(&console)["pending_by_tranches"].clone();
        let frames = console.subscribe();
        console.enqueue(reusing("")).unwrap();

        run_the_next_group(&console.shared);

        let batches: Vec<serde_json::Value> = frames
            .try_iter()
            .filter_map(|frame| {
                let data = frame.strip_prefix("event: state\ndata: ")?;
                let state: serde_json::Value = serde_json::from_str(data.trim_end()).ok()?;
                let mut batch = state["batch"].as_object()?.clone();
                batch.remove("elapsed_ms");
                Some(serde_json::Value::Object(batch))
            })
            .collect();
        assert_eq!(
            pending_before,
            serde_json::json!({"unattended": 4, "attended": 1, "all": 5})
        );
        assert_eq!(
            batches,
            [
                serde_json::json!({"total": 5, "done": 0, "running": 0}),
                serde_json::json!({"total": 5, "done": 0, "running": 2}),
                serde_json::json!({"total": 5, "done": 2, "running": 0}),
            ]
        );
        run_the_queue(&console);
        assert_eq!(
            the_published_state(&console)["batch"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn publishing_the_state_does_not_read_the_reports_again() {
        let runner = a_rejecting_runner();
        let (console, dir) = a_console_replaying(&runner);
        let catalogue = console.shared.catalogue.clone();
        a_report_in(dir.path(), "written-behind", ClientKind::Rfirma, &catalogue);
        let names = |state: &serde_json::Value| -> Vec<String> {
            state["reports"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| entry["name"].as_str().unwrap().to_owned())
                .collect()
        };

        console.enqueue(reusing("a_rejection")).unwrap();
        let published = the_published_state(&console);
        run_the_queue(&console);
        console.open_report("written-behind".to_owned()).unwrap();

        assert_eq!(names(&published), ["a-report"]);
        assert_eq!(
            names(&the_published_state(&console)),
            ["a-report", "written-behind"]
        );
    }

    #[test]
    fn a_new_report_is_proposed_a_name_and_a_version_from_its_client() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.shared.lock().client = Some(an_rfirma_at("/usr/bin/rfirma", "0.10.0+rades"));

        let proposed = serde_json::to_value(console.the_deduced_coordinates().unwrap()).unwrap();

        assert_eq!(proposed["client_version"], "0.10.0+rades");
        assert_eq!(
            proposed["name"],
            format!("rfirma-0.10.0-rades-{}", crate::report::today())
        );
        assert!(is_a_report_name(proposed["name"].as_str().unwrap()));
    }

    #[test]
    fn the_header_of_an_open_report_is_corrected_and_the_view_shows_it() {
        let runner = a_rejecting_runner();
        let (console, dir) = a_console_replaying(&runner);

        console
            .correct_header(HeaderCorrection {
                title: "Prueba de rades".to_owned(),
                notes: "Con el build local".to_owned(),
                client_version: "0.11.0".to_owned(),
                os: "Fedora".to_owned(),
                os_version: "44".to_owned(),
                scope: None,
            })
            .unwrap();

        let view = console.report_view("a-report").unwrap();
        assert_eq!(view["header"]["title"], "Prueba de rades");
        assert_eq!(view["header"]["notes"], "Con el build local");
        assert_eq!(view["header"]["client_version"], "0.11.0");
        assert_eq!(view["header"]["os"], "Fedora");
        assert_eq!(view["header"]["os_version"], "44");
        assert!(dir.path().join("a-report").is_dir());
    }

    #[test]
    fn a_header_without_a_version_is_not_accepted_as_a_correction() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);

        let refused = console
            .correct_header(HeaderCorrection {
                title: String::new(),
                notes: String::new(),
                client_version: " ".to_owned(),
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                scope: None,
            })
            .unwrap_err();

        assert!(refused.contains("falta la versión"), "{refused}");
    }

    #[test]
    fn a_new_report_keeps_the_title_and_notes_it_was_created_with() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.shared.lock().client = Some(an_rfirma_at("/usr/bin/rfirma", "0.10.0"));

        console
            .create_report(NewReport {
                name: "fresh".to_owned(),
                title: "Prueba de rades".to_owned(),
                notes: "Con el build local".to_owned(),
                client_version: "0.10.0".to_owned(),
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                scope: None,
            })
            .unwrap();

        let view = console.report_view("fresh").unwrap();
        assert_eq!(view["header"]["title"], "Prueba de rades");
        assert_eq!(view["header"]["notes"], "Con el build local");
    }

    #[test]
    fn a_new_report_is_proposed_a_title_from_its_client() {
        let runner = a_rejecting_runner();
        let (console, _dir) = a_console_replaying(&runner);
        console.shared.lock().client = Some(an_rfirma_at("/usr/bin/rfirma", "0.10.0"));

        let proposed = serde_json::to_value(console.the_deduced_coordinates().unwrap()).unwrap();

        assert_eq!(proposed["title"], "rfirma 0.10.0");
    }

    #[test]
    fn a_report_written_before_titles_and_notes_is_read_with_both_empty() {
        let runner = a_rejecting_runner();
        let (console, dir) = a_console_replaying(&runner);
        let path = dir.path().join("a-report").join(THE_REPORT_FILE);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let header = json["header"].as_object_mut().unwrap();
        header.remove("title");
        header.remove("notes");
        std::fs::write(&path, json.to_string()).unwrap();

        let view = console.report_view("a-report").unwrap();

        assert_eq!(view["header"]["title"], "");
        assert_eq!(view["header"]["notes"], "");
    }

    fn a_scoped_entry(id: &str, set: &str, script: &str, extra: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"{set}\"\nchapter = \"15\"\ncitation = \"A.java:1\"\n\
             statement = \"Algo.\"\n\n[check.drive]\nmode = \"v4\"\nscript = \"{script}\"\n{extra}\n"
        )
    }

    fn a_catalogue_to_scope(more: &[String]) -> Vec<Check> {
        let entries = [
            a_scoped_entry(
                "ws_greeting",
                "saludo",
                "protocol-v4",
                "greeting = true\nexpects.completes = {}",
            ),
            a_scoped_entry(
                "ws_echo",
                "canal.websocket",
                "protocol-v4",
                "expects.completes = {}",
            ),
            a_scoped_entry(
                "a_rejection",
                "errores",
                "signwithoutaformat",
                "expects.code = \"SAF_03\"",
            ),
            a_scoped_entry(
                "its_twin",
                "parametros",
                "signwithoutaformat",
                "expects.code = \"SAF_03\"",
            ),
        ];
        let mut catalogue =
            crate::catalogue::the_catalogue_in(&[&entries[..], more].concat().concat()).unwrap();
        for check in &mut catalogue {
            check.family = Some(
                match check
                    .provocation()
                    .map_or("", |provocation| provocation.script.as_str())
                {
                    "protocol-v4" => crate::manifest::Family::V4Echo,
                    _ => crate::manifest::Family::EndToEnd,
                },
            );
        }
        catalogue
    }

    fn a_console_to_scope(dir: &Path, catalogue: Vec<Check>) -> Console {
        let console = Console::running_errands_with(
            catalogue,
            dir.to_owned(),
            Duration::from_secs(1),
            a_rejecting_runner() as Arc<dyn ErrandRunner>,
        );
        {
            let mut session = console.shared.lock();
            session.client = Some(an_rfirma_at("/usr/bin/rfirma", "1.0"));
            session.profiles = Store::ALL
                .into_iter()
                .map(|store| Profile {
                    store,
                    ..crate::client::a_profile_in(&dir.join(store.name()))
                })
                .collect();
        }
        console
    }

    fn a_report_scoped_to(scope: Option<&str>) -> NewReport {
        NewReport {
            name: "scoped".to_owned(),
            title: "Con alcance".to_owned(),
            notes: String::new(),
            client_version: "1.0".to_owned(),
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            scope: scope.map(str::to_owned),
        }
    }

    fn a_correction_to(scope: Option<&str>) -> HeaderCorrection {
        HeaderCorrection {
            title: String::new(),
            notes: String::new(),
            client_version: "1.0".to_owned(),
            os: "Linux".to_owned(),
            os_version: "6.0".to_owned(),
            scope: scope.map(str::to_owned),
        }
    }

    fn the_ids_in(view: &serde_json::Value) -> Vec<&str> {
        view["sets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|set| set["checks"].as_array().unwrap())
            .map(|check| check["id"].as_str().unwrap())
            .collect()
    }

    #[test]
    fn a_new_report_with_a_name_the_catalogue_lacks_is_refused_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));

        let refused = console
            .create_report(a_report_scoped_to(Some("canal, no_such_check")))
            .unwrap_err();

        assert!(refused.contains("no_such_check"), "{refused}");
        assert!(!refused.contains("canal"), "{refused}");
        assert!(!dir.path().join("scoped").exists());
    }

    #[test]
    fn a_new_report_counts_and_shows_only_its_scope_with_the_greetings_it_adds() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));

        console
            .create_report(a_report_scoped_to(Some("canal")))
            .unwrap();

        let view = console.report_view("scoped").unwrap();
        assert_eq!(the_ids_in(&view), ["ws_greeting", "ws_echo"]);
        assert_eq!(view["summary"]["total"], 2);
        assert_eq!(view["summary"]["pending"], 2);
        assert_eq!(view["header"]["scope"], "canal");
    }

    #[test]
    fn a_report_with_a_scope_is_not_validated_against_the_reference() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));
        console
            .create_report(a_report_scoped_to(Some("canal")))
            .unwrap();
        let reference = the_references_in(&the_reference_dir()).unwrap().remove(0);

        let refused = console.validate("scoped", &reference).unwrap_err();

        assert!(refused.contains("alcance"), "{refused}");
    }

    #[test]
    fn a_check_outside_the_scope_cannot_be_queued() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));
        console
            .create_report(a_report_scoped_to(Some("errores")))
            .unwrap();

        let refused = console.enqueue(reusing("its_twin")).unwrap_err();
        let not_reset = console.reset("parametros").unwrap_err();
        console.enqueue(reusing("")).unwrap();

        assert!(refused.contains("fuera del alcance"), "{refused}");
        assert!(not_reset.contains("fuera del alcance"), "{not_reset}");
        let queued: Vec<String> = console
            .shared
            .lock()
            .queue
            .iter()
            .map(|queued| queued.id.clone())
            .collect();
        assert_eq!(queued, ["a_rejection"]);
    }

    #[test]
    fn a_set_in_the_scope_takes_in_the_checks_the_catalogue_gains_later() {
        let dir = tempfile::tempdir().unwrap();
        a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]))
            .create_report(a_report_scoped_to(Some("canal")))
            .unwrap();
        let grown = a_catalogue_to_scope(&[a_scoped_entry(
            "ws_later",
            "canal.websocket.later",
            "protocol-v4-echo",
            "expects.completes = {}",
        )]);

        let view = a_console_to_scope(dir.path(), grown)
            .report_view("scoped")
            .unwrap();

        assert_eq!(the_ids_in(&view), ["ws_greeting", "ws_echo", "ws_later"]);
    }

    #[test]
    fn narrowing_the_scope_keeps_the_results_outside_uncounted_and_widening_brings_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));
        console
            .create_report(a_report_scoped_to(Some("errores parametros")))
            .unwrap();
        console.enqueue(reusing("")).unwrap();
        run_the_queue(&console);
        let before = console.report_view("scoped").unwrap();

        console
            .correct_header(a_correction_to(Some("errores")))
            .unwrap();
        let narrowed = console.report_view("scoped").unwrap();
        console.correct_header(a_correction_to(None)).unwrap();
        let widened = console.report_view("scoped").unwrap();

        assert_eq!(the_ids_in(&narrowed), ["a_rejection"]);
        assert_eq!(narrowed["summary"]["total"], 1);
        assert_eq!(narrowed["orphans"], serde_json::json!([]));
        assert_eq!(widened["header"]["scope"], serde_json::Value::Null);
        assert_eq!(widened["summary"]["total"], 4);
        let twin_in = |view: &serde_json::Value| {
            view["sets"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|set| set["checks"].as_array().unwrap())
                .find(|check| check["id"] == "its_twin")
                .map(|check| check["state"].clone())
        };
        assert_ne!(twin_in(&before), Some("PENDIENTE".into()));
        assert_eq!(twin_in(&widened), twin_in(&before));
    }

    #[test]
    fn a_correction_naming_what_the_catalogue_lacks_is_refused_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));
        console.create_report(a_report_scoped_to(None)).unwrap();

        let refused = console
            .correct_header(a_correction_to(Some("errores nowhere")))
            .unwrap_err();

        assert!(refused.contains("nowhere"), "{refused}");
        let view = console.report_view("scoped").unwrap();
        assert_eq!(view["header"]["scope"], serde_json::Value::Null);
    }

    #[test]
    fn a_report_written_before_scopes_is_read_as_one_without_a_scope() {
        let dir = tempfile::tempdir().unwrap();
        let console = a_console_to_scope(dir.path(), a_catalogue_to_scope(&[]));
        console.create_report(a_report_scoped_to(None)).unwrap();
        let path = dir.path().join("scoped").join(THE_REPORT_FILE);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["header"].as_object_mut().unwrap().remove("scope");
        std::fs::write(&path, json.to_string()).unwrap();

        let view = console.report_view("scoped").unwrap();

        assert_eq!(view["header"]["scope"], serde_json::Value::Null);
        assert_eq!(view["summary"]["total"], 4);
    }
}
