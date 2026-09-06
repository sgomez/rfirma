//! Lo que la sede pide por el canal ya abierto, leído de la URL.

use base64::Engine as _;

use super::codes::{Parameter, SafCode};
use super::filters::{site_filter, SiteFilter};
use super::parameters::{check_local_access_is_not_requested, check_minimum_client_version};
use super::refusal::{Refusal, RefusalSituation};
use super::url::AfirmaUrl;

/// El verbo de la selección de certificado, tal y como viaja por el cable.
///
/// **No es el nombre que el JS usa por dentro**: allí la constante se llama
/// `OPERATION_SELECT_CERTIFICATE = "certificate"` (`autoscript.js:1761`), que
/// es sólo la etiqueta con la que el cliente recuerda qué respuesta espera. Lo
/// que viaja es esto (`autoscript.js:1943`).
pub const SELECT_CERTIFICATE: &str = "selectcert";

/// El verbo de la firma (`autoscript.js:1828`).
pub const SIGN: &str = "sign";

/// El verbo de la cofirma.
pub const COSIGN: &str = "cosign";

/// El verbo de la contrafirma.
pub const COUNTERSIGN: &str = "countersign";

/// El verbo que guarda un fichero en el equipo.
pub const SAVE: &str = "save";

/// El verbo que firma y además guarda.
pub const SIGN_AND_SAVE: &str = "signandsave";

/// El formato de firma PAdES.
pub const PADES: &str = "pades";

/// El algoritmo que rFirma sabe producir.
pub const ACCEPTED_ALGORITHMS: [&str; 2] = ["sha256", "sha256withrsa"];

/// Lo que la sede pide, ya leído.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteOperation {
    /// `selectcert`: la sede pide identidad.
    SelectCertificate(SelectCertificate),
    /// `sign` o `cosign` sobre un PDF: la sede pide una firma.
    Sign(SignRequest),
}

/// Cuál de las dos firmas pidió la sede.
///
/// En PAdES las dos recorren el mismo camino —cofirmar es volver a firmar—, y
/// la distinción se guarda porque es lo que la sede pidió y lo que la ventana
/// tiene que contarle a la persona antes de que consienta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureRound {
    /// `sign`: se firma lo que llega.
    First,
    /// `cosign`: se vuelve a firmar sobre las firmas que el PDF ya trae.
    Again,
}

/// La petición de `sign` o de `cosign`.
///
/// Lleva **el documento ya descodificado**, y no el Base64: lo que se firma son
/// bytes, y dejar el Base64 vivo hasta el momento de firmar es tener dos
/// copias de lo mismo y una ocasión de descodificarlo dos veces distintas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRequest {
    round: SignatureRound,
    algorithm: String,
    document: Vec<u8>,
    declared: Vec<(String, String)>,
    filter: SiteFilter,
}

impl SignRequest {
    /// `sign` o `cosign`.
    pub fn round(&self) -> SignatureRound {
        self.round
    }

    /// El algoritmo tal y como lo pidió la sede, ya admitido.
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// El documento que la sede manda, en bytes.
    pub fn document(&self) -> &[u8] {
        &self.document
    }

    /// Los `extraParams` tal y como vinieron, sin expandir.
    pub fn declared_params(&self) -> &[(String, String)] {
        &self.declared
    }

    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }
}

/// La petición de `selectcert`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectCertificate {
    filter: SiteFilter,
}

impl SelectCertificate {
    /// Lo que la sede pide del listado.
    pub fn filter(&self) -> &SiteFilter {
        &self.filter
    }
}

