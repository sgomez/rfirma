//! El estado de un proceso que ha cargado el módulo: sesiones, login, búsquedas y firmas en curso.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use cryptoki_sys::{
    CKF_SERIAL_SESSION, CKR_KEY_HANDLE_INVALID, CKR_MECHANISM_INVALID, CKR_OBJECT_HANDLE_INVALID,
    CKR_OK, CKR_OPERATION_ACTIVE, CKR_OPERATION_NOT_INITIALIZED, CKR_SESSION_HANDLE_INVALID,
    CKR_SESSION_PARALLEL_NOT_SUPPORTED, CKR_SLOT_ID_INVALID, CKR_USER_ALREADY_LOGGED_IN,
    CKR_USER_NOT_LOGGED_IN, CKR_USER_TYPE_INVALID, CKU_CONTEXT_SPECIFIC, CKU_USER,
    CK_ATTRIBUTE_TYPE, CK_FLAGS, CK_MECHANISM_TYPE, CK_OBJECT_HANDLE, CK_RV, CK_SESSION_HANDLE,
    CK_SLOT_ID, CK_USER_TYPE,
};

use crate::card::{self, Profile};
use crate::material::{self, Failure, Material};
use crate::objects::{self, key_role, KeyRole, Lookup, Object};
use crate::pin;
use crate::signing;

pub(crate) const SLOT: CK_SLOT_ID = 0;

#[derive(Default)]
struct Session {
    found: Option<Vec<CK_OBJECT_HANDLE>>,
    signing: Option<(CK_MECHANISM_TYPE, CK_OBJECT_HANDLE)>,
    signed_data: Vec<u8>,
    signature_authorized: bool,
}

pub(crate) struct Module {
    dir: PathBuf,
    material: Material,
    profile: Profile,
    objects: Vec<Object>,
    logged_in: bool,
    pin_signals: CK_FLAGS,
    interference_pending: bool,
    stale_sessions: BTreeSet<CK_SESSION_HANDLE>,
    sessions: BTreeMap<CK_SESSION_HANDLE, Session>,
    next_session: CK_SESSION_HANDLE,
}

impl Module {
    pub(crate) fn load(dir: PathBuf) -> Result<Self, Failure> {
        let material = material::load_or_generate(&dir)?;
        let profile = card::read_profile(&dir);
        let objects = objects::objects_of(&material, profile)?;
        Ok(Self {
            interference_pending: card::interference_configured(&dir),
            dir,
            material,
            profile,
            objects,
            logged_in: false,
            pin_signals: 0,
            stale_sessions: BTreeSet::new(),
            sessions: BTreeMap::new(),
            next_session: 1,
        })
    }

    pub(crate) fn pin_signals(&self) -> CK_FLAGS {
        match self.profile {
            Profile::Dnie => self.pin_signals,
            Profile::Signals => pin::standing_signals(card::read_tries_left(&self.dir)),
        }
    }

    pub(crate) fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub(crate) fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    pub(crate) fn open_session(
        &mut self,
        slot: CK_SLOT_ID,
        flags: CK_FLAGS,
    ) -> Result<CK_SESSION_HANDLE, CK_RV> {
        check_slot(slot)?;
        if flags & CKF_SERIAL_SESSION == 0 {
            return Err(CKR_SESSION_PARALLEL_NOT_SUPPORTED);
        }
        let handle = self.next_session;
        self.next_session += 1;
        self.sessions.insert(handle, Session::default());
        Ok(handle)
    }

    pub(crate) fn close_session(&mut self, handle: CK_SESSION_HANDLE) -> Result<(), CK_RV> {
        self.sessions
            .remove(&handle)
            .ok_or(CKR_SESSION_HANDLE_INVALID)?;
        if self.sessions.is_empty() {
            self.logged_in = false;
        }
        Ok(())
    }

    pub(crate) fn close_all_sessions(&mut self, slot: CK_SLOT_ID) -> Result<(), CK_RV> {
        check_slot(slot)?;
        self.sessions.clear();
        self.logged_in = false;
        Ok(())
    }

    pub(crate) fn check_session(&self, handle: CK_SESSION_HANDLE) -> Result<(), CK_RV> {
        self.sessions
            .contains_key(&handle)
            .then_some(())
            .ok_or(CKR_SESSION_HANDLE_INVALID)
    }

