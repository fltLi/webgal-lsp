//! 变量系统

use std::collections::HashMap;

use expression::{EvaluationContext, EvaluationError, Value};

#[derive(Debug, Clone, Copy)]
pub struct VariableTable<'a>(pub &'a HashMap<String, Value>);

impl<'a> EvaluationContext for VariableTable<'a> {
    fn get_variable(&self, name: &str) -> Option<Value> {
        self.0.get(name).cloned()
    }

    fn call_function(
        &self,
        name: &str,
        _arguments: &[Value],
    ) -> Result<Value, expression::EvaluationError> {
        Err(EvaluationError::UnknownFunction(name.to_string()))
    }
}
