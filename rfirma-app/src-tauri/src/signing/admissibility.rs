//! Detección rápida de admisibilidad de documentos antes de solicitar el PIN.

use std::fmt;

/// La cabecera de cualquier PDF. La versión va detrás y no la miramos: firmar
/// un 1.4 y firmar un 2.0 es el mismo recorrido.
const HEADER: &[u8] = b"%PDF-";

/// Cuántos bytes del principio se miran buscando la cabecera.
///
/// No es siempre el byte 0: hay PDFs con basura delante —un `Content-Type`
/// pegado, los restos de un correo— que todos los visores abren, y rechazarlos
/// aquí sería más estricto que el resto del mundo. Adobe admite hasta 1024.
const HEADER_WINDOW: usize = 1024;

/// La entrada `/Encrypt` del tráiler, que es lo que hace que un PDF esté
/// cifrado. Con permisos restringidos o con contraseña: las dos cosas son esta
/// misma entrada, y por eso son la misma negativa.
const ENCRYPT: &[u8] = b"/Encrypt";

/// El método de transformación de una firma de certificación.
///
/// Va dentro del diccionario `/Reference` de la firma y en ningún otro sitio,
/// así que encontrarlo es encontrar un PDF certificado. AutoFirma lo rechaza
/// entero, sin mirar el `/P`, y aquí se hace lo mismo: es el oráculo.
const DOC_MDP: &[u8] = b"/DocMDP";

/// El `/ByteRange` de una firma ya puesta. **No es un motivo de rechazo**: es
/// justo el caso de la cofirma, y está aquí para poder decirlo.
const BYTE_RANGE: &[u8] = b"/ByteRange";

/// La clave `/SubFilter` de un diccionario de firma, que es la que dice **con
/// qué formato** se firmó.
const SUB_FILTER: &[u8] = b"/SubFilter";

/// Los cuatro subfiltros que el puente sabe leer, tal y como los nombra
/// `PdfUtil.SUPPORTED_SUBFILTERS` de la 1.9.2 —con la barra delante, porque ahí
/// se comparan contra el `toString()` de un `PdfName`—.
///
/// **Uno de los cuatro es `/ETSI.RFC3161`** —el sello de tiempo de documento— y
/// entre ellos **no está** `/adbe.x509.rsa.sha1`, que sí es un subfiltro de la
/// norma: la lista no es la del PDF, es la del original, y es la del original
/// la que decide si el puente aborta.
const KNOWN_SUB_FILTERS: [&[u8]; 4] = [
    b"/ETSI.RFC3161",
    b"/adbe.pkcs7.detached",
    b"/ETSI.CAdES.detached",
    b"/adbe.pkcs7.sha1",
];

/// Por qué no se puede firmar este documento.
///
/// Son situaciones, no mensajes: quien las traduce es el catálogo de cadenas
/// (ADR-0009), igual que con los errores del token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Lo que se ha abierto no es un PDF.
    NotAPdf,
    /// El PDF está cifrado: con contraseña, o con los permisos restringidos.
    /// Son la misma entrada del tráiler y la misma negativa.
    Encrypted,
    /// El PDF está **certificado**: su autor firmó prohibiendo cambios, y
    /// añadir una firma lo invalidaría.
    Certified,
}

impl Refusal {
    /// El nombre que cruza a la interfaz, que es el de la situación en el
    /// catálogo.
    pub fn situation(self) -> &'static str {
        match self {
            Self::NotAPdf => "notAPdf",
            Self::Encrypted => "documentEncrypted",
            Self::Certified => "documentCertified",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotAPdf => "el fichero no es un PDF",
            Self::Encrypted => {
                "el PDF está cifrado o tiene los permisos restringidos, y no se puede firmar \
                 sin quitarle la protección"
            }
            Self::Certified => {
                "el PDF está certificado por su autor, que prohibió los cambios: firmarlo \
                 invalidaría esa certificación"
            }
        })
    }
}

impl std::error::Error for Refusal {}