    pub(crate) fn login(
        &mut self,
        handle: CK_SESSION_HANDLE,
        user: CK_USER_TYPE,
        pin: Option<&[u8]>,
    ) -> Result<(), CK_RV> {
        self.check_session(handle)?;
        match user {
            CKU_USER => self.login_user(handle, pin),
            CKU_CONTEXT_SPECIFIC => self.login_for_signature(handle, pin),
            _ => Err(CKR_USER_TYPE_INVALID),
        }
    }

    fn login_user(&mut self, handle: CK_SESSION_HANDLE, pin: Option<&[u8]>) -> Result<(), CK_RV> {
        if self.logged_in {
            return Err(CKR_USER_ALREADY_LOGGED_IN);
        }
        if self.interfered_with(handle) {
            return Err(CKR_USER_NOT_LOGGED_IN);
        }
        self.submit_pin(pin)?;
        self.logged_in = true;
        Ok(())
    }

    fn login_for_signature(
        &mut self,
        handle: CK_SESSION_HANDLE,
        pin: Option<&[u8]>,
    ) -> Result<(), CK_RV> {
        if !self.logged_in {
            return Err(CKR_USER_NOT_LOGGED_IN);
        }
        if self.session(handle)?.signing.is_none() {
            return Err(CKR_OPERATION_NOT_INITIALIZED);
        }
        self.submit_pin(pin)?;
        self.session(handle)?.signature_authorized = true;
        Ok(())
    }

    /// Otro programa usa la tarjeta una vez: las sesiones abiertas hasta entonces pierden su canal.
    fn interfered_with(&mut self, handle: CK_SESSION_HANDLE) -> bool {
        if std::mem::take(&mut self.interference_pending) {
            self.stale_sessions.extend(self.sessions.keys().copied());
        }
        self.stale_sessions.remove(&handle)
    }

    fn submit_pin(&mut self, pin: Option<&[u8]>) -> Result<(), CK_RV> {
        let attempt = pin::verify(&self.dir, pin);
        if let Some(signals) = attempt.signals {
            self.pin_signals = signals;
        }
        match attempt.rv {
            CKR_OK => Ok(()),
            rv => Err(rv),
        }
    }

    pub(crate) fn logout(&mut self, handle: CK_SESSION_HANDLE) -> Result<(), CK_RV> {
        self.check_session(handle)?;
        if !self.logged_in {
            return Err(CKR_USER_NOT_LOGGED_IN);
        }
        self.logged_in = false;
        Ok(())
    }

    pub(crate) fn find_init(
        &mut self,
        handle: CK_SESSION_HANDLE,
        template: &[(CK_ATTRIBUTE_TYPE, Vec<u8>)],
    ) -> Result<(), CK_RV> {
        let found: Vec<CK_OBJECT_HANDLE> = self
            .visible_objects()
            .filter(|(_, object)| object.matches(template))
            .map(|(object_handle, _)| object_handle)
            .collect();
        let session = self.session(handle)?;
        if session.found.is_some() {
            return Err(CKR_OPERATION_ACTIVE);
        }
        session.found = Some(found);
        Ok(())
    }

    pub(crate) fn find(
        &mut self,
        handle: CK_SESSION_HANDLE,
        max: usize,
    ) -> Result<Vec<CK_OBJECT_HANDLE>, CK_RV> {
        let found = self
            .session(handle)?
            .found
            .as_mut()
            .ok_or(CKR_OPERATION_NOT_INITIALIZED)?;
        let taken = max.min(found.len());
        Ok(found.drain(..taken).collect())
    }

    pub(crate) fn find_final(&mut self, handle: CK_SESSION_HANDLE) -> Result<(), CK_RV> {
        self.session(handle)?
            .found
            .take()
            .map(|_| ())
            .ok_or(CKR_OPERATION_NOT_INITIALIZED)
    }

    pub(crate) fn object(
        &self,
        handle: CK_SESSION_HANDLE,
        object: CK_OBJECT_HANDLE,
    ) -> Result<&Object, CK_RV> {
        self.check_session(handle)?;
        self.visible_objects()
            .find(|(candidate, _)| *candidate == object)
            .map(|(_, found)| found)
            .ok_or(CKR_OBJECT_HANDLE_INVALID)
    }

