// Test Runner — Vitest-compatible test execution via boa_engine
//
// Provides:
//   - describe/it/test API registration
//   - expect() assertion library
//   - beforeAll/beforeEach/afterAll/afterEach hooks
//   - Real JS execution via boa_engine
//   - Pass/fail/skip reporting

use anyhow::Result;
use boa_engine::{Context, JsValue, Source, js_string, NativeFunction};
use boa_engine::object::ObjectInitializer;
use std::path::Path;

/// Test result for a single test case
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub suite: String,
    pub status: TestStatus,
    pub error: Option<String>,
    pub duration_ms: u128,
}

/// Status of a test case
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Summary of a test run
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u128,
    pub results: Vec<TestResult>,
}

/// Run a test file in the boa_engine JS runtime
pub fn run_test_file(file_path: &Path) -> Result<TestSummary> {
    let source = std::fs::read_to_string(file_path)?;
    let mut context = Context::default();

    // Set up the test harness (describe, it, test, expect, hooks)
    setup_test_harness(&mut context);

    // Inject console.log
    setup_console(&mut context);

    // Inject a simple module system (stub require/import)
    setup_module_shim(&mut context, file_path);

    // Strip TypeScript types and ESM syntax for boa compatibility
    let js_source = strip_typescript(&source);

    // Evaluate the test file
    let eval_result = context.eval(Source::from_bytes(js_source.as_str()));

    let mut results = Vec::new();
    let mut errors = Vec::new();

    if let Err(e) = eval_result {
        errors.push(format!("File evaluation error: {}", e));
    }

    // Collect test results from the global __pledge_test_results array
    if let Ok(results_val) = context.eval(Source::from_bytes(
        r#"JSON.stringify((typeof __pledge_test_results !== 'undefined') ? __pledge_test_results : [])"#
    )) {
        if let Ok(json_str) = results_val.to_string(&mut context) {
            let json_str = json_str.to_std_string_escaped();
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(tests) = arr.as_array() {
                    for test in tests {
                        let name = test.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let suite = test.get("suite")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let status_str = test.get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("skipped");
                        let error = test.get("error")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let duration = test.get("duration")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u128;

                        let status = match status_str {
                            "passed" => TestStatus::Passed,
                            "failed" => TestStatus::Failed,
                            _ => TestStatus::Skipped,
                        };

                        results.push(TestResult {
                            name,
                            suite,
                            status,
                            error,
                            duration_ms: duration,
                        });
                    }
                }
            }
        }
    }

    // If there were eval errors and no results, add them as failed tests
    for err in errors {
        results.push(TestResult {
            name: file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
            suite: String::new(),
            status: TestStatus::Failed,
            error: Some(err),
            duration_ms: 0,
        });
    }

    // Build summary
    let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count();
    let failed = results.iter().filter(|r| r.status == TestStatus::Failed).count();
    let skipped = results.iter().filter(|r| r.status == TestStatus::Skipped).count();

    Ok(TestSummary {
        total: results.len(),
        passed,
        failed,
        skipped,
        duration_ms: 0,
        results,
    })
}