/// Un PDF que ha pasado las tres comprobaciones.
///
/// Es un **tipo** y no un `bool` por la misma razón que
/// [`CheckedFolder`](crate::destination::CheckedFolder): sin pasar por
/// [`AdmissibleDocument::check`] no hay forma de nombrar los bytes que se le
/// pasan a la prefirma, así que no hace falta acordarse de comprobar.
#[derive(Clone, Copy, Debug)]
pub struct AdmissibleDocument<'a> {
    pdf: &'a [u8],
    already_signed: bool,
    unregistered_signatures: bool,
}

impl<'a> AdmissibleDocument<'a> {
    /// Mira las tres marcas. **No abre nada y no descifra nada.**
    pub fn check(pdf: &'a [u8]) -> Result<Self, Refusal> {
        if !has_header(pdf) {
            return Err(Refusal::NotAPdf);
        }
        if is_encrypted(pdf) {
            return Err(Refusal::Encrypted);
        }
        if contains(pdf, DOC_MDP) {
            return Err(Refusal::Certified);
        }
        Ok(Self {
            pdf,
            already_signed: contains(pdf, BYTE_RANGE),
            unregistered_signatures: has_unregistered_signatures(pdf),
        })
    }

    /// Los bytes, ya admitidos.
    pub fn bytes(&self) -> &'a [u8] {
        self.pdf
    }

    /// Si el documento **ya trae alguna firma**, es decir, si esto va a ser una
    /// cofirma.
    ///
    /// No cambia nada de lo que se envía —una cofirma PAdES es una firma más,
    /// con los mismos `extraParams`—: está para poder contarlo, y para que una
    /// prueba pueda afirmar que la segunda firma vio la primera.
    pub fn already_signed(&self) -> bool {
        self.already_signed
    }

    /// Indica si el documento contiene firmas con subfiltros no registrados.
    pub fn has_unregistered_signatures(&self) -> bool {
        self.unregistered_signatures
    }
}

fn has_header(pdf: &[u8]) -> bool {
    let window = &pdf[..pdf.len().min(HEADER_WINDOW + HEADER.len())];
    contains(window, HEADER)
}

/// Busca `/Encrypt` como entrada de un diccionario.
fn is_encrypted(pdf: &[u8]) -> bool {
    let mut from = 0;
    while let Some(offset) = find(&pdf[from..], ENCRYPT) {
        let after = &pdf[from + offset + ENCRYPT.len()..];
        if looks_like_an_entry(after) {
            return true;
        }
        from += offset + ENCRYPT.len();
    }
    false
}

/// Lo que sigue a `/Encrypt` cuando es de verdad la entrada del tráiler.
fn looks_like_an_entry(after: &[u8]) -> bool {
    let rest = after
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .map(|start| &after[start..])
        .unwrap_or_default();
    // `/Encrypt<<…>>`: el diccionario puesto en el sitio.
    if rest.starts_with(b"<<") {
        return true;
    }
    // `/Encrypt 12 0 R`: la referencia indirecta.
    let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    digits > 0 && rest[digits..].first().is_some_and(u8::is_ascii_whitespace)
}

/// Busca un `/SubFilter` cuyo valor no sea uno de [`KNOWN_SUB_FILTERS`].
fn has_unregistered_signatures(pdf: &[u8]) -> bool {
    let mut from = 0;
    while let Some(offset) = find(&pdf[from..], SUB_FILTER) {
        let after = from + offset + SUB_FILTER.len();
        if let Some(name) = name_at(&pdf[after..]) {
            if !KNOWN_SUB_FILTERS.contains(&name) {
                return true;
            }
        }
        from = after;
    }
    false
}

/// El nombre PDF que sigue a una clave, si lo que sigue es un nombre.
///
/// Un nombre acaba en el primer espacio o delimitador (`%PDF-2.0`, 7.3.5); la
/// barra inicial va dentro, que es como los nombra [`KNOWN_SUB_FILTERS`].
fn name_at(after: &[u8]) -> Option<&[u8]> {
    let start = after.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let rest = &after[start..];
    if rest.first() != Some(&b'/') {
        return None;
    }
    let end = rest[1..]
        .iter()
        .position(|byte| byte.is_ascii_whitespace() || is_delimiter(*byte))
        .map_or(rest.len(), |length| length + 1);
    Some(&rest[..end])
}

/// Los delimitadores de la sintaxis del PDF (`%PDF-2.0`, tabla 2).
fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests;
