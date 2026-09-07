//! `TriphaseData` calcado del original, y la regla de `PK1` que firma y borra el `PRE` (`TriphaseDataSigner.doSign`, 1.9.2).

use std::fmt;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::json::Json;

/// El nombre del parámetro con la prefirma, tal y como lo llama el original.
const PRE: &str = "PRE";
/// El nombre del parámetro con la firma PKCS#1.
const PK1: &str = "PK1";
/// El nombre del parámetro que indica si la postfirma necesita la prefirma.
const NEED_PRE: &str = "NEED_PRE";

/// Un `TriphaseData` mal formado, o una prefirma que no se pudo aplicar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriphaseDataError(String);

impl fmt::Display for TriphaseDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TriphaseDataError {}

/// Los datos de una firma trifásica individual: sus dos identificadores y sus parámetros, en el
/// orden del documento del que se leyeron (`ConcurrentHashMap` en el original: sin ese orden como
/// contrato, pero sí como algo que un ida y vuelta congelado conserva aquí).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriSign {
    id: Option<String>,
    signature_id: Option<String>,
    params: Vec<(String, String)>,
}

impl TriSign {
    /// Crea una firma trifásica individual con sus parámetros ya leídos.
    pub fn new(
        id: Option<String>,
        signature_id: Option<String>,
        params: Vec<(String, String)>,
    ) -> Self {
        Self {
            id,
            signature_id,
            params,
        }
    }

    /// El identificador individual de la firma (`Id`/`id`).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// El identificador de la firma global (`signid`), cuando varias firmas individuales
    /// pertenecen a la misma.
    pub fn signature_id(&self) -> Option<&str> {
        self.signature_id.as_deref()
    }

    /// Los parámetros, en el orden en que se leyeron.
    pub fn params(&self) -> &[(String, String)] {
        &self.params
    }

    /// El valor de un parámetro, si lo tiene.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Añade un parámetro, o sustituye el valor si ya existía.
    pub fn set_param(&mut self, key: &str, value: String) {
        match self.params.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value,
            None => self.params.push((key.to_owned(), value)),
        }
    }

    /// Elimina un parámetro, si lo tenía.
    pub fn remove_param(&mut self, key: &str) {
        self.params.retain(|(k, _)| k != key);
    }
}

/// Los datos de una sesión de firma trifásica: el formato, si lo hay, y sus firmas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriphaseData {
    format: Option<String>,
    signs: Vec<TriSign>,
}

impl TriphaseData {
    /// Construye una sesión trifásica con sus firmas ya leídas.
    pub fn new(format: Option<String>, signs: Vec<TriSign>) -> Self {
        Self { format, signs }
    }

    /// El formato declarado, si lo hay.
    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    /// Las firmas de la sesión.
    pub fn signs(&self) -> &[TriSign] {
        &self.signs
    }

