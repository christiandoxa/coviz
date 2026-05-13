//! Core library for analyzing source calls and rendering graph output.

pub mod analyzer;
pub mod language;
pub mod model;
pub mod render;

pub use analyzer::analyze_path;
pub use language::Language;
pub use model::{Analysis, Call, Function};
pub use render::{render_dot, render_html, render_json};
