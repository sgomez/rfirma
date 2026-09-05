//! La configuración de firma: seis ajustes y ni uno más (ID-18).
//!
//! Todo lo demás se hereda de AutoFirma sin enviarse. Quedan fuera a propósito
//! (ID-20) la política de firma, la ciudad, el contacto, el perfil,
//! `signReservedSize` —los 27 000 por omisión dan un `/Contents` de 54 002, que
//! sobra— y las dos llaves de la trifásica `doNotUseCertChainOnPostSign` e
//! `includeOnlySignningCertificate`, que solo tienen sentido cuando la
//! postfirma corre en otra máquina y aquí corre en la del usuario.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::placement::PageSet;

/// Subfiltro de la firma. Se envía **explícito** aunque parezca redundante: el
/// javadoc de `PdfExtraParams.SIGNATURE_SUBFILTER` **miente**, dice que por
/// omisión es `adbe.pkcs7.detached` y el código cae en `ETSI.CAdES.detached`.
/// Fiarse del javadoc es firmar con un subfiltro distinto del que se cree.
pub const SUB_FILTER: &str = "ETSI.CAdES.detached";

const SUB_FILTER_KEY: &str = "signatureSubFilter";
const PAGES_KEY: &str = "signaturePages";
const LOWER_LEFT_X_KEY: &str = "signaturePositionOnPageLowerLeftX";
const LOWER_LEFT_Y_KEY: &str = "signaturePositionOnPageLowerLeftY";
const UPPER_RIGHT_X_KEY: &str = "signaturePositionOnPageUpperRightX";
const UPPER_RIGHT_Y_KEY: &str = "signaturePositionOnPageUpperRightY";
const LAYER2_TEXT_KEY: &str = "layer2Text";
const RUBRIC_IMAGE_KEY: &str = "signatureRubricImage";
const SIGN_REASON_KEY: &str = "signReason";
const LAYER2_FONT_SIZE_KEY: &str = "layer2FontSize";

/// El tamaño de letra del recuadro, **cero**, que no es un tamaño: es lo que
/// hace que el compositor reparta `alto del recuadro / número de líneas` y
/// ajuste la letra desde ahí. Sin la clave, `PdfSessionManager:261-272` cae en
/// su valor por omisión —12 pt— y ese funciona como **tope**: solo encoge,
/// nunca crece, así que un recuadro de media página salía con letra minúscula.
/// **Sin tope superior**: si alguien traza un recuadro grande es porque quiere
/// verlo, y lo ve antes de firmar.
const LAYER2_FONT_SIZE: &str = "0";

/// Los seis ajustes de la configuración de firma. La lista es cerrada: si
/// alguien quiere un séptimo, tiene que añadir una variante aquí y decir qué
/// clave emite, y eso ya no se cuela en una revisión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Setting {
    /// El subfiltro, siempre [`SUB_FILTER`].
    SubFilter,
    /// La geometría del recuadro: las páginas y las cuatro esquinas. **No
    /// emite nada cuando la firma no lleva colocación nuestra**, que es como
    /// los `signaturePositionOnPage*` de una sede llegan crudos al puente
    /// (ID-282).
    Geometry,
    /// El texto del recuadro, ya compuesto por rFirma.
    Layer2Text,
    /// La rúbrica, si la hay.
    RubricImage,
    /// El motivo de la firma, si lo hay.
    SignReason,
    /// El tamaño de letra del recuadro, siempre [`LAYER2_FONT_SIZE`].
    Layer2FontSize,
}

impl Setting {
    /// Los seis.
    pub const ALL: [Self; 6] = [
        Self::SubFilter,
        Self::Geometry,
        Self::Layer2Text,
        Self::RubricImage,
        Self::SignReason,
        Self::Layer2FontSize,
    ];