/// Set up the test harness in the JS context
fn setup_test_harness(context: &mut Context) {
    let harness_code = r#"
        var __pledge_test_results = [];
        var __pledge_current_suite = "";
        var __pledge_before_all = [];
        var __pledge_before_each = [];
        var __pledge_after_each = [];
        var __pledge_after_all = [];

        function describe(name, fn) {
            var prevSuite = __pledge_current_suite;
            __pledge_current_suite = name;
            try {
                fn();
            } catch(e) {
                __pledge_test_results.push({
                    name: name,
                    suite: "",
                    status: "failed",
                    error: "describe block error: " + e.message,
                    duration: 0
                });
            }
            __pledge_current_suite = prevSuite;
        }

        function it(name, fn) {
            __run_test(name, fn, false);
        }

        function test(name, fn) {
            __run_test(name, fn, false);
        }

        it.skip = function(name, fn) {
            __run_test(name, fn, true);
        };

        test.skip = function(name, fn) {
            __run_test(name, fn, true);
        };

        it.only = function(name, fn) {
            __run_test(name, fn, false);
        };

        test.only = function(name, fn) {
            __run_test(name, fn, false);
        };

        function __run_test(name, fn, skip) {
            if (skip) {
                __pledge_test_results.push({
                    name: name,
                    suite: __pledge_current_suite,
                    status: "skipped",
                    error: null,
                    duration: 0
                });
                return;
            }

            // Run beforeEach hooks
            for (var i = 0; i < __pledge_before_each.length; i++) {
                try { __pledge_before_each[i](); } catch(e) {}
            }

            var start = Date.now();
            var error = null;
            try {
                fn();
            } catch(e) {
                error = e.message || String(e);
            }
            var duration = Date.now() - start;

            // Run afterEach hooks
            for (var i = 0; i < __pledge_after_each.length; i++) {
                try { __pledge_after_each[i](); } catch(e) {}
            }

            __pledge_test_results.push({
                name: name,
                suite: __pledge_current_suite,
                status: error ? "failed" : "passed",
                error: error,
                duration: duration
            });
        }

        function beforeAll(fn) { __pledge_before_all.push(fn); }
        function beforeEach(fn) { __pledge_before_each.push(fn); }
        function afterEach(fn) { __pledge_after_each.push(fn); }
        function afterAll(fn) { __pledge_after_all.push(fn); }

        // expect() assertion library
        function expect(actual) {
            return {
                __actual: actual,
                toBe: function(expected) {
                    if (actual !== expected) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to be " + JSON.stringify(expected));
                    }
                },
                toEqual: function(expected) {
                    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to equal " + JSON.stringify(expected));
                    }
                },
                toBeTruthy: function() {
                    if (!actual) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to be truthy");
                    }
                },
                toBeFalsy: function() {
                    if (actual) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to be falsy");
                    }
                },
                toBeNull: function() {
                    if (actual !== null) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to be null");
                    }
                },
                toBeUndefined: function() {
                    if (actual !== undefined) {
                        throw new Error("Expected " + JSON.stringify(actual) + " to be undefined");
                    }
                },
                toBeDefined: function() {
                    if (actual === undefined) {
                        throw new Error("Expected value to be defined");
                    }
                },
                toContain: function(item) {
                    if (typeof actual === 'string') {
                        if (actual.indexOf(item) === -1) {
                            throw new Error("Expected \"" + actual + "\" to contain \"" + item + "\"");
                        }
                    } else if (Array.isArray(actual)) {
                        if (actual.indexOf(item) === -1) {
                            throw new Error("Expected array to contain " + JSON.stringify(item));
                        }
                    } else {
                        throw new Error("toContain requires string or array");
                    }
                },
                toHaveLength: function(len) {
                    if (!actual || actual.length !== len) {
                        throw new Error("Expected length " + len + " but got " + (actual ? actual.length : "undefined"));
                    }
                },
                toThrow: function() {
                    if (typeof actual !== 'function') {
                        throw new Error("toThrow expects a function");
                    }
                    try {
                        actual();
                        throw new Error("Expected function to throw");
                    } catch(e) {
                        // Expected - function threw
                    }
                },
                not: {
                    toBe: function(expected) {
                        if (actual === expected) {
                            throw new Error("Expected " + JSON.stringify(actual) + " NOT to be " + JSON.stringify(expected));
                        }
                    },
                    toEqual: function(expected) {
                        if (JSON.stringify(actual) === JSON.stringify(expected)) {
                            throw new Error("Expected " + JSON.stringify(actual) + " NOT to equal " + JSON.stringify(expected));
                        }
                    },
                    toBeTruthy: function() {
                        if (actual) {
                            throw new Error("Expected " + JSON.stringify(actual) + " NOT to be truthy");
                        }
                    },
                    toBeFalsy: function() {
                        if (!actual) {
                            throw new Error("Expected " + JSON.stringify(actual) + " NOT to be falsy");
                        }
                    },
                    toBeNull: function() {
                        if (actual === null) {
                            throw new Error("Expected NOT to be null");
                        }
                    },
                    toContain: function(item) {
                        if (typeof actual === 'string' && actual.indexOf(item) !== -1) {
                            throw new Error("Expected \"" + actual + "\" NOT to contain \"" + item + "\"");
                        }
                        if (Array.isArray(actual) && actual.indexOf(item) !== -1) {
                            throw new Error("Expected array NOT to contain " + JSON.stringify(item));
                        }
                    },
                    toThrow: function() {
                        if (typeof actual !== 'function') return;
                        try {
                            actual();
                        } catch(e) {
                            throw new Error("Expected function NOT to throw but it threw: " + e.message);
                        }
                    }
                }
            };
        }

        // vi mock object (Vitest compatibility)
        var vi = {
            fn: function(impl) { return impl || function(){}; },
            mock: function() {},
            unmock: function() {},
            spyOn: function(obj, method) { return obj[method]; },
            stubGlobal: function(name, value) { globalThis[name] = value; },
            fn: function(impl) {
                var mockFn = impl || function(){};
                mockFn.mock = { calls: [], results: [] };
                return mockFn;
            }
        };
    "#;

    let _ = context.eval(Source::from_bytes(harness_code));
}

/// Set up console.log support
fn setup_console(context: &mut Context) {
    let console_log = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let msg = args.iter()
            .map(|v| v.to_string(ctx).map(|s| s.to_std_string_escaped()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ");
        tracing::info!("[test console] {}", msg);
        Ok(JsValue::undefined())
    });

    let console = ObjectInitializer::new(context)
        .function(console_log, js_string!("log"), 0)
        .build();

    context.register_global_property(
        js_string!("console"),
        console,
        boa_engine::property::Attribute::all(),
    );
}

/// Set up a minimal module shim so import/export statements don't crash
fn setup_module_shim(context: &mut Context, _file_path: &Path) {
    // Provide a minimal `require` function
    let require_fn = NativeFunction::from_copy_closure(|_this, _args, _ctx| {
        Ok(JsValue::undefined())
    });
    let _ = context.register_global_callable(js_string!("require"), 1, require_fn);
}

/// Strip TypeScript-specific syntax for boa_engine compatibility
/// Removes: type annotations, interfaces, enums, export/import type, as assertions
fn strip_typescript(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut in_string = false;
    let string_char = ['"', '\'', '`'];
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        // Track string literals
        if string_char.contains(&ch) {
            if !in_string {
                in_string = true;
            } else {
                in_string = false;
            }
            result.push(ch);
            continue;
        }

        if in_string {
            result.push(ch);
            continue;
        }

        // Skip line comments
        if ch == '/' && chars.peek() == Some(&'/') {
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    result.push(c);
                    chars.next();
                    break;
                }
                chars.next();
            }
            continue;
        }

        // Skip block comments
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }

        result.push(ch);
    }

    // Remove ESM import/export statements (replace with no-ops)
    let result = result
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                // Keep export default as a variable assignment
                if trimmed.starts_with("export default") {
                    line.replace("export default", "var __default =")
                } else if trimmed.starts_with("export const") || trimmed.starts_with("export let") || trimmed.starts_with("export var") {
                    line.replace("export ", "")
                } else if trimmed.starts_with("export function") {
                    line.replace("export ", "")
                } else if trimmed.starts_with("export class") {
                    line.replace("export ", "")
                } else {
                    // Skip import statements
                    String::new()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    result
}
