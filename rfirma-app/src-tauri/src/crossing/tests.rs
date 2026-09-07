use super::*;

crossing! {
    /// Un tipo de pruebas.
    #[derive(Clone, Debug, PartialEq, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SyntheticView {
        /// Un campo con guion bajo.
        pub holder_name: String,
        pub rect: [f64; 4],
        pub inner: Option<Vec<SyntheticStageView>>,
    }
}

crossing! {
    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum SyntheticStageView {
        Waiting,
        #[serde(rename_all = "camelCase")]
        AskingToSign {
            /// Un campo.
            document: String,
            attempts_left: Option<u32>,
        },
    }
}

crossing! {
    #[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SyntheticOrder {
        pub page_count: u32,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Borrowed {
    All,
    Only(std::collections::BTreeSet<u32>),
    Some { count: usize },
}

crossing! {
    lent from "somewhere/domain/borrowed.rs":
    pub enum Borrowed {
        All,
        Only(std::collections::BTreeSet<u32>),
        Some { count: usize },
    }
}

#[test]
fn a_struct_renders_its_fields_with_the_names_the_window_sees() {
    assert_eq!(
        SyntheticView::CROSSING.rendered(),
        "\n  pub struct SyntheticView\n      holderName: String\n      rect: [f64; 4]\n      inner: Option<Vec<SyntheticStageView>>\n"
    );
}

#[test]
fn a_tagged_enum_renders_its_tag_and_its_variants_on_one_line_each() {
    assert_eq!(
        SyntheticStageView::CROSSING.rendered(),
        "\n  pub enum SyntheticStageView   (serde: etiqueta \"kind\")\n      Waiting\n      AskingToSign { document: String, attemptsLeft: Option<u32> }\n"
    );
}

#[test]
fn a_lent_type_renders_where_it_comes_from_and_its_tuple_variant() {
    assert_eq!(
        Borrowed::CROSSING.rendered(),
        "\n  pub enum Borrowed   [somewhere/domain/borrowed.rs]\n      All\n      Only(std::collections::BTreeSet<u32>)\n      Some { count: usize }\n"
    );
}

#[test]
fn the_direction_of_a_crossing_is_read_from_its_derives() {
    assert!(SyntheticView::CROSSING.serialises());
    assert!(!SyntheticView::CROSSING.deserialises());
    assert!(!SyntheticOrder::CROSSING.serialises());
    assert!(SyntheticOrder::CROSSING.deserialises());
}

#[test]
fn every_declared_type_is_in_the_registry_without_editing_any_list() {
    let names: Vec<&str> = all_crossings().iter().map(|c| c.name).collect();
    for expected in [
        "SyntheticView",
        "SyntheticStageView",
        "SyntheticOrder",
        "Borrowed",
    ] {
        assert!(names.contains(&expected), "{expected} no esta en {names:?}");
    }
}

#[test]
fn the_registry_orders_own_types_by_file_and_line_and_lent_ones_by_name_after_them() {
    let all = all_crossings();
    let own_after_lent = all
        .iter()
        .skip_while(|c| c.lent_from.is_none())
        .any(|c| c.lent_from.is_none());
    assert!(!own_after_lent, "un propio detras de un prestado");
    let own: Vec<(&str, u32)> = all
        .iter()
        .filter(|c| c.lent_from.is_none())
        .map(|c| (c.file, c.line))
        .collect();
    assert!(own.windows(2).all(|pair| pair[0] <= pair[1]), "{own:?}");
    assert!(SyntheticView::CROSSING.file.ends_with("crossing/tests.rs"));
}

#[test]
fn the_types_a_crossing_names_are_the_capitalised_words_of_its_fields() {
    assert_eq!(
        SyntheticView::CROSSING.referenced_types(),
        ["Option", "String", "SyntheticStageView", "Vec"]
    );
    assert_eq!(Borrowed::CROSSING.referenced_types(), ["BTreeSet"]);
}

#[test]
fn what_serde_writes_is_what_the_description_says() {
    let value = SyntheticView {
        holder_name: "Ana".to_owned(),
        rect: [0.0, 0.0, 1.0, 1.0],
        inner: Some(vec![
            SyntheticStageView::Waiting,
            SyntheticStageView::AskingToSign {
                document: "d".to_owned(),
                attempts_left: Some(2),
            },
        ]),
    };
    let json = serde_json::to_value(&value).expect("serializa");
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("objeto")
        .keys()
        .map(String::as_str)
        .collect();
    let described: std::collections::BTreeSet<String> = SyntheticView::CROSSING
        .members
        .iter()
        .map(|member| match member {
            Member::Field(field) => SyntheticView::CROSSING.field_name(field),
            Member::Variant { .. } => unreachable!("un struct no tiene variantes"),
        })
        .collect();
    let described: std::collections::BTreeSet<&str> =
        described.iter().map(String::as_str).collect();
    assert_eq!(keys, described);
    assert_eq!(json["inner"][1]["kind"], "askingToSign");
    assert_eq!(json["inner"][1]["attemptsLeft"], 2);
}

#[test]
fn a_lent_declaration_is_checked_against_the_real_type_by_the_compiler() {
    let each = [
        Borrowed::All,
        Borrowed::Only(std::collections::BTreeSet::from([1])),
        Borrowed::Some { count: 1 },
    ];
    assert_eq!(each.len(), Borrowed::CROSSING.members.len());
}

#[test]
fn camel_case_is_what_serde_does() {
    assert_eq!(camel("attempts_left"), "attemptsLeft");
    assert_eq!(camel("id"), "id");
    assert_eq!(camel("also_entering_now"), "alsoEnteringNow");
}
