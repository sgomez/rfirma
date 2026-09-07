//! El rasgo de los tipos que cruzan a la ventana y el registro que el enlazador completa: de él salen el contrato y la guarda de rutas, no del fuente.

/// Lo que un tipo de cruce dice de sí mismo, con la forma exacta que ve la ventana.
pub trait WindowCrossing {
    const CROSSING: Crossing;
}

/// La descripción de un tipo de cruce: su declaración, campo a campo, y de dónde viene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crossing {
    pub name: &'static str,
    pub shape: Shape,
    pub attributes: &'static [&'static str],
    pub members: &'static [Member],
    pub file: &'static str,
    pub line: u32,
    pub lent_from: Option<&'static str>,
}

/// `struct` o `enum`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Struct,
    Enum,
}

/// Un campo de un `struct` o de una variante, con el nombre de Rust y el tipo tal cual se escribió.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub ty: &'static str,
}

/// Un campo de un `struct` o una variante de un `enum`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    Field(Field),
    Variant {
        name: &'static str,
        tuple: Option<&'static str>,
        fields: &'static [Field],
    },
}

/// Una entrada del registro: la descripción de un tipo, puesta ahí por `crossing!`.
pub struct Registered(pub &'static Crossing);

inventory::collect!(Registered);

/// Todo lo registrado, en orden de fichero y línea: primero los propios, luego los prestados por nombre.
pub fn all_crossings() -> Vec<&'static Crossing> {
    let mut found: Vec<&'static Crossing> = inventory::iter::<Registered>
        .into_iter()
        .map(|registered| registered.0)
        .collect();
    found.sort_by_key(|crossing| {
        (
            crossing.lent_from.is_some(),
            if crossing.lent_from.is_some() {
                (crossing.name, 0)
            } else {
                (crossing.file, crossing.line)
            },
        )
    });
    found
}

impl Crossing {
    /// Comprueba si el tipo deriva `Serialize`: sale hacia la ventana.
    pub fn serialises(&self) -> bool {
        self.derives("Serialize")
    }

    /// Comprueba si el tipo deriva `Deserialize`: entra desde la ventana.
    pub fn deserialises(&self) -> bool {
        self.derives("Deserialize")
    }

    fn derives(&self, what: &str) -> bool {
        self.attributes
            .iter()
            .filter_map(|attribute| attribute.strip_prefix("derive("))
            .flat_map(|list| list.split(|letter: char| !letter.is_alphanumeric() && letter != '_'))
            .any(|derived| derived == what)
    }

    fn serde_attribute(&self) -> &'static str {
        self.attributes
            .iter()
            .copied()
            .find(|attribute| attribute.starts_with("serde("))
            .unwrap_or_default()
    }

    /// El valor de `tag = "…"` en el atributo de serde, si lo hay.
    pub fn tag(&self) -> Option<&'static str> {
        let serde = self.serde_attribute();
        let after = serde.split_once("tag = \"")?.1;
        after.split_once('"').map(|(tag, _)| tag)
    }

    fn renames_to_camel_case(&self) -> bool {
        self.serde_attribute().contains("camelCase")
    }

    /// El nombre de un campo tal como lo ve la ventana.
    pub fn field_name(&self, field: &Field) -> String {
        if self.renames_to_camel_case() {
            camel(field.name)
        } else {
            field.name.to_owned()
        }
    }

    /// Los tipos nombrados en los campos, con su nombre de Rust, sin repetir.
    pub fn referenced_types(&self) -> Vec<&'static str> {
        let mut found: Vec<&'static str> = self
            .members
            .iter()
            .flat_map(|member| match member {
                Member::Field(field) => vec![field.ty],
                Member::Variant { tuple, fields, .. } => tuple
                    .iter()
                    .copied()
                    .chain(fields.iter().map(|f| f.ty))
                    .collect(),
            })
            .flat_map(type_names_in)
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    /// La declaración como la lee la ventana: cabecera y una línea por campo o variante.
    pub fn rendered(&self) -> String {
        let keyword = match self.shape {
            Shape::Struct => "struct",
            Shape::Enum => "enum",
        };
        let mut text = format!("\n  pub {keyword} {}", self.name);
        if let Some(tag) = self.tag() {
            text.push_str(&format!("   (serde: etiqueta \"{tag}\")"));
        }
        if let Some(source) = self.lent_from {
            text.push_str(&format!("   [{source}]"));
        }
        text.push('\n');
        for member in self.members {
            text.push_str("      ");
            text.push_str(&self.rendered_member(member));
            text.push('\n');
        }
        text
    }

    fn rendered_member(&self, member: &Member) -> String {
        match member {
            Member::Field(field) => format!("{}: {}", self.field_name(field), field.ty),
            Member::Variant {
                name,
                tuple: Some(payload),
                ..
            } => format!("{name}({payload})"),
            Member::Variant {
                name, fields: [], ..
            } => (*name).to_owned(),
            Member::Variant { name, fields, .. } => {
                let listed: Vec<String> = fields
                    .iter()
                    .map(|field| format!("{}: {}", self.field_name(field), field.ty))
                    .collect();
                format!("{name} {{ {} }}", listed.join(", "))
            }
        }
    }
}