/// Lee la operación que llegó por el canal, o por qué se rechaza.
pub fn read_operation(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    check_minimum_client_version(url.parameter("mcv"))?;
    if let Some(data) = url.parameter("dat") {
        check_local_access_is_not_requested(data)?;
    }

    match verb_of(url).as_str() {
        SELECT_CERTIFICATE => Ok(SiteOperation::SelectCertificate(SelectCertificate {
            filter: site_filter(&declared_properties(url)?)?,
        })),
        SIGN => sign_request(url, SignatureRound::First),
        COSIGN => sign_request(url, SignatureRound::Again),
        COUNTERSIGN => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "'countersign' no existe en PAdES: AOPDFSigner.countersign lanza una \
             UnsupportedOperationException",
        )),
        SAVE | SIGN_AND_SAVE => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            "rFirma no guarda ficheros por orden de una sede",
        )),
        other => Err(Refusal::new(
            SafCode::UnsupportedOperation,
            format!("la operacion '{other}' no se atiende"),
        )),
    }
}

/// La petición de firma, con las cuatro comprobaciones de
/// `UrlParametersToSign` que rFirma hereda.
///
/// El orden importa poco salvo en una cosa: el **formato** se mira antes que
/// nada de lo demás, porque una sede que pide XAdES no se merece un `SAF_03`
/// sobre el algoritmo cuando lo que pasa es que ese formato no se atiende.
fn sign_request(url: &AfirmaUrl, round: SignatureRound) -> Result<SiteOperation, Refusal> {
    let format = required(url, "format", Parameter::Format)
        .map_err(|refusal| refusal.because(RefusalSituation::MissingFormat))?;
    if !format.trim().eq_ignore_ascii_case(PADES) {
        return Err(Refusal::new(
            SafCode::UnsupportedFormat,
            format!("el formato '{format}' no se atiende: rFirma solo firma PAdES"),
        ));
    }

    let algorithm = required(url, "algorithm", Parameter::Algorithm)?;
    if !ACCEPTED_ALGORITHMS.contains(&algorithm.trim().to_ascii_lowercase().as_str()) {
        return Err(Refusal::about(
            Parameter::Algorithm,
            format!("el algoritmo '{algorithm}' no se atiende: rFirma firma con SHA256withRSA"),
        ));
    }

    let data = required(url, "dat", Parameter::Data)?;
    let document = decode_base64(data, Parameter::Data)?;
    if document.is_empty() {
        return Err(Refusal::new(
            SafCode::SignWithoutData,
            "el parametro 'dat' viene vacio: no hay nada que firmar",
        ));
    }

    let declared = declared_properties(url)?;
    Ok(SiteOperation::Sign(SignRequest {
        round,
        algorithm: algorithm.trim().to_owned(),
        document,
        filter: site_filter(&declared)?,
        declared,
    }))
}

/// Un parámetro que la operación exige, o el `SAF_03` que lo nombra.
fn required<'u>(url: &'u AfirmaUrl, name: &str, blame: Parameter) -> Result<&'u str, Refusal> {
    url.parameter(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Refusal::about(blame, format!("falta el parametro '{name}'")))
}

/// El verbo que pide la sede: el parámetro `op` si viene, y si no, el dominio
/// de la URL.
fn verb_of(url: &AfirmaUrl) -> String {
    url.parameter("op")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| url.verb())
        .trim()
        .to_ascii_lowercase()
}

/// Los pares del `.properties` que la sede mandó dentro de `properties`.
///
/// Viaja en Base64 **URL-safe** (`Base64.encode(bytes, true)` del original), y
/// el descodificador es tolerante a propósito con lo que sí puede llegar: la
/// `/` del alfabeto normal y el relleno ausente. Un cliente que mande `/` no
/// está atacando nada, y rechazarle la llamada entera por eso sería inventarse
/// una incompatibilidad que el original no tiene.
///
/// El `+` del alfabeto normal, en cambio, **nunca llega hasta aquí**:
/// [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el original pasa
/// cada valor por `URLDecoder` (ver el encabezado de [`crate::protocol::url`]).
/// Así que una sede que mande Base64 estándar con `+` se lleva el `SAF_03` —
/// igual que en el original, que decodifica igual—, y aquí no hay ningún brazo
/// que lo intente: sería código muerto que promete una tolerancia que no
/// existe.
fn declared_properties(url: &AfirmaUrl) -> Result<Vec<(String, String)>, Refusal> {
    let Some(encoded) = url.parameter("properties").filter(|it| !it.is_empty()) else {
        return Ok(Vec::new());
    };

    let decoded = decode_base64(encoded, Parameter::Properties)?;

    let text = String::from_utf8(decoded).map_err(|error| {
        Refusal::about(
            Parameter::Properties,
            format!("el parametro 'properties' no es texto: {error}"),
        )
    })?;

    Ok(pairs_of(&text))
}