    /// Las claves de `extraParams` que emite este ajuste.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::SubFilter => &[SUB_FILTER_KEY],
            Self::Geometry => &[
                PAGES_KEY,
                LOWER_LEFT_X_KEY,
                LOWER_LEFT_Y_KEY,
                UPPER_RIGHT_X_KEY,
                UPPER_RIGHT_Y_KEY,
            ],
            Self::Layer2Text => &[LAYER2_TEXT_KEY],
            Self::RubricImage => &[RUBRIC_IMAGE_KEY],
            Self::SignReason => &[SIGN_REASON_KEY],
            Self::Layer2FontSize => &[LAYER2_FONT_SIZE_KEY],
        }
    }
}

/// El rectángulo de la firma visible, ya en puntos PAdES.
///
/// Los valores son los que espera `setVisibleSignature`, no el `/Rect` que
/// acabará teniendo el widget: la conversión desde el recuadro que el usuario
/// arrastra en el visor —incluida la inversa de la rotación de la página— la
/// hace [`super::placement::Page::place`].
///
/// **No lleva página**: en qué páginas se estampa es [`PageSet`], y las dos
/// mitades juntas son [`Placement`]. Con el widget replicado el rectángulo es
/// forzosamente el mismo en todas (ID-96), así que hay un rectángulo y un
/// conjunto, no un rectángulo por página.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadesRect {
    /// Esquina inferior izquierda, eje X.
    pub lower_left_x: i32,
    /// Esquina inferior izquierda, eje Y.
    pub lower_left_y: i32,
    /// Esquina superior derecha, eje X.
    pub upper_right_x: i32,
    /// Esquina superior derecha, eje Y.
    pub upper_right_y: i32,
}

/// **La colocación**: dónde cae el recuadro y en qué páginas (ID-90).
///
/// Es un registro llano y **no una unión de un brazo**: un `kind` que nunca
/// discrimina es ruido, y no ahorraría la migración del día que entre otra
/// rama de colocación, porque en esa rama `rect` y `pages` tienen que
/// desaparecer, no convivir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// El rectángulo, el mismo en todas las páginas del conjunto.
    pub rect: PadesRect,
    /// Las páginas en las que se estampa.
    pub pages: PageSet,
}

