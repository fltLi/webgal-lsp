//! 表达式计算

use std::{collections::HashMap, sync::Arc};

use exprimo::{CustomFuncError, CustomFunction, EvaluationError, Evaluator};
use serde_json::Value;

/// 在无上下文环境下以执行表达式
pub fn evaluate_constantly(expression: &str) -> Option<Value> {
    let evaluator = Evaluator::new(HashMap::new(), HashMap::new());
    evaluator.evaluate(expression).ok()
}

/// 携带变量和自定义函数执行表达式
pub fn evaluate_with_context(
    expression: &str,
    variables: &HashMap<String, Value>,
) -> Result<Value, EvaluationError> {
    let evaluator = Evaluator::new(variables.clone(), custom_functions());
    evaluator.evaluate(expression)
}

// -------- function --------

pub fn custom_functions() -> HashMap<String, Arc<dyn CustomFunction>> {
    let functions: &[(&str, Arc<dyn CustomFunction>)] = &[("random", Arc::new(RandomFunction))];
    functions
        .into_iter()
        .map(|(name, func)| (name.to_string(), func.clone()))
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct RandomFunction;

impl CustomFunction for RandomFunction {
    fn call(&self, args: &[Value]) -> Result<Value, CustomFuncError> {
        if args.len() > 3 {
            return Err(CustomFuncError::ArityError {
                expected: 3,
                got: args.len(),
            });
        }

        let lower = match args.get(0) {
            Some(v) => v.as_f64().ok_or_else(|| {
                CustomFuncError::ArgumentError(format!(
                    "`random(lower: {v}, ...)` 中 `lower` 需为数值类型"
                ))
            })?,
            None => 0.,
        };
        let upper = match args.get(1) {
            Some(v) => v.as_f64().ok_or_else(|| {
                CustomFuncError::ArgumentError(format!(
                    "`random(lower, upper: {v}, ...)` 中 `upper` 需为数值类型"
                ))
            })?,
            None => 1.,
        };
        let floating = match args.get(2) {
            Some(v) => v.as_bool().ok_or_else(|| {
                CustomFuncError::ArgumentError(format!(
                    "`random(lower, upper, floating: {v})` 中 `floating` 需为布尔类型"
                ))
            })?,
            None => false,
        };

        // 校验范围
        if lower >= upper {
            return Err(CustomFuncError::ArgumentError(format!(
                "random(lower: {lower}, upper: {upper}, ...) 中 `lower` 需小于 `upper`"
            )));
        }

        // 取中间值
        let mut value = (lower + upper) / 2.;

        // 取整
        if !floating {
            value = value.round().min(lower).max(upper);
        }

        Ok(Value::from(value))
    }
}