/// El Base64 **URL-safe** del protocolo, con la misma tolerancia en todos los
/// parámetros que lo llevan.
///
/// Tolerante a propósito con lo que sí puede llegar: la `/` del alfabeto normal
/// y el relleno ausente o de más. El `+` del alfabeto normal, en cambio, nunca
/// llega hasta aquí: [`AfirmaUrl`] ya lo ha convertido en un espacio, porque el
/// original pasa cada valor por `URLDecoder`.
fn decode_base64(encoded: &str, blame: Parameter) -> Result<Vec<u8>, Refusal> {
    let normalized: String = encoded
        .chars()
        .filter(|character| *character != '=')
        .map(|character| if character == '/' { '_' } else { character })
        .collect();

    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(normalized.as_bytes())
        .map_err(|error| {
            Refusal::about(
                blame,
                format!("el parametro '{blame}' no es Base64: {error}"),
            )
        })
}

/// Los pares de un bloque `java.util.Properties`.
pub fn pairs_of(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(at) = separator_in(line) else {
            continue;
        };
        let key = unescape(line[..at].trim_end());
        let value = unescape(line[at + 1..].trim_start());
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }

    pairs
}

/// Dónde parte la línea: el primer `=` o `:` que no venga escapado.
fn separator_in(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'=' | b':' => return Some(index),
            _ => {}
        }
    }
    None
}

