//! Las firmas de un documento, en árbol, y el aviso previo a firmar que componen; no las valida.

/// El estado de una firma previa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Se sostiene.
    Valid,
    /// El certificado con el que se firmó ya ha caducado.
    CertificateExpired,
    /// El certificado con el que se firmó todavía no era válido al firmar.
    CertificateNotYetValid,
    /// No corresponde con los datos, está dañada, o el PDF certificado no admitía firmas.
    Broken,
    /// Formato no reconocido: no se puede validar.
    Unverifiable,
    /// Perfil longevo con el certificado caducado (`SIGN_PROFILE_NOT_CHECKED` en el original).
    NotFullyChecked,
}

impl SignatureStatus {
    /// Son KO el certificado caducado, aún no válido, rota y no se puede validar.
    pub fn is_ko(self) -> bool {
        matches!(
            self,
            Self::CertificateExpired
                | Self::CertificateNotYetValid
                | Self::Broken
                | Self::Unverifiable
        )
    }

    /// El tono que aporta este estado por sí solo.
    fn tone(self) -> Tone {
        if self.is_ko() {
            Tone::Attention
        } else if self == Self::NotFullyChecked {
            Tone::Indeterminate
        } else {
            Tone::Information
        }
    }
}

/// El tono del peor aviso, de menor a mayor gravedad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Todo bien.
    Information,
    /// Alguna firma no se ha podido comprobar del todo.
    Indeterminate,
    /// Alguna firma es KO, o el documento cambió después de la última.
    Attention,
}

/// Firmante de una de las firmas que ya trae el documento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSignature {
    /// El nombre del titular, leído del `CN` del sujeto.
    pub name: String,
    /// El NIF, leído del `SERIALNUMBER` del sujeto.
    pub id_number: String,
    /// El `organizationIdentifier` del sujeto, si el certificado lo lleva.
    pub organization_identifier: Option<String>,
    /// La autoridad emisora del certificado.
    pub issuer: String,
    /// Número de serie del certificado, distinto del `SERIALNUMBER` del sujeto.
    pub certificate_serial_number: String,
    /// Instante de la firma en ISO-8601, si el puente lo devolvió.
    pub signing_time: Option<String>,
    /// El estado de la firma.
    pub status: SignatureStatus,
    /// Motivo del original, tal como lo nombra, si el estado no es `Valid`.
    pub reason: Option<String>,
    /// Las contrafirmas de esta firma; en PDF, siempre vacías.
    pub countersignatures: Vec<DocumentSignature>,
}

/// El formato de firma del documento, reconocido por su forma.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignatureStandard {
    #[default]
    Pades,
    Cades,
    Xades,
    /// Ni un PDF ni una firma CAdES o XAdES.
    Unrecognized,
}

/// Las firmas que ya trae el documento, en el orden cronológico que devuelve el puente.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentSignatures {
    signatures: Vec<DocumentSignature>,
    changed_after_last_signature: bool,
    format: SignatureStandard,
}

impl DocumentSignatures {
    /// Construye el informe a partir de las firmas ya traducidas, en el orden en que llegaron.
    pub fn new(signatures: Vec<DocumentSignature>, changed_after_last_signature: bool) -> Self {
        Self {
            signatures,
            changed_after_last_signature,
            format: SignatureStandard::Pades,
        }
    }

    /// El mismo informe, de un documento en ese formato.
    pub fn in_format(self, format: SignatureStandard) -> Self {
        Self { format, ..self }
    }

    /// El formato de firma del documento.
    pub fn format(&self) -> SignatureStandard {
        self.format
    }

    /// Cuántas firmas trae el documento.
    pub fn count(&self) -> usize {
        self.signatures.len()
    }

    /// Las firmas, en orden cronológico.
    pub fn signatures(&self) -> &[DocumentSignature] {
        &self.signatures
    }

    /// Las firmas, en propiedad.
    pub fn into_signatures(self) -> Vec<DocumentSignature> {
        self.signatures
    }

    /// Si el documento cambió después de la última firma.
    pub fn changed_after_last_signature(&self) -> bool {
        self.changed_after_last_signature
    }

    /// Avisos: firmas KO + firmas sin comprobar del todo + 1 si el documento cambió.
    pub fn warning_count(&self) -> usize {
        let from_signatures = self
            .signatures
            .iter()
            .filter(|s| s.status.is_ko() || s.status == SignatureStatus::NotFullyChecked)
            .count();
        from_signatures + usize::from(self.changed_after_last_signature)
    }

    /// El tono del peor aviso.
    pub fn tone(&self) -> Tone {
        let changed_tone = if self.changed_after_last_signature {
            Tone::Attention
        } else {
            Tone::Information
        };
        self.signatures
            .iter()
            .map(|s| s.status.tone())
            .chain(std::iter::once(changed_tone))
            .max()
            .unwrap_or(Tone::Information)
    }
}

#[cfg(test)]
mod tests;