    /// Lee una sesión trifásica del XML del original (`TriphaseData.parser`, 1.9.2).
    pub fn parse_xml(xml: &[u8]) -> Result<Self, TriphaseDataError> {
        let text = std::str::from_utf8(xml)
            .map_err(|error| TriphaseDataError(format!("el XML no es UTF-8: {error}")))?;

        let mut reader = Reader::from_str(text);
        reader.config_mut().trim_text(true);

        let mut format = None;
        let mut found_firmas = false;
        let mut signs = Vec::new();
        let mut current: Option<TriSign> = None;
        let mut current_param: Option<String> = None;

        loop {
            match reader
                .read_event()
                .map_err(|error| TriphaseDataError(error.to_string()))?
            {
                Event::Start(tag) => match tag.local_name().as_ref() {
                    b"firmas" => {
                        found_firmas = true;
                        format = attribute_of(&tag, b"format");
                    }
                    b"firma" => {
                        current = Some(TriSign::new(
                            attribute_of(&tag, b"Id"),
                            attribute_of(&tag, b"signid"),
                            Vec::new(),
                        ));
                    }
                    b"param" => {
                        current_param = attribute_of(&tag, b"n");
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    if let (Some(sign), Some(name)) = (current.as_mut(), current_param.as_deref()) {
                        let value = text
                            .decode()
                            .map_err(|error| TriphaseDataError(error.to_string()))?
                            .trim()
                            .to_owned();
                        sign.set_param(name, value);
                    }
                }
                Event::End(tag) => match tag.local_name().as_ref() {
                    b"firma" => {
                        if let Some(sign) = current.take() {
                            signs.push(sign);
                        }
                    }
                    b"param" => current_param = None,
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }

        if !found_firmas {
            return Err(TriphaseDataError(
                "no se encontro el nodo 'firmas' en el XML proporcionado".to_owned(),
            ));
        }

        Ok(Self { format, signs })
    }

    /// Genera el XML del original, literal (`TriphaseData.toString()`, 1.9.2).
    pub fn to_xml(&self) -> String {
        let mut out = String::from("<xml>\n <firmas");
        if let Some(format) = &self.format {
            out.push_str(" format=\"");
            out.push_str(format);
            out.push('"');
        }
        out.push_str(">\n");
        for sign in &self.signs {
            out.push_str("  <firma");
            if let Some(id) = &sign.id {
                out.push_str(" Id=\"");
                out.push_str(id);
                out.push('"');
            }
            if let Some(signature_id) = &sign.signature_id {
                out.push_str(" signid=\"");
                out.push_str(signature_id);
                out.push('"');
            }
            out.push_str(">\n");
            for (key, value) in &sign.params {
                out.push_str("   <param n=\"");
                out.push_str(key);
                out.push_str("\">");
                out.push_str(value);
                out.push_str("</param>\n");
            }
            out.push_str("  </firma>\n");
        }
        out.push_str(" </firmas>\n</xml>");
        out
    }

    /// Lee una sesión trifásica del JSON, en cualquiera de sus dos formas: `signs[].signinfo[]`
    /// anidada, o `signinfo[]` en la raíz (`TriphaseDataParser.parseFromJSON`, 1.9.2).
    pub fn parse_json(json: &[u8]) -> Result<Self, TriphaseDataError> {
        let text = std::str::from_utf8(json)
            .map_err(|error| TriphaseDataError(format!("el JSON no es UTF-8: {error}")))?;
        let value = Json::parse(text)
            .map_err(|error| TriphaseDataError(format!("el JSON no es valido: {error}")))?;

        let format = value
            .get("format")
            .and_then(Json::as_str)
            .map(str::to_owned);

        let signinfo_arrays: Vec<&[Json]> =
            if let Some(signs) = value.get("signs").and_then(Json::as_array) {
                signs
                    .iter()
                    .map(|entry| {
                        entry
                            .get("signinfo")
                            .and_then(Json::as_array)
                            .ok_or_else(|| {
                                TriphaseDataError("falta 'signinfo' dentro de 'signs'".to_owned())
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                let root = value
                    .get("signinfo")
                    .and_then(Json::as_array)
                    .ok_or_else(|| {
                        TriphaseDataError(
                            "no se encontro 'signinfo' en el JSON proporcionado".to_owned(),
                        )
                    })?;
                vec![root]
            };

        let mut signs = Vec::new();
        for array in signinfo_arrays {
            for entry in array {
                let id = entry.get("id").and_then(Json::as_str).map(str::to_owned);
                let signature_id = entry
                    .get("signid")
                    .and_then(Json::as_str)
                    .map(str::to_owned);
                let params = entry
                    .get("params")
                    .and_then(Json::as_object)
                    .ok_or_else(|| TriphaseDataError("falta 'params' en 'signinfo'".to_owned()))?
                    .iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_owned()))
                    })
                    .collect();
                signs.push(TriSign::new(id, signature_id, params));
            }
        }

        Ok(Self { format, signs })
    }

    /// Genera el JSON del original: `{"format":…,"signinfo":[{"id","signid","params"}]}`
    /// (`TriphaseDataParser.triphaseDataToJsonString`, 1.9.2).
    pub fn to_json(&self) -> String {
        let mut signinfo = Vec::with_capacity(self.signs.len());
        for sign in &self.signs {
            let mut object = Vec::new();
            if let Some(id) = &sign.id {
                object.push(("id".to_owned(), Json::String(id.clone())));
            }
            if let Some(signature_id) = &sign.signature_id {
                object.push(("signid".to_owned(), Json::String(signature_id.clone())));
            }
            let params = sign
                .params
                .iter()
                .map(|(key, value)| (key.clone(), Json::String(value.clone())))
                .collect();
            object.push(("params".to_owned(), Json::Object(params)));
            signinfo.push(Json::Object(object));
        }

        let mut root = Vec::new();
        if let Some(format) = &self.format {
            root.push(("format".to_owned(), Json::String(format.clone())));
        }
        root.push(("signinfo".to_owned(), Json::Array(signinfo)));

        Json::Object(root).to_json_string()
    }
}

fn attribute_of(tag: &BytesStart, name: &[u8]) -> Option<String> {
    tag.attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == name)
        .map(|attribute| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
}

/// Aplica la regla de `TriphaseDataSigner.doSign`: por cada firma, `PK1` es el cierre aplicado al
/// `PRE` descodificado, y el propio `PRE` se borra salvo que `NEED_PRE=true`.
///
/// La clave privada no aparece aquí: el cierre la encapsula quien lo construye (ADR-0001).
pub fn apply_pk1(
    mut data: TriphaseData,
    mut sign: impl FnMut(&[u8]) -> Vec<u8>,
) -> Result<TriphaseData, TriphaseDataError> {
    for triphase_sign in data.signs.iter_mut() {
        let pre_base64 = triphase_sign.param(PRE).map(str::to_owned).ok_or_else(|| {
            TriphaseDataError(format!(
                "el servidor no ha devuelto la prefirma de '{}'",
                triphase_sign.id().unwrap_or("?")
            ))
        })?;
        let pre = STANDARD
            .decode(pre_base64.as_bytes())
            .map_err(|error| TriphaseDataError(format!("PRE no es Base64: {error}")))?;

        let pk1 = sign(&pre);
        triphase_sign.set_param(PK1, STANDARD.encode(pk1));

        let needs_pre = triphase_sign
            .param(NEED_PRE)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        if !needs_pre {
            triphase_sign.remove_param(PRE);
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests;