/// Los identificadores de tipo que aparecen en un tipo escrito: `Option<Vec<Foo>>` nombra tres.
pub fn type_names_in(ty: &str) -> Vec<&str> {
    ty.split(|letter: char| !letter.is_alphanumeric() && letter != '_')
        .filter(|word| word.starts_with(|letter: char| letter.is_ascii_uppercase()))
        .collect()
}

/// `snake_case` a `camelCase`, como `rename_all = "camelCase"`.
pub fn camel(name: &str) -> String {
    let mut parts = name.split('_');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let mut letters = part.chars();
        if let Some(first) = letters.next() {
            out.extend(first.to_uppercase());
            out.push_str(letters.as_str());
        }
    }
    out
}

/// La sección «TIPOS QUE CRUZAN» del contrato, con los prestados al final.
pub fn types_section() -> String {
    let crossings = all_crossings();
    let mut text = String::new();
    for crossing in crossings.iter().filter(|c| c.lent_from.is_none()) {
        text.push_str(&crossing.rendered());
    }
    let lent: Vec<&Crossing> = crossings
        .iter()
        .copied()
        .filter(|c| c.lent_from.is_some())
        .collect();
    if !lent.is_empty() {
        text.push_str("\n  PRESTADOS DE OTROS MODULOS\n");
        for crossing in lent {
            text.push_str(&crossing.rendered());
        }
    }
    text
}

macro_rules! register {
    (
        $name:ident, $shape:expr, $lent:expr,
        [$($attribute:expr),*],
        [$($member:expr),*]
    ) => {
        impl $crate::crossing::WindowCrossing for $name {
            const CROSSING: $crate::crossing::Crossing = $crate::crossing::Crossing {
                name: stringify!($name),
                shape: $shape,
                attributes: &[$($attribute),*],
                members: &[$($member),*],
                file: file!(),
                line: line!(),
                lent_from: $lent,
            };
        }
        inventory::submit! {
            $crate::crossing::Registered(&<$name as $crate::crossing::WindowCrossing>::CROSSING)
        }
    };
}

/// Declara un tipo que cruza a la ventana: la declaración tal cual, más su descripción en el registro.
macro_rules! crossing {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field: $ty,
            )*
        }
        $crate::crossing::register! {
            $name, $crate::crossing::Shape::Struct, None,
            [$(stringify!($meta)),*],
            [$($crate::crossing::Member::Field($crate::crossing::Field {
                name: stringify!($field),
                ty: stringify!($ty),
            })),*]
        }
    };
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident
                $( { $( $(#[$vf_meta:meta])* $vf:ident : $vt:ty ),* $(,)? } )?
                $( ( $tuple:ty ) )?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant
                $( { $( $(#[$vf_meta])* $vf: $vt ),* } )?
                $( ( $tuple ) )?,
            )*
        }
        $crate::crossing::register! {
            $name, $crate::crossing::Shape::Enum, None,
            [$(stringify!($meta)),*],
            [$($crate::crossing::Member::Variant {
                name: stringify!($variant),
                tuple: $crate::crossing::payload!($($tuple)?),
                fields: &[$($($crate::crossing::Field {
                    name: stringify!($vf),
                    ty: stringify!($vt),
                }),*)?],
            }),*]
        }
    };
    (
        lent from $source:literal:
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $variant:ident
                $( { $( $vf:ident : $vt:ty ),* $(,)? } )?
                $( ( $tuple:ty ) )?
            ),* $(,)?
        }
    ) => {
        const _: fn($name) = |value| match value {
            $(
                $name::$variant
                $( { $($vf),* } )?
                $( ( $crate::crossing::wild!($tuple) ) )?
                => {
                    $( $( let _: $vt = $vf; )* )?
                    $( let _: fn($tuple) -> $name = $name::$variant; )?
                }
            )*
        };
        $crate::crossing::register! {
            $name, $crate::crossing::Shape::Enum, Some($source),
            [$(stringify!($meta)),*],
            [$($crate::crossing::Member::Variant {
                name: stringify!($variant),
                tuple: $crate::crossing::payload!($($tuple)?),
                fields: &[$($($crate::crossing::Field {
                    name: stringify!($vf),
                    ty: stringify!($vt),
                }),*)?],
            }),*]
        }
    };
}

macro_rules! payload {
    () => {
        None
    };
    ($tuple:ty) => {
        Some(stringify!($tuple))
    };
}

macro_rules! wild {
    ($tuple:ty) => {
        _
    };
}

pub(crate) use {crossing, payload, register, wild};

#[cfg(test)]
mod tests;
