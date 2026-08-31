//! La configuración de firma: cinco ajustes y ni uno más (ID-18).
//!
//! Todo lo demás se hereda de AutoFirma sin enviarse. Quedan fuera a propósito
//! (ID-20) la política de firma, la ciudad, el contacto, el perfil,
//! `signReservedSize` —los 27 000 por omisión dan un `/Contents` de 54 002, que
//! sobra— y las dos llaves de la trifásica `doNotUseCertChainOnPostSign` e
//! `includeOnlySignningCertificate`, que solo tienen sentido cuando la
//! postfirma corre en otra máquina y aquí corre en la del usuario.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Subfiltro de la firma. Se envía **explícito** aunque parezca redundante: el
/// javadoc de `PdfExtraParams.SIGNATURE_SUBFILTER` **miente**, dice que por
/// omisión es `adbe.pkcs7.detached` y el código cae en `ETSI.CAdES.detached`.
/// Fiarse del javadoc es firmar con un subfiltro distinto del que se cree.
pub const SUB_FILTER: &str = "ETSI.CAdES.detached";

const SUB_FILTER_KEY: &str = "signatureSubFilter";
const PAGE_KEY: &str = "signaturePage";
const LOWER_LEFT_X_KEY: &str = "signaturePositionOnPageLowerLeftX";
const LOWER_LEFT_Y_KEY: &str = "signaturePositionOnPageLowerLeftY";
const UPPER_RIGHT_X_KEY: &str = "signaturePositionOnPageUpperRightX";
const UPPER_RIGHT_Y_KEY: &str = "signaturePositionOnPageUpperRightY";
const LAYER2_TEXT_KEY: &str = "layer2Text";
const RUBRIC_IMAGE_KEY: &str = "signatureRubricImage";
const SIGN_REASON_KEY: &str = "signReason";

/// Los cinco ajustes de la configuración de firma. La lista es cerrada: si
/// alguien quiere un sexto, tiene que añadir una variante aquí y decir qué
/// clave emite, y eso ya no se cuela en una revisión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    /// El subfiltro, siempre [`SUB_FILTER`].
    SubFilter,
    /// La geometría del recuadro: página y las cuatro esquinas.
    Geometry,
    /// El texto del recuadro, ya compuesto por rFirma.
    Layer2Text,
    /// La rúbrica, si la hay.
    RubricImage,
    /// El motivo de la firma, si lo hay.
    SignReason,
}

impl Setting {
    /// Los cinco.
    pub const ALL: [Self; 5] = [
        Self::SubFilter,
        Self::Geometry,
        Self::Layer2Text,
        Self::RubricImage,
        Self::SignReason,
    ];

    /// Las claves de `extraParams` que emite este ajuste.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::SubFilter => &[SUB_FILTER_KEY],
            Self::Geometry => &[
                PAGE_KEY,
                LOWER_LEFT_X_KEY,
                LOWER_LEFT_Y_KEY,
                UPPER_RIGHT_X_KEY,
                UPPER_RIGHT_Y_KEY,
            ],
            Self::Layer2Text => &[LAYER2_TEXT_KEY],
            Self::RubricImage => &[RUBRIC_IMAGE_KEY],
            Self::SignReason => &[SIGN_REASON_KEY],
        }
    }
}

/// El recuadro de la firma visible, ya en puntos PAdES.
///
/// Los valores son los que espera `setVisibleSignature`, no el `/Rect` que
/// acabará teniendo el widget: la conversión desde el recuadro que el usuario
/// arrastra en el visor —incluida la inversa de la rotación de la página— la
/// hace [`super::placement::Page::place`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureBox {
    /// Página, **1-based**, tal cual la numera `pdf.js`.
    pub page: u32,
    /// Esquina inferior izquierda, eje X.
    pub lower_left_x: i32,
    /// Esquina inferior izquierda, eje Y.
    pub lower_left_y: i32,
    /// Esquina superior derecha, eje X.
    pub upper_right_x: i32,
    /// Esquina superior derecha, eje Y.
    pub upper_right_y: i32,
}

impl SignatureBox {
    fn extra_params(&self) -> Vec<(String, String)> {
        let Self {
            page,
            lower_left_x,
            lower_left_y,
            upper_right_x,
            upper_right_y,
        } = self;
        vec![
            (PAGE_KEY.to_owned(), page.to_string()),
            (LOWER_LEFT_X_KEY.to_owned(), lower_left_x.to_string()),
            (LOWER_LEFT_Y_KEY.to_owned(), lower_left_y.to_string()),
            (UPPER_RIGHT_X_KEY.to_owned(), upper_right_x.to_string()),
            (UPPER_RIGHT_Y_KEY.to_owned(), upper_right_y.to_string()),
        ]
    }
}

/// Lo que distingue una firma de otra a igualdad de documento y certificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureConfig {
    /// Dónde cae el recuadro.
    pub signature_box: SignatureBox,
    /// El texto del recuadro, compuesto por
    /// [`super::layer2_text::compose_layer2_text`]. Puede estar vacío.
    pub layer2_text: String,
    /// La rúbrica en JPEG opaco y sin perfil ICC, en base64. `None` si no la
    /// hay.
    pub rubric_image: Option<String>,
    /// El motivo de la firma. `None` si no lo hay.
    pub sign_reason: Option<String>,
}

