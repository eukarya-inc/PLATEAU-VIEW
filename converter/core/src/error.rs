use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    RawIo(#[from] std::io::Error),

    #[error("{path}: {source}")]
    Zip {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },

    #[error("{path}: {source}")]
    Xml {
        path: PathBuf,
        #[source]
        source: quick_xml::Error,
    },

    #[error("{path}: malformed XML: {message}")]
    MalformedXml { path: PathBuf, message: String },

    #[error("profile: {0}")]
    Profile(String),

    /// The input is well-formed but no profile converts it -- the wrong CityGML
    /// version, or an i-UR version nothing targets. Distinct from `Profile`,
    /// which means a profile *file* is wrong.
    #[error("{0}")]
    Unsupported(String),

    #[error("could not parse profile: {0}")]
    ProfileSyntax(#[from] toml::de::Error),

    /// The inputs could not be understood as a PLATEAU dataset.
    #[error("unrecognised input layout: {0}")]
    Layout(String),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn zip(path: impl Into<PathBuf>, source: zip::result::ZipError) -> Self {
        Error::Zip {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn xml(path: impl Into<PathBuf>, source: quick_xml::Error) -> Self {
        Error::Xml {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn malformed(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Error::MalformedXml {
            path: path.into(),
            message: message.into(),
        }
    }
}
