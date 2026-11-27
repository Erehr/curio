//! HTML, JSON, XML, scripts and source code files.
use super::Family;
use crate::core::model::AssetKind;

pub const FAMILY: Family = Family {
    id: "code",
    name: "Code",
    description: "HTML, JSON, XML, scripts and source code files.",
    icon: "code",
    kind: AssetKind::Code,
    exts: &[
        "html", "htm", "mhtml", "xhtml", "json", "jsonc", "json5", "xml", "xsl", "xslt",
        "js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx", "css", "scss", "sass", "less",
        "yaml", "yml", "toml", "ini", "cfg", "conf", "env",
        "sh", "bash", "zsh", "fish", "bat", "cmd", "ps1", "psm1",
        "py", "pyw", "rs", "c", "h", "cpp", "cc", "cxx", "hpp", "hh", "cs", "java", "kt", "kts",
        "go", "rb", "php", "lua", "sql", "r", "swift", "dart", "scala", "ex", "exs", "erl",
        "hs", "ml", "zig", "nim", "vue", "svelte", "astro", "glsl", "wgsl", "hlsl",
        "dockerfile", "makefile", "cmake", "gradle", "proto", "graphql", "gql",
    ],
};