    pub(crate) fn attribute(
        &self,
        handle: CK_SESSION_HANDLE,
        object: CK_OBJECT_HANDLE,
        kind: CK_ATTRIBUTE_TYPE,
    ) -> Result<Lookup<'_>, CK_RV> {
        Ok(self.object(handle, object)?.lookup(kind))
    }

    pub(crate) fn sign_init(
        &mut self,
        handle: CK_SESSION_HANDLE,
        mechanism: CK_MECHANISM_TYPE,
        key: CK_OBJECT_HANDLE,
    ) -> Result<(), CK_RV> {
        if !signing::MECHANISMS.contains(&mechanism) {
            return Err(CKR_MECHANISM_INVALID);
        }
        if !self.logged_in {
            return Err(CKR_USER_NOT_LOGGED_IN);
        }
        key_role(key).ok_or(CKR_KEY_HANDLE_INVALID)?;
        let session = self.session(handle)?;
        if session.signing.is_some() {
            return Err(CKR_OPERATION_ACTIVE);
        }
        session.signing = Some((mechanism, key));
        Ok(())
    }

    pub(crate) fn sign_update(
        &mut self,
        handle: CK_SESSION_HANDLE,
        data: &[u8],
    ) -> Result<(), CK_RV> {
        self.signing_operation(handle)?;
        self.session(handle)?.signed_data.extend_from_slice(data);
        Ok(())
    }

    /// La firma de lo acumulado por `sign_update`, sin cerrar la operación: la cierra `sign_done`.
    pub(crate) fn sign_final(&mut self, handle: CK_SESSION_HANDLE) -> Result<Vec<u8>, CK_RV> {
        self.signing_operation(handle)?;
        let data = self.session(handle)?.signed_data.clone();
        self.sign(handle, &data)
    }

    /// La firma de `data` con la operación en curso, sin cerrarla: la cierra `sign_done`.
    pub(crate) fn sign(
        &mut self,
        handle: CK_SESSION_HANDLE,
        data: &[u8],
    ) -> Result<Vec<u8>, CK_RV> {
        let (mechanism, key) = self.signing_operation(handle)?;
        let role = key_role(key).ok_or(CKR_KEY_HANDLE_INVALID)?;
        if self.requires_login_per_signature(role) && !self.session(handle)?.signature_authorized {
            return Err(CKR_USER_NOT_LOGGED_IN);
        }
        let signature = signing::sign(&self.material, role, mechanism, data);
        if signature.is_err() {
            self.sign_done(handle);
        }
        signature
    }

    pub(crate) fn signature_len(&mut self, handle: CK_SESSION_HANDLE) -> Result<usize, CK_RV> {
        let (_, key) = self.signing_operation(handle)?;
        let role = key_role(key).ok_or(CKR_KEY_HANDLE_INVALID)?;
        Ok(signing::signature_len(&self.material, role))
    }

    pub(crate) fn sign_done(&mut self, handle: CK_SESSION_HANDLE) {
        if let Some(session) = self.sessions.get_mut(&handle) {
            session.signing = None;
            session.signed_data.clear();
            session.signature_authorized = false;
        }
    }

    fn requires_login_per_signature(&self, role: KeyRole) -> bool {
        self.profile == Profile::Signals && role == KeyRole::Signing
    }

    fn signing_operation(
        &mut self,
        handle: CK_SESSION_HANDLE,
    ) -> Result<(CK_MECHANISM_TYPE, CK_OBJECT_HANDLE), CK_RV> {
        self.session(handle)?
            .signing
            .ok_or(CKR_OPERATION_NOT_INITIALIZED)
    }

    fn session(&mut self, handle: CK_SESSION_HANDLE) -> Result<&mut Session, CK_RV> {
        self.sessions
            .get_mut(&handle)
            .ok_or(CKR_SESSION_HANDLE_INVALID)
    }

    fn visible_objects(&self) -> impl Iterator<Item = (CK_OBJECT_HANDLE, &Object)> {
        let logged_in = self.logged_in;
        (1..)
            .zip(self.objects.iter())
            .filter(move |(_, object)| logged_in || !object.is_private())
    }
}

pub(crate) fn check_slot(slot: CK_SLOT_ID) -> Result<(), CK_RV> {
    if slot == SLOT {
        Ok(())
    } else {
        Err(CKR_SLOT_ID_INVALID)
    }
}