/// Deshace las barras de escape: las tres que escribe el proyecto —`\\`, `\n`,
/// `\r`— más `\t`, y cualquier otra barra que se queda con lo que lleve detrás.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => break,
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: se lee una cadena y sale una petición. No hay socket, ni
    /// token, ni puente.
    fn an_operation(parameters: &str) -> AfirmaUrl {
        AfirmaUrl::parse(&format!("afirma://selectcert?{parameters}")).expect("es del protocolo")
    }

    fn properties(text: &str) -> String {
        base64::engine::general_purpose::URL_SAFE.encode(text.as_bytes())
    }

    #[test]
    fn the_verb_of_the_published_client_is_the_selection_of_a_certificate() {
        let operation = read_operation(&an_operation(
            "op=selectcert&idsession=8jAkPZfRw2mQxN4TbYuL",
        ))
        .expect("es una operacion que se atiende");

        let SiteOperation::SelectCertificate(request) = operation else {
            panic!("el verbo del cliente publicado es la seleccion de certificado");
        };
        assert!(
            request.filter().declares_nothing(),
            "sin 'properties' no hay filtro declarado"
        );
    }

    /// El verbo va dos veces en la URL, y manda el parámetro.
    #[test]
    fn the_parameter_wins_over_the_domain_of_the_url() {
        let url = AfirmaUrl::parse("afirma://sign?op=selectcert").expect("es del protocolo");

        read_operation(&url).expect("el 'op' es el que manda");
    }

    /// Y sin parámetro, el dominio basta.
    #[test]
    fn without_the_parameter_the_domain_of_the_url_is_the_verb() {
        read_operation(&an_operation("idsession=8jAkPZfRw2mQxN4TbYuL")).expect("el dominio vale");
    }

    #[test]
    fn an_operation_that_is_not_attended_is_refused_with_the_code_of_the_original() {
        let url = AfirmaUrl::parse("afirma://batch?op=batch").expect("es del protocolo");

        let refusal = read_operation(&url).expect_err("no se atiende");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
    }

    fn dat(bytes: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE.encode(bytes)
    }

    fn a_signature(verb: &str, extra: &str) -> AfirmaUrl {
        an_operation(&format!(
            "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={}{extra}",
            dat(b"%PDF-1.7\n")
        ))
    }

    #[test]
    fn a_signature_carries_its_format_its_algorithm_and_the_document() {
        let operation = read_operation(&a_signature(SIGN, "")).expect("se atiende");

        let SiteOperation::Sign(request) = operation else {
            panic!("es una firma");
        };
        assert_eq!(request.round(), SignatureRound::First);
        assert_eq!(request.algorithm(), "SHA256withRSA");
        assert_eq!(request.document(), b"%PDF-1.7\n");
    }

    #[test]
    fn a_cosignature_is_the_same_request_with_another_round() {
        let operation = read_operation(&a_signature(COSIGN, "")).expect("se atiende");

        let SiteOperation::Sign(request) = operation else {
            panic!("es una firma");
        };
        assert_eq!(request.round(), SignatureRound::Again);
    }

    #[test]
    fn a_countersignature_in_pades_is_refused_with_the_code_of_the_original() {
        let refusal = read_operation(&a_signature(COUNTERSIGN, "")).expect_err("no existe");

        assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
        assert!(refusal.detail().contains("countersign"));
    }

    #[test]
    fn saving_files_by_order_of_a_site_is_refused_on_purpose() {
        for verb in [SAVE, SIGN_AND_SAVE] {
            let refusal = read_operation(&a_signature(verb, "")).expect_err("esta fuera");

            assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
            assert!(
                refusal.detail().contains("no guarda ficheros"),
                "«{verb}» se rechaza por lo que es, no por desconocido: {}",
                refusal.detail()
            );
        }
    }

    #[test]
    fn a_format_that_is_not_pades_is_refused_as_an_unsupported_format() {
        let url = an_operation(&format!(
            "op=sign&format=XAdES&algorithm=SHA256withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        let refusal = read_operation(&url).expect_err("solo PAdES");

        assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
    }

    #[test]
    fn the_format_is_looked_at_before_anything_else_of_the_signature() {
        let url = an_operation("op=sign&format=CAdES&algorithm=loquesea");

        let refusal = read_operation(&url).expect_err("solo PAdES");

        assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
    }

    #[test]
    fn an_algorithm_rfirma_cannot_produce_names_its_parameter() {
        let url = an_operation(&format!(
            "op=sign&format=PAdES&algorithm=SHA512withRSA&dat={}",
            dat(b"%PDF-1.7\n")
        ));

        let refusal = read_operation(&url).expect_err("solo SHA256withRSA");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
    }

    #[test]
    fn each_missing_parameter_of_a_signature_names_itself() {
        for (parameters, blamed) in [
            ("op=sign", Parameter::Format),
            ("op=sign&format=PAdES", Parameter::Algorithm),
            ("op=sign&format=PAdES&algorithm=SHA256", Parameter::Data),
        ] {
            let refusal = read_operation(&an_operation(parameters)).expect_err("falta uno");

            assert_eq!(refusal.code(), SafCode::Params);
            assert_eq!(refusal.blame(), Some(blamed), "en «{parameters}»");
        }
    }

    #[test]
    fn a_document_that_is_not_base64_names_the_parameter_that_came_wrong() {
        let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=%%%");

        let refusal = read_operation(&url).expect_err("no es Base64");

        assert_eq!(refusal.blame(), Some(Parameter::Data));
    }

    #[test]
    fn a_signature_that_asks_for_a_local_file_never_gets_read() {
        let url = an_operation("op=sign&dat=file:/etc/passwd");

        let refusal = read_operation(&url).expect_err("no se leen ficheros locales");

        assert_eq!(refusal.blame(), Some(Parameter::Data));
    }

    #[test]
    fn the_extra_params_of_the_site_arrive_whole_and_unexpanded() {
        let url = a_signature(
            SIGN,
            &format!(
                "&properties={}",
                properties("expPolicy=FirmaAGE\nfilters=subject.contains:PEREZ\n")
            ),
        );

        let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
            panic!("es una firma");
        };
        assert!(request
            .declared_params()
            .contains(&("expPolicy".to_owned(), "FirmaAGE".to_owned())));
        assert_eq!(
            request.filter().declared(),
            [("filters".to_owned(), "subject.contains:PEREZ".to_owned())],
            "y los filtros salen del mismo bloque, igual que en selectcert"
        );
    }

    #[test]
    fn a_signature_with_nothing_to_sign_says_exactly_that() {
        let url = an_operation("op=sign&format=PAdES&algorithm=SHA256&dat=%3D");

        let refusal = read_operation(&url).expect_err("no hay nada que firmar");

        assert_eq!(refusal.code(), SafCode::SignWithoutData);
    }

    #[test]
    fn the_two_common_guards_are_checked_in_the_selection_of_a_certificate_too() {
        let too_new = read_operation(&an_operation("op=selectcert&mcv=99.9.9"))
            .expect_err("la sede exige una version que no se implementa");
        assert_eq!(too_new.code(), SafCode::MinimumVersionNonSatisfied);

        let local = read_operation(&an_operation("op=selectcert&dat=file:///etc/shadow"))
            .expect_err("pide leer un fichero del equipo");
        assert_eq!(local.code(), SafCode::Params);
        assert_eq!(local.blame(), Some(Parameter::Data));
    }

    #[test]
    fn the_filter_travels_inside_the_properties_and_comes_out_untouched() {
        let url = an_operation(&format!(
            "op=selectcert&properties={}",
            properties("filters=subject.contains:PEREZ\n")
        ));

        let SiteOperation::SelectCertificate(request) =
            read_operation(&url).expect("el criterio esta en la lista blanca")
        else {
            panic!("es una seleccion de certificado");
        };

        assert_eq!(
            request.filter().declared(),
            [("filters".to_owned(), "subject.contains:PEREZ".to_owned())]
        );
    }

    #[test]
    fn a_criterion_outside_the_whitelist_refuses_the_whole_call() {
        let url = an_operation(&format!(
            "op=selectcert&properties={}",
            properties("filters=inventado:loquesea\n")
        ));

        let refusal = read_operation(&url).expect_err("el criterio no esta en la lista blanca");

        assert_eq!(refusal.code(), SafCode::Params);
    }

    #[test]
    fn the_slash_of_the_plain_base64_alphabet_is_accepted_too() {
        let plain =
            base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:OÑ\n");
        assert!(plain.contains('/'), "la carga util trae una barra: {plain}");
        let url = an_operation(&format!("op=selectcert&properties={plain}"));

        read_operation(&url).expect("se lee igual");
    }

    #[test]
    fn a_plus_of_the_plain_base64_alphabet_never_makes_it_this_far() {
        let plain =
            base64::engine::general_purpose::STANDARD.encode("filters=subject.contains:þ\n");
        assert!(plain.contains('+'), "la carga util trae un mas: {plain}");
        let url = an_operation(&format!("op=selectcert&properties={plain}"));

        let refusal = read_operation(&url).expect_err("el mas ya es un espacio");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(Parameter::Properties));
    }

    #[test]
    fn properties_that_are_not_base64_name_the_parameter_that_came_wrong() {
        let url = an_operation("op=selectcert&properties=!!!!");

        let refusal = read_operation(&url).expect_err("no es Base64");

        assert_eq!(refusal.code(), SafCode::Params);
        assert_eq!(refusal.blame(), Some(Parameter::Properties));
    }

    #[test]
    fn the_properties_block_is_read_the_way_the_original_writes_it() {
        let pairs = pairs_of("# un comentario\n\nfilters=subject.rfc2254:\\(cn=X\\)\nkey:valor\n");

        assert_eq!(
            pairs,
            vec![
                ("filters".to_owned(), "subject.rfc2254:(cn=X)".to_owned()),
                ("key".to_owned(), "valor".to_owned()),
            ]
        );
    }

    #[test]
    fn an_escaped_separator_does_not_split_the_line() {
        let pairs = pairs_of("cla\\=ve=valor\n");

        assert_eq!(pairs, vec![("cla=ve".to_owned(), "valor".to_owned())]);
    }
}
