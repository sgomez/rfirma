//! La expresión de filtro que manda la sede: **qué se deja pasar al motor**, no
//! qué hace el motor con ella (ID-252, ID-256).
//!
//! Quien decide qué certificados sobreviven es `CertFilterManager`, el motor de
//! `afirma-keystores-filters` prestado al puente. Aquí no se interpreta ni un
//! criterio: la expresión cruza **literal**, tal y como la escribió la sede.
//!
//! Lo único que se decide en este módulo es **si se llama**. El original es
//! *fail-open* —un criterio que no reconoce lo ignora en silencio, y el listado
//! sale más ancho de lo que la sede pidió—, así que aquí hay una **lista
//! blanca**: un criterio que no esté en ella es un `SAF_03` y no se filtra a
//! medias. Ese cierre del *fail-open*, comprobado en Rust antes de llamar al
//! puente, es el ID-255.
//!
//! **Lo que la lista blanca cierra, y lo que no.** Cierra el criterio
//! *desconocido*. **No** cierra el criterio conocido con el argumento
//! malformado: `CertFilterManager` sólo añade `ThumbPrintCertificateFilter` si
//! el argumento parte en exactamente dos trozos, y `PolicyIdFilter` sólo si los
//! OID no vienen vacíos; si no, devuelve un `MultipleCertificateFilter` de cero
//! filtros, que por construcción acepta a todos. Un `thumbprint:SHA1` o un
//! `policyid:` a secas siguen dejando el listado entero, igual que en el
//! original. Validar los argumentos de los cuatro criterios que los llevan
//! queda fuera de este módulo: haría falta reimplementar aquí lo que el motor
//! interpreta, que es justo lo que el ID-253 evita.
//!
//! # Las tres formas de declararlo, y su precedencia
//!
//! Los filtros no son un parámetro propio: viajan dentro de `properties`, como
//! claves del `.properties` que la sede codifica en Base64
//! (`CertFilterManager.java:165`-`182`):
//!
//! 1. `filter=<expresión>`
//! 2. `filters=<expresión>`
//! 3. `filters.1=<expresión>`, `filters.2=…`, numeradas desde 1 y sin huecos.
//!
//! La primera que aparezca gana, y las de abajo ni se miran. Dentro de una
//! expresión, `;` es **Y**; entre expresiones numeradas, **O**. Ninguna de las
//! dos reglas se reimplementa aquí: las aplica el motor (ID-253).
//!
//! # El `nonexpired` implícito, y dónde no llega
//!
//! Cuando la sede no declara ningún filtro, el motor añade uno que oculta los
//! caducados, citando la ETSI TS 119 102-1. Eso se hereda tal cual, y por eso
//! [`SiteFilter`] existe **también** cuando no hay nada declarado: el camino de
//! la sede llama al motor igualmente. Lo que **no** pasa por aquí es el listado
//! local de rFirma, que sigue enseñando el caducado con su estado (ID-254).

use super::refusal::Refusal;

/// La clave de la primera forma.
const FILTER: &str = "filter";
/// La clave de la segunda forma, y el prefijo de la tercera.
const FILTERS: &str = "filters";

/// Los criterios que rFirma deja cruzar al motor.
///
/// Son los del original (`CertFilterManager.java:39`-`67`), con su prefijo
/// literal y en minúsculas. La lista se compara **en minúsculas**, igual que el
/// `toLowerCase().startsWith(...)` del original.
///
/// `keyusage.` no lleva dos puntos a propósito: los nueve bits son
/// `keyusage.<bit>:` y el original los agrupa por ese prefijo.
pub const ACCEPTED_CRITERIA: &[&str] = &[
    "authcert:",
    "dnie:",
    "encodedcert:",
    "issuer.contains:",
    "issuer.rfc2254.recurse:",
    "issuer.rfc2254:",
    "keyusage.",
    "nonexpired:",
    "policyid:",
    "pseudonym:",
    "qualified:",
    "signingcert:",
    "ssl:",
    "sscd:",
    "subject.contains:",
    "subject.rfc2254:",
    "thumbprint:",
];

/// El único criterio **sin argumento**, que además no filtra nada.
///
/// Prohíbe abrir otros almacenes desde el diálogo de selección del original.
/// En rFirma queda satisfecha **por construcción** (ID-257): de la selección de
/// certificado no se abre ningún almacén. Se acepta para no rechazar una URL
/// perfectamente válida por una clave que aquí no hace falta.
pub const SATISFIED_BY_CONSTRUCTION: &str = "disableopeningexternalstores";

