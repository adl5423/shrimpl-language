// src/tests.rs
//
// Built-in testing framework for Shrimpl.
// Test syntax in app.shr:
//
//   test "adds numbers":
//     add(1, 2) == 3
//
// Each indented line under `test` is an expression that must evaluate to
// a truthy value. A failing expression marks the test as failed.

use crate::interpreter::eval;
use crate::parser::ast::{Expr, Program, TestCase};
use crate::value::ValueRuntime;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub name: String,
    pub passed: bool,
    pub assertions: usize,
    pub failures: Vec<String>,
}

fn value_truthy(v: &ValueRuntime) -> bool {
    match v {
        ValueRuntime::Bool(b) => *b,
        ValueRuntime::Number(n) => *n != 0.0,
        ValueRuntime::Str(s) => !s.is_empty(),
        ValueRuntime::Json(_) => true,
        ValueRuntime::Null => false,
    }
}

fn run_single_test(program: &Program, test: &TestCase) -> TestOutcome {
    let mut failures = Vec::new();
    let mut count = 0usize;

    for (idx, expr) in test.assertions.iter().enumerate() {
        count += 1;
        let vars = HashMap::<String, String>::new();
        match eval::eval_body_value(expr, program, &vars) {
            Ok(val) => {
                if !value_truthy(&val) {
                    failures.push(format!("assertion {} evaluated to false", idx + 1));
                }
            }
            Err(err) => {
                failures.push(format!("assertion {} raised error: {}", idx + 1, err));
            }
        }
    }

    TestOutcome {
        name: test.name.clone(),
        passed: failures.is_empty(),
        assertions: count,
        failures,
    }
}

/// Run all tests in the program. Returns vector of outcomes.
pub fn run_all_tests(program: &Program) -> Vec<TestOutcome> {
    program.tests.iter().map(|t| run_single_test(program, t)).collect()
}
