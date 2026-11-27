//! What Curio tells a person in the window: the English, with the key of the window's catalog (`locales/en/app.json`) that says each sentence and its values, so the window shows it in its own language.
//! An id must be in English's catalog with the same named values (Curio's catalog check reads this crate for them).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One sentence: its English, and the catalog key with its values when the window has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub args: Value,
}

/// Sentences shown one after another; text from elsewhere (a plugin, the system) has no id and shows as it is.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Said(pub Vec<Part>);

impl Said {
    /// `args` is a JSON object; a value named `count` picks the plural form.
    pub fn new(id: &str, args: Value, text: impl Into<String>) -> Said {
        Said(vec![Part { text: text.into(), id: Some(id.into()), args }])
    }

    pub fn and(mut self, id: &str, args: Value, text: impl Into<String>) -> Said {
        self.0.push(Part { text: text.into(), id: Some(id.into()), args });
        self
    }

    /// After the first of `total` problems: how many more there were, which the log lists.
    pub fn and_more(self, total: usize) -> Said {
        match total {
            0 | 1 => self,
            n => self.and("errors.more", serde_json::json!({"count": n - 1}), format!("(and {} more, listed in curio.log)", n - 1)),
        }
    }

    /// The English.
    pub fn text(&self) -> String {
        self.0.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join(" ")
    }
}

impl From<String> for Said {
    fn from(text: String) -> Said {
        Said(vec![Part { text, id: None, args: Value::Null }])
    }
}

impl From<&str> for Said {
    fn from(text: &str) -> Said {
        text.to_string().into()
    }
}

impl std::fmt::Display for Said {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text())
    }
}
