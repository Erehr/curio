//! PDF, text, Markdown and office files.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "document",
    name: "Documents",
    description: "PDF, text, Markdown, office files and spreadsheets.",
    icon: "file-text",
    kind: AssetKind::Document,
    exts: &[
        "pdf", "txt", "md", "markdown", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "potx",
        "odt", "ods", "odp", "rtf", "csv", "tsv", "log", "srt", "vtt", "key", "pages", "number", "numbers",
    ],
};
