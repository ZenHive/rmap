//! Typed wire vocabularies. Unknown values survive decoding so validation can
//! retain its semantic diagnostics (including source lines and exit codes).
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! vocabulary {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            $($variant,)+
            /// Unvalidated input, rejected by the semantic validator.
            Unknown(String),
        }
        impl JsonSchema for $name {
            fn inline_schema() -> bool { true }
            fn schema_name() -> std::borrow::Cow<'static, str> { String::schema_name() }
            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                String::json_schema(generator)
            }
        }
        impl $name {
            pub const VALUES: &'static [&'static str] = &[$($wire,)+];
            pub fn as_str(&self) -> &str {
                match self { $(Self::$variant => $wire,)+ Self::Unknown(value) => value }
            }
            pub fn is_known(&self) -> bool { !matches!(self, Self::Unknown(_)) }
        }
        impl From<String> for $name {
            fn from(value: String) -> Self {
                match value.as_str() { $($wire => Self::$variant,)+ _ => Self::Unknown(value) }
            }
        }
        impl From<&str> for $name {
            fn from(value: &str) -> Self { value.to_owned().into() }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self { value.as_str().to_owned() }
        }
        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &str { self.as_str() }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str { self.as_str() }
        }
        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str { self.as_str() }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool { self.as_str() == other }
        }
        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool { self.as_str() == *other }
        }
        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool { self.as_str() == other }
        }
    };
}

vocabulary!(Status {
    Pending => "pending", InProgress => "in_progress", Blocked => "blocked",
    Done => "done", Superseded => "superseded",
});
vocabulary!(Marker {
    Parallel => "parallel", Cx => "cx", Csr => "csr", Bug => "bug",
    Security => "security", Docs => "docs", Handbuild => "handbuild",
});
vocabulary!(Relation { Blocks => "blocks", BlockedBy => "blocked_by", Related => "related" });

impl Default for Status {
    fn default() -> Self {
        Self::Pending
    }
}

impl Status {
    pub fn symbol(&self) -> &str {
        match self {
            Self::Pending => "⬜",
            Self::InProgress => "🔄",
            Self::Blocked => "🔶",
            Self::Done => "✅",
            Self::Superseded => "⛔",
            Self::Unknown(value) => value,
        }
    }
}

impl Marker {
    pub fn suffix(&self) -> Option<&'static str> {
        match self {
            Self::Parallel => Some("`[P]`"),
            Self::Cx => Some("`[CX]`"),
            Self::Csr => Some("`[CSR]`"),
            _ => None,
        }
    }
    pub fn category(&self) -> Option<&'static str> {
        match self {
            Self::Bug => Some("🐛"),
            Self::Security => Some("🔒"),
            Self::Docs => Some("📝"),
            _ => None,
        }
    }
}

/// Ordered typed markers. The generic membership check also accepts wire
/// strings, preserving callers that queried the former string list.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Markers(Vec<Marker>);

impl Markers {
    pub fn contains<T: ?Sized>(&self, value: &T) -> bool
    where
        Marker: PartialEq<T>,
    {
        self.0.iter().any(|marker| marker == value)
    }
}

impl std::ops::Deref for Markers {
    type Target = Vec<Marker>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Markers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a> IntoIterator for &'a Markers {
    type Item = &'a Marker;
    type IntoIter = std::slice::Iter<'a, Marker>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl JsonSchema for Markers {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> std::borrow::Cow<'static, str> {
        Vec::<String>::schema_name()
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        Vec::<String>::json_schema(generator)
    }
}
