use super::super::*;
use crate::desktop::application::invocation::Invocation;
use crate::site::application::tests::InMemoryCaSlots;
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown, Situation,
};
use crate::site::domain::trust_error::TrustError;
use std::path::Path;
use std::sync::Mutex;

pub(super) const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

#[derive(Default)]
pub(super) struct World {
    pub(super) steps: Mutex<Vec<String>>,
    pub(super) every_port_taken: bool,
    pub(super) trusted: Mutex<Vec<(std::path::PathBuf, Vec<u8>)>>,
}

impl World {
    pub(super) fn note(&self, step: &str) {
        self.steps
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push(step.to_owned());
    }

    pub(super) fn steps(&self) -> Vec<String> {
        self.steps
            .lock()
            .expect("el doble no envenena su cerrojo")
            .clone()
    }

    pub(super) fn wait_until_shown(&self) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !self.steps().iter().any(|step| step == "ventana:enseñada")
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn port_of(location: &ChannelLocation) -> u16 {
        match location {
            ChannelLocation::Drawn(ports) | ChannelLocation::Service(ports) => {
                *ports.first().expect("puertos")
            }
            ChannelLocation::Fixed(port) => *port,
            ChannelLocation::Relay(_) => 0,
        }
    }

    pub(super) fn transport(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        self.note("canal");
        if self.every_port_taken {
            return Err(ChannelError::new(
                Situation::NoDrawnPortIsFree,
                "los tres puertos sorteados estan ocupados",
            ));
        }
        let port = Self::port_of(location);
        if matches!(
            (location, duty),
            (ChannelLocation::Relay(_), ChannelDuty::Serve(_))
        ) {
            return Ok(OpenChannel::with_delivery(
                port,
                Shutdown::of(|| {}),
                Delivery::of(|| {}),
            ));
        }
        Ok(OpenChannel::new(port, Shutdown::of(|| {})))
    }
}

impl SiteWindow for World {
    fn open(&self, content: SiteWindowContent<'_>) {
        self.note(&match content {
            SiteWindowContent::TheErrand(errand) => {
                format!("ventana:creada:{:?}", errand.arrival())
            }
            SiteWindowContent::ADeadEnd(dead_end) => note_of(&dead_end),
            SiteWindowContent::TheOldWebClientWarning => "ventana:aviso".to_owned(),
        });
    }

    fn show(&self) {
        self.note("ventana:enseñada");
    }

    fn hide(&self) {
        self.note("ventana:oculta");
    }

    fn close(&self) {
        self.note("ventana:cerrada");
    }

    fn errand_ended(&self, delivered: Acknowledgement) {
        delivered.wait(Duration::from_secs(1));
        self.note("ventana:trámite-terminado");
    }
}

fn note_of(dead_end: &DeadEnd) -> String {
    match dead_end {
        DeadEnd::ChannelNotOpened => "ventana:sin-puertos".to_owned(),
        DeadEnd::NoLocalCa => "ventana:sin-ca".to_owned(),
        DeadEnd::RefusedWithoutChannel(refusal) => format!("ventana:rechazo:{}", refusal.code()),
    }
}

const TRUSTED: u32 = 0x38;

impl TrustStores for World {
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        _nickname: &str,
    ) -> Result<(), TrustError> {
        self.note("confianza");
        self.trusted
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push((profile.to_path_buf(), certificate_der.to_vec()));
        Ok(())
    }

    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError> {
        let installed = self
            .trusted
            .lock()
            .expect("el doble no envenena su cerrojo")
            .iter()
            .any(|(where_, der)| where_ == profile && der == certificate_der);
        Ok(installed.then_some(TRUSTED))
    }

    fn withdraw(&self, profile: &Path, certificate_der: &[u8]) -> Result<(), TrustError> {
        self.note("retirada");
        self.trusted
            .lock()
            .expect("el doble no envenena su cerrojo")
            .retain(|(where_, der)| !(where_ == profile && der == certificate_der));
        Ok(())
    }
}

pub(super) fn a_store() -> InMemoryCaSlots {
    InMemoryCaSlots::default()
}

pub(super) fn a_codec_table() -> crate::site::application::site::CodecTable {
    crate::site::application::site::CodecTable {
        v4: std::sync::Arc::new(crate::site::adapters::codec::V4Codec),
        v3: std::sync::Arc::new(crate::site::adapters::codec_v3::V3Codec),
        v1: std::sync::Arc::new(|version| {
            std::sync::Arc::new(crate::site::adapters::codec_v1::V1Codec::new(version))
                as crate::site::application::errand::NegotiatedCodec
        }),
        relay: std::sync::Arc::new(|key, version| {
            std::sync::Arc::new(crate::site::adapters::codec_relay::RelayCodec::new(
                key, version,
            )) as crate::site::application::errand::NegotiatedCodec
        }),
    }
}

pub(super) fn invoked_with(arguments: &[&str]) -> Invocation {
    let mut command_line = vec!["rfirma".to_owned()];
    command_line.extend(arguments.iter().map(|argument| (*argument).to_string()));
    Invocation {
        command_line,
        folder: PathBuf::from("/tmp"),
    }
}

pub(super) fn a_launch(parameters: &str) -> String {
    format!("afirma://websocket?ports=51001,51002,51003&{parameters}")
}

pub(super) fn starting_with(
    world: &Arc<World>,
    store: &InMemoryCaSlots,
    invocation: &Invocation,
) -> Startup {
    let profiles = [PathBuf::from("/perfiles/firefox")];
    let live = LiveErrand::default();
    attend_startup(
        invocation.site_launch(),
        TrustAtStartup {
            store,
            profiles: &profiles,
            stores: &**world,
        },
        &a_codec_table(),
        &|location, duty| world.transport(location, duty),
        Arc::clone(world) as Arc<dyn SiteWindow>,
        &live,
    )
}
