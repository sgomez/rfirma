//! Las firmas de un documento, en árbol, y el aviso previo a firmar que componen; no las valida.

/// La validez de una firma, la misma en todas partes (ADR-0043).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validity {
    Valid,
    Expired,
    Invalid,
}

/// Por qué una firma está caducada o no es válida: el más grave de sus problemas (ADR-0043).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidityReason {
    /// El certificado caducó en `date`; `holder`, su nombre común, si no es el del firmante.
    CertificateExpired {
        date: String,
        holder: Option<String>,
    },
    ModifiedAfterSigning,
    /// Ilegible, o sin certificado de firma.
    Damaged,
    /// El certificado no entra en vigor hasta `date`.
    CertificateNotYetValid {
        date: String,
    },
    UnknownSignatureType,
    /// La firma resume con MD5 o MD2.
    UnsupportedAlgorithm,
    /// Cofirma de un documento que no admitía más firmas, cerrado por `closed_by`, su nombre común.
    CosignNotAdmitted {
        closed_by: Option<String>,
    },
}

/// La fecha de una firma: la declara quien firma o la prueba el sello de una TSA (ADR-0043).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SigningDate {
    /// El instante ISO-8601 que declara la propia firma.
    Declared { at: String },
    /// El instante ISO-8601 del sello de tiempo, y el nombre común de la TSA que lo selló.
    Stamped { at: String, tsa: String },
}

/// Lo que se ve en el documento entero, y no se cuelga de ninguna firma (ADR-0043).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentFinding {
    ModifiedAfterLastSignature,
    FormFilledAfterSigning,
    ContentAddedOnTop,
}

/// El tono del peor aviso, de menor a mayor gravedad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Todo bien.
    Information,
    /// Alguna firma está caducada o no es válida, o el documento cambió después de la última.
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
    /// El `organizationName` del sujeto, si el certificado lo lleva.
    pub organization_name: Option<String>,
    /// La autoridad emisora del certificado.
    pub issuer: String,
    /// El sujeto del certificado, como DN de RFC 4514.
    pub certificate_subject: String,
    /// El emisor del certificado, como DN de RFC 4514.
    pub certificate_issuer: String,
    /// Número de serie del certificado, distinto del `SERIALNUMBER` del sujeto.
    pub certificate_serial_number: String,
    /// El inicio de la vigencia del certificado, en ISO-8601, si el puente lo devolvió.
    pub certificate_valid_from: Option<String>,
    /// El fin de la vigencia del certificado, en ISO-8601, si el puente lo devolvió.
    pub certificate_valid_until: Option<String>,
    /// El algoritmo de la firma, como `SHA256withRSA`, si el puente lo devolvió.
    pub signature_algorithm: Option<String>,
    /// El perfil de la firma, tal como lo nombra el original, si el puente lo devolvió.
    pub profile: Option<String>,
    /// Instante de la firma en ISO-8601, si el puente lo devolvió.
    pub signing_time: Option<String>,
    /// La validez de la firma (ADR-0043).
    pub validity: Validity,
    /// El motivo de la validez, si no es `Valid`.
    pub validity_reason: Option<ValidityReason>,
    /// La fecha declarada o sellada, si la firma trae alguna.
    pub signing_date: Option<SigningDate>,
    /// Si es la firma de certificación que no admite más firmas.
    pub closes_document: bool,
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
    findings: Vec<DocumentFinding>,
}

impl DocumentSignatures {
    /// Construye el informe a partir de las firmas ya traducidas, en el orden en que llegaron.
    pub fn new(signatures: Vec<DocumentSignature>, changed_after_last_signature: bool) -> Self {
        Self {
            signatures,
            changed_after_last_signature,
            format: SignatureStandard::Pades,
            findings: Vec::new(),
        }
    }

    /// El mismo informe, con los hallazgos del documento.
    pub fn with_findings(self, findings: Vec<DocumentFinding>) -> Self {
        Self { findings, ..self }
    }

    /// Los hallazgos del documento, que no son de ninguna firma.
    pub fn findings(&self) -> &[DocumentFinding] {
        &self.findings
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

    /// Avisos: firmas caducadas o no válidas + 1 si el documento cambió.
    pub fn warning_count(&self) -> usize {
        self.signatures
            .iter()
            .filter(|s| s.validity != Validity::Valid)
            .count()
            + usize::from(self.changed_after_last_signature)
    }

    /// El tono del peor aviso.
    pub fn tone(&self) -> Tone {
        if self.warning_count() > 0 {
            Tone::Attention
        } else {
            Tone::Information
        }
    }
}

#[cfg(test)]
mod tests;