/// Los cuatro criterios que **se aceptan sin cobertura de su veredicto**
/// (ID-260).
///
/// Medir cualquiera de ellos exige material que no existe en el kit de pruebas:
/// un DNIe de verdad, un certificado SSL, uno cualificado con su declaración
/// QCStatement y uno de seudónimo emitido por una CA real. Cruzan al motor como
/// los demás y es él quien los aplica; lo que falta es la prueba de que su
/// veredicto sea el correcto, y esta constante es donde está anotado.
pub const UNMEASURED_CRITERIA: &[&str] = &["dnie:", "pseudonym:", "qualified:", "ssl:"];

/// Lo que la sede pide del listado, listo para cruzar al motor.
///
/// **Guarda las claves y los valores tal cual llegaron.** No hay expresión
/// reescrita, ni criterios reordenados, ni valores normalizados: lo único que
/// se hace al serializar es escapar lo que rompería el formato
/// `java.util.Properties` por el camino.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteFilter {
    declared: Vec<(String, String)>,
}

impl SiteFilter {
    /// La sede no declaró ningún filtro: el motor aplicará su `nonexpired`
    /// implícito (ID-254).
    pub fn declares_nothing(&self) -> bool {
        self.declared.is_empty()
    }

    /// Las claves declaradas, en el orden en que se recogieron.
    pub fn declared(&self) -> &[(String, String)] {
        &self.declared
    }

    /// El bloque `java.util.Properties` que recibe el puente.
    ///
    /// El escapado es de **transporte**: `Properties.load` se come una barra
    /// invertida suelta y parte el bloque en un salto de línea, así que un
    /// `subject.rfc2254` con escapes del RFC 2254 llegaría mutilado. Escapando
    /// aquí, lo que el motor lee es byte a byte lo que escribió la sede.
    ///
    /// El bloque sale en **UTF-8** y el puente lo lee con un `Reader` en UTF-8
    /// (`SessionStamp.parseParams`), no con la sobrecarga de `Properties.load`
    /// que toma un flujo de bytes: aquella descodifica ISO-8859-1 por contrato
    /// y mutilaría en silencio cualquier `subject.contains:` con eñe o con
    /// tilde —el caso normal en España—, que el motor devolvería como listado
    /// vacío y la aplicación contaría como «la sede los excluyó» (ID-258).
    pub fn as_java_properties(&self) -> String {
        let mut block = String::new();
        for (key, value) in &self.declared {
            block.push_str(key);
            block.push('=');
            for character in value.chars() {
                match character {
                    '\\' => block.push_str("\\\\"),
                    '\n' => block.push_str("\\n"),
                    '\r' => block.push_str("\\r"),
                    other => block.push(other),
                }
            }
            block.push('\n');
        }
        block
    }
}

/// Lo que la sede pide del listado, o por qué no se le sirve.
///
/// `properties` son los pares del `.properties` que la sede mandó, ya
/// descodificados. Se recogen las claves de filtro **con la precedencia del
/// original** y se comprueba criterio a criterio contra [`ACCEPTED_CRITERIA`].
///
/// Devuelve un [`SiteFilter`] **siempre** que la expresión sea aceptable,
/// incluso vacío: en el camino de la sede, «no declaró nada» no es «no
/// filtrar», es «el `nonexpired` de la ETSI» (ID-254).
pub fn site_filter(properties: &[(String, String)]) -> Result<SiteFilter, Refusal> {
    let declared = declared_keys(properties);

    for (key, expression) in &declared {
        for criterion in expression.split(';') {
            check_is_accepted(key, criterion)?;
        }
    }

    Ok(SiteFilter { declared })
}

/// Las claves de filtro que la sede declaró, con la precedencia del original:
/// `filter`, si no `filters`, si no `filters.1`, `filters.2`… sin huecos.
fn declared_keys(properties: &[(String, String)]) -> Vec<(String, String)> {
    if let Some(value) = value_of(properties, FILTER) {
        return vec![(FILTER.to_owned(), value.to_owned())];
    }
    if let Some(value) = value_of(properties, FILTERS) {
        return vec![(FILTERS.to_owned(), value.to_owned())];
    }

    let mut numbered = Vec::new();
    for index in 1.. {
        let key = format!("{FILTERS}.{index}");
        let Some(value) = value_of(properties, &key) else {
            break;
        };
        numbered.push((key, value.to_owned()));
    }
    numbered
}

