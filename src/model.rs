use serde::{Deserialize, Serialize};

/// Function or method definition discovered in source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: usize,
}

/// Syntax category for a resolved call expression.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallKind {
    Direct,
    Method,
    Associated,
    #[default]
    Unknown,
}

/// Resolved call expression between two discovered functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    pub caller: String,
    pub callee: String,
    pub file: String,
    pub line: usize,
    #[serde(default)]
    pub kind: CallKind,
}

/// Complete analysis result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    pub functions: Vec<Function>,
    pub calls: Vec<Call>,
}

#[cfg(test)]
mod tests {
    use super::{Analysis, Call, CallKind, Function};

    #[test]
    fn analysis_defaults_to_empty_graph() {
        let analysis = Analysis::default();
        assert!(analysis.functions.is_empty());
        assert!(analysis.calls.is_empty());
    }

    #[test]
    fn model_serializes_expected_fields() {
        let function = Function {
            id: "f0".to_string(),
            name: "main".to_string(),
            file: "main.go".to_string(),
            line: 3,
        };
        let json = serde_json::to_string(&function).unwrap();
        assert!(json.contains("\"id\":\"f0\""));
        assert!(json.contains("\"name\":\"main\""));
    }

    #[test]
    fn call_kind_serializes_as_lowercase_metadata() {
        let call = Call {
            caller: "f0".to_string(),
            callee: "f1".to_string(),
            file: "main.rs".to_string(),
            line: 3,
            kind: CallKind::Associated,
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("\"kind\":\"associated\""));
    }
}
