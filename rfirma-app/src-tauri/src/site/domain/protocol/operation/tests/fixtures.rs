use super::super::*;

/// **Grada A**: se lee una cadena y sale una petición. No hay socket, ni
/// token, ni puente.
pub(super) fn an_operation(parameters: &str) -> AfirmaUrl {
    AfirmaUrl::parse(&format!("afirma://selectcert?{parameters}")).expect("es del protocolo")
}

pub(super) fn properties(text: &str) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(text.as_bytes())
}

pub(super) fn dat(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

pub(super) fn a_signature(verb: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"%PDF-1.7\n")
    ))
}

pub(super) fn a_sign_and_save(cop: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={SIGN_AND_SAVE}&cop={cop}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&\
         algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"%PDF-1.7\n")
    ))
}

pub(super) fn a_batch(extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost{extra}"
    ))
}

pub(super) fn xml_lote(algorithm: &str, stop_on_error: bool) -> String {
    format!(
        "<signbatch algorithm=\"{algorithm}\" stoponerror=\"{stop_on_error}\">\
         <singlesign id=\"001\"/></signbatch>"
    )
}

pub(super) fn json_lote(algorithm: &str, stop_on_error: bool) -> String {
    format!("{{\"algorithm\":\"{algorithm}\",\"stoponerror\":{stop_on_error}}}")
}

/// La contrafirma que pide una sede, con el formato y los `extraParams` que se le digan.
pub(super) fn a_countersignature(format: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={COUNTERSIGN}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&\
         algorithm=SHA256withRSA&dat={}{extra}",
        dat(b"una firma")
    ))
}

/// Una factura mínima con la raíz y los tres hijos que el original le exige.
pub(super) const AN_INVOICE: &[u8] = b"<Facturae><FileHeader/><Parties/><Invoices/></Facturae>";

pub(super) fn an_invoice_signature(verb: &str, format: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format={format}&algorithm=SHA256withRSA&dat={}",
        dat(AN_INVOICE)
    ))
}

pub(super) fn gzipped(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("comprime en memoria");
    encoder.finish().expect("termina el gzip")
}

pub(super) fn url_encoded(text: &str) -> String {
    text.bytes().map(|byte| format!("%{byte:02X}")).collect()
}

pub(super) fn the_signed_document(dat: &str, extra: &str) -> Vec<u8> {
    let url = an_operation(&format!(
        "op=sign&idsession=8jAkPZfRw2mQxN4TbYuL&format=CAdES&algorithm=SHA256withRSA&dat={dat}{extra}"
    ));
    let SiteOperation::Sign(request) = read_operation(&url).expect("se atiende") else {
        panic!("es sign");
    };
    request.document().to_vec()
}

/// El origen de datos que nunca baja nada: el `dat` de casi todas estas pruebas viene en la URL.
pub(super) struct NoDownloads;

impl DataSource for NoDownloads {
    fn download(&self, _url: &str) -> Result<Vec<u8>, String> {
        panic!("ninguna prueba de la grada A baja nada de la red")
    }
}

/// Lo que hay en esa URL, comprobando de paso que se pide la que mandó la sede.
pub(super) struct ADownload {
    pub(super) from: &'static str,
    pub(super) content: Vec<u8>,
}

impl DataSource for ADownload {
    fn download(&self, url: &str) -> Result<Vec<u8>, String> {
        assert_eq!(url, self.from, "se baja la URL que mando la sede");
        Ok(self.content.clone())
    }
}

/// La descarga que no llega a ninguna parte.
pub(super) struct ADownloadThatFails;

impl DataSource for ADownloadThatFails {
    fn download(&self, _url: &str) -> Result<Vec<u8>, String> {
        Err("la sede no responde".to_owned())
    }
}

pub(super) fn read_operation(url: &AfirmaUrl) -> Result<SiteOperation, Refusal> {
    super::super::read_operation(url, &NoDownloads)
}

pub(super) fn read_downloading(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<SiteOperation, Refusal> {
    super::super::read_operation(url, data)
}

pub(super) const A_REMOTE_DOCUMENT: &str = "https://sede.example/documentos/4711.pdf";

pub(super) fn a_signature_of_a_url(verb: &str, url_of_the_data: &str, extra: &str) -> AfirmaUrl {
    an_operation(&format!(
        "op={verb}&idsession=8jAkPZfRw2mQxN4TbYuL&format=PAdES&algorithm=SHA256withRSA&dat={url_of_the_data}{extra}"
    ))
}

pub(super) const CHANNEL_SESSION: &str = "Rj5Ct7Pr9Ob1Es3Tt5Ab";

/// La orden tal y como la manda por el canal el guion de conformidad.
pub(super) fn an_order(url: &str) -> AfirmaUrl {
    AfirmaUrl::parse(&format!("{url}&idsession={CHANNEL_SESSION}")).expect("es del protocolo")
}

/// La firma del guion que, pasado el análisis de parámetros, se para en un formato inexistente.
pub(super) fn a_signature_order(parameters: &str) -> AfirmaUrl {
    an_order(&format!(
        "afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&{parameters}"
    ))
}

pub(super) fn code_of_the_order(url: &AfirmaUrl) -> SafCode {
    read_operation(url).expect_err("la orden se rechaza").code()
}

pub(super) fn refusal_of(url: &str) -> Refusal {
    read_operation(&AfirmaUrl::parse(url).expect("es del protocolo"))
        .expect_err("la guarda deberia rechazarlo")
}