impl SignatureConfig {
    /// Los `extraParams` que rFirma envía al puente.
    ///
    /// `layer2Text` se emite **siempre**, aunque esté vacío: si la clave falta
    /// y tampoco hay `signatureRubricImage`, `PdfSessionManager` inyecta su
    /// texto por omisión, en castellano fijo y con comodines dentro.
    pub fn extra_params(&self) -> BTreeMap<String, String> {
        // Destructurado exhaustivo A PROPÓSITO: un sexto ajuste no compila
        // hasta que alguien lo declare también en `Setting`.
        let Self {
            signature_box,
            layer2_text,
            rubric_image,
            sign_reason,
        } = self;

        let mut params = BTreeMap::new();
        params.insert(SUB_FILTER_KEY.to_owned(), SUB_FILTER.to_owned());
        params.extend(signature_box.extra_params());
        params.insert(LAYER2_TEXT_KEY.to_owned(), layer2_text.clone());
        if let Some(image) = rubric_image {
            params.insert(RUBRIC_IMAGE_KEY.to_owned(), image.clone());
        }
        if let Some(reason) = sign_reason {
            params.insert(SIGN_REASON_KEY.to_owned(), reason.clone());
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::{Setting, SignatureBox, SignatureConfig, SUB_FILTER};
    use std::collections::HashSet;

    fn a_box() -> SignatureBox {
        SignatureBox {
            page: 3,
            lower_left_x: 100,
            lower_left_y: 200,
            upper_right_x: 300,
            upper_right_y: 260,
        }
    }

    fn minimal() -> SignatureConfig {
        SignatureConfig {
            signature_box: a_box(),
            layer2_text: "Firmado por: Ada Lovelace Byron".to_owned(),
            rubric_image: None,
            sign_reason: None,
        }
    }

    fn complete() -> SignatureConfig {
        SignatureConfig {
            rubric_image: Some("/9j/4AAQSkZJRg==".to_owned()),
            sign_reason: Some("Conforme".to_owned()),
            ..minimal()
        }
    }

    #[test]
    fn closes_the_configuration_at_five_settings() {
        assert_eq!(Setting::ALL.len(), 5);
    }

    #[test]
    fn emits_no_key_outside_the_five_settings() {
        let owned: HashSet<&str> = Setting::ALL
            .iter()
            .flat_map(|setting| setting.keys().iter().copied())
            .collect();

        for config in [minimal(), complete()] {
            for key in config.extra_params().keys() {
                assert!(
                    owned.contains(key.as_str()),
                    "«{key}» no pertenece a ninguno de los cinco ajustes"
                );
            }
        }
    }

    #[test]
    fn emits_every_key_the_five_settings_declare() {
        // La dirección contraria a la de arriba: una clave declarada en
        // `Setting::keys()` que `extra_params` no llegue a emitir nunca sería
        // una promesa muerta. `complete()` tiene los cinco ajustes puestos, así
        // que sobre ella la contención va en los dos sentidos.
        let declared: HashSet<&str> = Setting::ALL
            .iter()
            .flat_map(|setting| setting.keys().iter().copied())
            .collect();
        let emitted = complete().extra_params();

        for key in declared {
            assert!(
                emitted.contains_key(key),
                "«{key}» lo declara un ajuste y no lo emite nadie"
            );
        }
    }

    #[test]
    fn gives_every_setting_its_own_keys() {
        let mut seen: HashSet<&str> = HashSet::new();
        for setting in Setting::ALL {
            for key in setting.keys() {
                assert!(seen.insert(key), "«{key}» lo emiten dos ajustes");
            }
        }
    }

    #[test]
    fn sends_the_sub_filter_explicitly() {
        assert_eq!(
            minimal().extra_params().get("signatureSubFilter"),
            Some(&SUB_FILTER.to_owned())
        );
    }

    #[test]
    fn sends_the_geometry_of_the_box() {
        let params = minimal().extra_params();
        assert_eq!(params.get("signaturePage"), Some(&"3".to_owned()));
        assert_eq!(
            params.get("signaturePositionOnPageLowerLeftX"),
            Some(&"100".to_owned())
        );
        assert_eq!(
            params.get("signaturePositionOnPageLowerLeftY"),
            Some(&"200".to_owned())
        );
        assert_eq!(
            params.get("signaturePositionOnPageUpperRightX"),
            Some(&"300".to_owned())
        );
        assert_eq!(
            params.get("signaturePositionOnPageUpperRightY"),
            Some(&"260".to_owned())
        );
    }

    #[test]
    fn always_sends_the_layer2_text_even_when_it_is_empty() {
        let config = SignatureConfig {
            layer2_text: String::new(),
            ..minimal()
        };
        assert_eq!(
            config.extra_params().get("layer2Text"),
            Some(&String::new())
        );
    }

    #[test]
    fn omits_the_rubric_and_the_reason_when_there_are_none() {
        let params = minimal().extra_params();
        assert!(!params.contains_key("signatureRubricImage"));
        assert!(!params.contains_key("signReason"));
    }

    #[test]
    fn sends_the_rubric_and_the_reason_when_there_are() {
        let params = complete().extra_params();
        assert_eq!(
            params.get("signatureRubricImage"),
            Some(&"/9j/4AAQSkZJRg==".to_owned())
        );
        assert_eq!(params.get("signReason"), Some(&"Conforme".to_owned()));
    }

    #[test]
    fn never_sends_what_the_spec_ruled_out() {
        // ID-20. `includeOnlySignningCertificate` va con la errata de
        // AutoFirma: es el nombre real de la clave.
        let ruled_out = [
            "signReservedSize",
            "policyIdentifier",
            "policyIdentifierHash",
            "signatureProductionCity",
            "signerContact",
            "profile",
            "doNotUseCertChainOnPostSign",
            "includeOnlySignningCertificate",
        ];
        let params = complete().extra_params();
        for key in ruled_out {
            assert!(!params.contains_key(key), "«{key}» no debería enviarse");
        }
    }
}