fn value_of<'a>(properties: &'a [(String, String)], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// Un criterio que la lista blanca no reconoce **no se ignora**: se rechaza
/// (ID-255).
///
/// Es la única diferencia deliberada con el original, y va en la dirección
/// segura. Allí un criterio desconocido se cae por el `else` final y el filtro
/// resultante es más ancho que el que la sede escribió; aquí, una sede que pida
/// algo que rFirma no sabe aplicar se entera con un `SAF_03` en vez de recibir
/// un listado que creía acotado.
fn check_is_accepted(key: &str, criterion: &str) -> Result<(), Refusal> {
    let trimmed = criterion.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let lowercase = trimmed.to_ascii_lowercase();

    if lowercase == SATISFIED_BY_CONSTRUCTION {
        return Ok(());
    }
    if ACCEPTED_CRITERIA
        .iter()
        .any(|accepted| lowercase.starts_with(accepted))
    {
        return Ok(());
    }

    Err(Refusal::params(format!(
        "el criterio de filtro '{trimmed}' de '{key}' no esta en la lista blanca"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::SafCode;

    fn properties(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    /// La expresión llega al motor **sin tocar**: mismo texto, mismo orden,
    /// misma clave (ID-256).
    #[test]
    fn the_expression_crosses_to_the_engine_literally() {
        let expression = "subject.contains:PEREZ;issuer.contains:FNMT";
        let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

        assert_eq!(
            filter.declared(),
            [("filters".to_owned(), expression.to_owned())]
        );
        assert_eq!(
            filter.as_java_properties(),
            format!("filters={expression}\n")
        );
    }

    /// Las tres formas del original, con su precedencia: la primera que aparece
    /// gana y las de abajo ni se miran.
    #[test]
    fn the_first_of_the_three_spellings_wins() {
        let all_three = properties(&[
            ("filter", "dnie:true"),
            ("filters", "ssl:true"),
            ("filters.1", "sscd:true"),
        ]);

        assert_eq!(
            site_filter(&all_three).expect("es aceptable").declared(),
            [("filter".to_owned(), "dnie:true".to_owned())]
        );

        let without_the_first = properties(&[("filters", "ssl:true"), ("filters.1", "sscd:true")]);
        assert_eq!(
            site_filter(&without_the_first)
                .expect("es aceptable")
                .declared(),
            [("filters".to_owned(), "ssl:true".to_owned())]
        );
    }

    /// Las numeradas se recogen **enteras y en orden**, y se paran en el primer
    /// hueco: es lo que el original hace con su `while`.
    #[test]
    fn the_numbered_ones_are_collected_in_order_and_stop_at_the_first_gap() {
        let with_a_gap = properties(&[
            ("filters.1", "subject.contains:UNO"),
            ("filters.2", "subject.contains:DOS"),
            ("filters.4", "subject.contains:CUATRO"),
        ]);

        let filter = site_filter(&with_a_gap).expect("es aceptable");

        assert_eq!(
            filter.declared(),
            [
                ("filters.1".to_owned(), "subject.contains:UNO".to_owned()),
                ("filters.2".to_owned(), "subject.contains:DOS".to_owned()),
            ]
        );
    }

    /// El caso que sostiene el ID-254: sin filtro declarado **sí hay** filtro,
    /// porque el motor añade el `nonexpired` de la ETSI. Lo que sale de aquí no
    /// es «no llames al motor».
    #[test]
    fn a_site_that_declares_nothing_still_gets_the_engine_called() {
        let filter = site_filter(&properties(&[("format", "PAdES")])).expect("es aceptable");

        assert!(filter.declares_nothing());
        assert_eq!(filter.as_java_properties(), "");
    }

    /// La lista blanca es *fail-closed*, al revés que el original: un criterio
    /// desconocido no se ignora, se rechaza (ID-255).
    #[test]
    fn a_criterion_outside_the_whitelist_is_refused_instead_of_ignored() {
        let refusal = site_filter(&properties(&[(
            "filters",
            "subject.contains:PEREZ;inventado:loquesea",
        )]))
        .expect_err("'inventado:' no existe");

        assert_eq!(refusal.code(), SafCode::Params);
        assert!(refusal.detail().contains("inventado:loquesea"));
    }

    /// Y los criterios de verdad pasan **todos**, incluidos los nueve bits de
    /// `keyusage` y la clave sin argumento del ID-257.
    #[test]
    fn every_criterion_the_original_understands_is_accepted() {
        for criterion in ACCEPTED_CRITERIA {
            let expression = format!("{criterion}loquesea");
            assert!(
                site_filter(&properties(&[("filters", &expression)])).is_ok(),
                "«{expression}» tendria que cruzar al motor"
            );
        }

        assert!(site_filter(&properties(&[(
            "filters",
            "keyusage.digitalsignature:true"
        )]))
        .is_ok());
        assert!(site_filter(&properties(&[("filters", SATISFIED_BY_CONSTRUCTION)])).is_ok());
    }

    /// La comparación es en minúsculas, como el `toLowerCase().startsWith` del
    /// original: una sede que escriba `Subject.Contains:` no se lleva un rechazo
    /// que el cliente publicado no le daría.
    #[test]
    fn the_criteria_are_recognised_regardless_of_case() {
        assert!(site_filter(&properties(&[("filters", "Subject.Contains:PEREZ")])).is_ok());
        assert!(site_filter(&properties(&[("filters", "NONEXPIRED:true")])).is_ok());
    }

    /// Los cuatro sin medir se aceptan igual que los demás (ID-260). La
    /// cobertura que falta es la de su **veredicto**, no la de su paso.
    #[test]
    fn the_four_unmeasured_criteria_are_accepted_all_the_same() {
        for criterion in UNMEASURED_CRITERIA {
            assert!(
                ACCEPTED_CRITERIA.contains(criterion),
                "«{criterion}» esta anotado como sin medir pero no cruza"
            );
            assert!(site_filter(&properties(&[("filters", &format!("{criterion}true"))])).is_ok());
        }
    }

    /// `headless` y `mandatoryCertSelection` **no son criterios de `filters=`**
    /// sino claves hermanas del mismo `.properties` (ID-257): no pasan por la
    /// lista blanca y no la ponen roja.
    #[test]
    fn the_sibling_keys_are_not_criteria_and_do_not_trip_the_whitelist() {
        let with_siblings = properties(&[
            ("headless", "true"),
            ("mandatoryCertSelection", "false"),
            ("filters", "subject.contains:PEREZ"),
        ]);

        assert!(site_filter(&with_siblings).is_ok());
    }

    /// Un valor con barras invertidas —un `subject.rfc2254` con escapes— llega
    /// al motor **entero**: si se serializara crudo, `Properties.load` se
    /// comería la barra y el filtro dejaría de ser el que la sede escribió.
    #[test]
    fn a_value_with_backslashes_survives_the_properties_block() {
        let expression = r"subject.rfc2254:(cn=PEREZ\, JUAN)";
        let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

        assert_eq!(
            filter.as_java_properties(),
            "filters=subject.rfc2254:(cn=PEREZ\\\\, JUAN)\n"
        );
    }

    /// Un valor **no ASCII** cruza igual de entero. El escapado no lo toca, y
    /// el bloque viaja en UTF-8: quien lo descodifique como ISO-8859-1 al otro
    /// lado convertiría cada letra acentuada en dos caracteres, ninguno el
    /// bueno, y el filtro no casaría con nada sin que nada se pusiera rojo.
    #[test]
    fn a_value_with_accents_reaches_the_engine_unchanged() {
        let expression = "subject.contains:MUÑOZ PÉREZ";
        let filter = site_filter(&properties(&[("filters", expression)])).expect("es aceptable");

        let block = filter.as_java_properties();

        assert_eq!(block, "filters=subject.contains:MUÑOZ PÉREZ\n");
        assert!(block.contains('Ñ'));
        assert!(block.contains('É'));
    }

    /// Un salto de línea dentro del valor partiría el bloque en dos claves.
    #[test]
    fn a_newline_inside_a_value_cannot_split_the_block() {
        let filter = site_filter(&properties(&[("filters", "subject.contains:A\nB")]))
            .expect("es aceptable");

        assert_eq!(
            filter.as_java_properties(),
            "filters=subject.contains:A\\nB\n"
        );
        assert_eq!(filter.as_java_properties().lines().count(), 1);
    }
}