impl Placement {
    fn extra_params(&self) -> Vec<(String, String)> {
        let Self { rect, pages } = self;
        let PadesRect {
            lower_left_x,
            lower_left_y,
            upper_right_x,
            upper_right_y,
        } = rect;
        vec![
            (PAGES_KEY.to_owned(), pages.literal()),
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
    /// Dónde cae el recuadro y en qué páginas, **cuando lo coloca rFirma**.
    ///
    /// `None` no es «una firma sin recuadro»: es «el recuadro no lo pone este
    /// lado». Los dos caminos del recuadro **no comparten conversión**
    /// (ID-282):
    ///
    /// - **Lo elige la persona** sobre el visor: llega aquí `Some`, ya con el
    ///   `T⁻¹` de la `/Rotate` aplicado por [`super::placement`].
    /// - **Lo elige la sede**: llega `None`, y sus `signaturePositionOnPage*`
    ///   cruzan al puente tal y como vinieron, porque esta configuración no
    ///   emite ninguna clave que los pise
    ///   ([`crate::protocol::visible`], [`crate::app::policies::merged_with`]).
    pub placement: Option<Placement>,
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
        // Destructurado exhaustivo A PROPÓSITO: un quinto campo no compila
        // hasta que alguien lo declare también en `Setting`.
        let Self {
            placement,
            layer2_text,
            rubric_image,
            sign_reason,
        } = self;

        let mut params = BTreeMap::new();
        params.insert(SUB_FILTER_KEY.to_owned(), SUB_FILTER.to_owned());
        if let Some(placement) = placement {
            params.extend(placement.extra_params());
        }
        params.insert(LAYER2_TEXT_KEY.to_owned(), layer2_text.clone());
        // Se envía siempre, y siempre cero: no es un ajuste del usuario, es lo
        // que enciende el reparto del alto entre las líneas.
        params.insert(LAYER2_FONT_SIZE_KEY.to_owned(), LAYER2_FONT_SIZE.to_owned());
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
    use super::{PadesRect, PageSet, Placement, Setting, SignatureConfig, SUB_FILTER};
    use std::collections::HashSet;

    fn a_rect() -> PadesRect {
        PadesRect {
            lower_left_x: 100,
            lower_left_y: 200,
            upper_right_x: 300,
            upper_right_y: 260,
        }
    }

    /// La colocación de rFirma, que es la del camino local: el recuadro lo
    /// puso la persona sobre el visor.
    fn placed_on(pages: PageSet) -> Option<Placement> {
        Some(Placement {
            rect: a_rect(),
            pages,
        })
    }

    fn minimal() -> SignatureConfig {
        SignatureConfig {
            placement: placed_on(PageSet::only_page(3)),
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
    fn closes_the_configuration_and_this_is_how_many_settings_it_has() {
        assert_eq!(Setting::ALL.len(), 6);
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
                    "«{key}» no pertenece a ninguno de los seis ajustes"
                );
            }
        }
    }

    #[test]
    fn emits_every_key_the_settings_declare() {
        // La dirección contraria a la de arriba: una clave declarada en
        // `Setting::keys()` que `extra_params` no llegue a emitir nunca sería
        // una promesa muerta. `complete()` tiene los seis ajustes puestos, así
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
        assert_eq!(params.get("signaturePages"), Some(&"3".to_owned()));
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

    /// El tamaño de letra viaja con **valor cero**, que es lo que hace
    /// que el compositor reparta el alto del recuadro entre sus líneas. Sin la
    /// clave, el valor por omisión del puente actúa como tope y la letra solo
    /// encoge.
    #[test]
    fn always_sends_the_font_size_as_zero() {
        for config in [minimal(), complete()] {
            assert_eq!(
                config.extra_params().get("layer2FontSize"),
                Some(&"0".to_owned())
            );
        }
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

    /// El singular y el plural conviven en el puente y **gana el plural**
    /// (`PdfUtil.getPages:699-703`), así que enviar los dos describiría una
    /// configuración que solo se entiende leyendo el código de AutoFirma.
    #[test]
    fn never_sends_the_singular_page_key() {
        assert!(!complete().extra_params().contains_key("signaturePage"));
    }

    /// El conjunto de tamaño 1, el de tamaño *k* y el de tamaño *n*: la misma
    /// clave, tres literales.
    #[test]
    fn writes_the_page_set_as_the_bridge_reads_it() {
        for (pages, literal) in [
            (PageSet::only_page(3), "3"),
            (PageSet::only([7, 3, 3]).expect("no esta vacio"), "3,7"),
            (PageSet::All, "all"),
        ] {
            let config = SignatureConfig {
                placement: placed_on(pages),
                ..minimal()
            };
            assert_eq!(
                config.extra_params().get("signaturePages"),
                Some(&literal.to_owned())
            );
        }
    }

    /// **ID-282**: en el camino de la sede rFirma no coloca nada, y eso se ve
    /// en que la geometría **no emite ni una clave**: las que mandó la sede
    /// llegan al puente sin que nadie las pise.
    #[test]
    fn emits_no_geometry_at_all_when_the_box_is_not_placed_by_rfirma() {
        let config = SignatureConfig {
            placement: None,
            ..complete()
        };

        let params = config.extra_params();

        for key in Setting::Geometry.keys() {
            assert!(
                !params.contains_key(*key),
                "'{key}' la pone la sede, no rFirma"
            );
        }
        assert!(params.contains_key("signatureSubFilter"), "lo demas sigue");
    }

    /// ID-91: «todas» no es un modo aparte. Lo único que cambia entre `all` y
    /// la lista completa es el literal; las otras cuatro claves de la
    /// geometría son las mismas, que es lo que hace que «algunas» no cueste
    /// nada de más.
    #[test]
    fn changes_nothing_but_the_page_set_between_all_and_the_full_list() {
        let all = SignatureConfig {
            placement: placed_on(PageSet::All),
            ..complete()
        };
        let listed = SignatureConfig {
            placement: placed_on(PageSet::only([1, 2, 3]).expect("no esta vacio")),
            ..complete()
        };

        let mut left = all.extra_params();
        let mut right = listed.extra_params();
        assert_ne!(
            left.remove("signaturePages"),
            right.remove("signaturePages")
        );
        assert_eq!(left, right);
    }
}
