//! Lo que el caso de uso cuenta de un documento, antes de que la vista le ponga el formato del cable.

/// Documento abierto para su visualización o firma (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedDocument {
    /// Identificador opaco asignado al documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Ruta en el anfitrión si está disponible.
    pub path: Option<String>,
}

/// Por qué lo soltado no ha abierto ningún documento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropRefusal {
    /// Ninguno de los ficheros soltados es un PDF.
    NotAPdf,
    /// El fichero soltado no se ha podido leer.
    Unreadable(String),
}

/// Resultado de soltar ficheros sobre la ventana (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroppedDocument {
    /// Documento abierto en el visor.
    pub document: Option<OpenedDocument>,
    /// Documentos adicionales incorporados a recientes.
    pub also_entering: Vec<OpenedDocument>,
    /// Motivo por el que no se abrió ningún documento.
    pub refused: Option<DropRefusal>,
    /// Número de ficheros descartados que no se incorporaron.
    pub discarded: usize,
}

/// Destino previsto para el documento firmado (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Nombre del fichero firmado resultante.
    pub name: Option<String>,
    /// Si la carpeta de destino tiene permisos de escritura.
    pub writable: bool,
}

/// Documento firmado ya entregado (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedDocument {
    /// Nombre del fichero resultante.
    pub name: String,
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Tamaño en bytes del fichero escrito.
    pub size_bytes: u64,
}
