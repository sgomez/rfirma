//! **Lo que la sede quiere**, sin versión de protocolo (RD-02).
//!
//! Es lo que el trámite consume: lo produce el códec
//! ([`super::ports::ProtocolCodec::decode`]) a partir del mensaje crudo, y de
//! ahí en adelante nadie vuelve a mirar cómo lo escribió la sede. Las tres
//! cosas que una sede puede querer de rFirma son elegir un certificado, firmar
//! un documento, y —cuando pide algo que no se atiende— que se le diga por qué
//! (ID-263, ID-264, ID-276).
//!
//! Las dos peticiones que se atienden viajan ya leídas ([`SelectCertificate`] y
//! [`SignRequest`]): el filtro que la sede declaró, y en la firma el documento
//! en bytes, la ronda, el algoritmo y los `extraParams` **sin expandir**
//! (ID-266), que es lo que el trámite necesita y nada de lo que el cable
//! añadía por encima.

use crate::protocol::{Refusal, SelectCertificate, SignRequest};

/// Lo que la sede pide, ya leído y sin versión.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteRequest {
    /// Elegir un certificado que pase este filtro: la sede pide identidad, no
    /// una firma (ID-276).
    SelectCertificate(SelectCertificate),
    /// Firmar estos bytes en PAdES con estas propiedades: `sign` o `cosign`
    /// (ID-263).
    Sign(SignRequest),
    /// Una operación que no se atiende, y el motivo con el que se le contesta
    /// a la sede: nace ya con su código del catálogo, porque el rechazo es del
    /// protocolo y no una situación nuestra que traducir (ID-288).
    NotAttended(Refusal),
}
