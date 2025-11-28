# Changelog

## [0.5.3] – 2025-11-28

### Language / syntax

- Added boolean literals and type:
  - `true` and `false` are now first-class values.
- Extended expression grammar with boolean and comparison operators:
  - Logical operators: `and`, `or`.
  - Comparison operators: `==`, `!=`, `<`, `<=`, `>`, `>=`.
  - Defined precedence so that:
    - `*` / `/` bind tighter than `+` / `-`.
    - Comparisons bind tighter than `and` / `or`.
    - `and` binds tighter than `or`.
- Introduced `if / elif / else` as an expression:
  - Syntax:
    ```shrimpl
    if cond1: expr1
    elif cond2: expr2
    else: expr3
    ```
  - Evaluates branches in order and returns the value of the first branch whose condition is truthy, or the `else` branch if provided. If no branch matches and there is no `else`, the result is an empty string.
- Added a safe, bounded loop expression:
  - Syntax:
    ```shrimpl
    repeat N times: expr
    ```
  - Evaluates the `count` expression once, floors it to an integer, and executes the body expression that many times.
  - Returns the value of the last iteration, or `""` if `N == 0`.
  - Includes a hard safety cap (10,000 iterations) to prevent runaway loops in teaching contexts.
- Defined truthiness rules used in conditionals and logical operators:
  - `Bool`: uses its value directly.
  - `Number`: `0.0` is false, anything else is true.
  - `String`: `""` is false, anything else is true.

### Interpreter / runtime

- Extended `ValueRuntime` to include booleans and implemented display formatting so booleans print as `true` / `false`.
- Implemented evaluation for new expression forms:
  - `Expr::Bool` is evaluated to `ValueRuntime::Bool`.
  - `Expr::If` walks each `(condition, body)` pair, using `as_bool` on the condition and evaluating the first matching body.
  - `Expr::Repeat` evaluates the count, checks non-negativity, enforces the iteration cap, and repeatedly evaluates the loop body.
- Introduced helper functions:
  - `as_bool` to map runtime values to truthiness (used by `if`, `and`, `or`).
  - Reused `as_number` to coerce values in arithmetic, comparisons, and repeat counts.
- Extended `eval_binary` to handle:
  - Boolean logic for `BinOp::And` and `BinOp::Or` using `as_bool`.
  - Numeric comparisons with clear error messages if operands are not numeric.
- Kept existing behavior where `+` between non-numeric values falls back to string concatenation, maintaining previous ergonomic patterns.

### Expression parser

- Extended tokenizer to recognize:
  - `==`, `!=`, `<`, `<=`, `>`, `>=`.
  - `:` as a significant token for `if`, `elif`, `else`, and `repeat` syntax.
- Added top-level recognizers for control-flow expressions:
  - When an expression starts with `if`, it is parsed as an `if / elif / else` expression.
  - When an expression starts with `repeat`, it is parsed as a `repeat N times: expr` loop expression.
- Implemented `parse_if_expr`:
  - Parses `if` condition and body, zero or more `elif` condition/body pairs, and optional `else` body.
  - Produces an `Expr::If { branches, else_branch }`.
- Implemented `parse_repeat_expr`:
  - Parses `repeat <expr> times: <expr>` into `Expr::Repeat { count, body }`.
  - Provides targeted error messages if `times` or `:` are missing.
- Wired in boolean / logical operators at the correct precedence levels:
  - `parse_or` and `parse_and` layers for `or` and `and` expressions.
  - `parse_comparison` layer for `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Ensured that the parser reports “Unexpected tokens after end of expression” if trailing tokens remain, preserving strict single-expression semantics.

### Diagnostics and API Studio

- Updated `collect_vars_expr` and diagnostics to account for new expression variants:
  - `Expr::Bool` is treated as a literal with no variable references.
  - Added arms for `Expr::If` and `Expr::Repeat` to keep matches exhaustive.
  - Left hooks in place to later traverse branches and loop bodies for variable usage analysis.
- Retained endpoint/function/method diagnostics:
  - Duplicate `(method, path)` endpoint detection.
  - Unused path parameters in endpoint bodies.
  - Unused parameters in functions and methods.
- API Studio UI (`/__shrimpl/ui`):
  - Modernized single-page HTML UI with:
    - Endpoint list and selector.
    - Request builder for path params and query strings.
    - Response viewer with status pill and JSON pretty-printing.
    - Code panel that fetches and syntax-highlights `app.shr`.
    - Diagnostics panel that renders static diagnostics (warnings/errors).
  - Syntax highlighting now covers:
    - Keywords: `server`, `func`, `class`, `endpoint`.
    - HTTP methods: `GET`, `POST`.
    - JSON keyword `json`.
    - Numbers, strings, and comments with distinct styles.

### CLI / UX

- Improved `shrimpl` `run` UX:
  - Running `shrimpl` or `shrimpl --file app.shr run` now prints a startup banner that includes:
    - The port configured in the `server` declaration.
    - Direct hints to open `http://localhost:<port>/__shrimpl/ui` to explore and test the API.
    - Clear instructions for shutting down the server with `Ctrl+C`.
- Kept `run` as the default behavior when no subcommand is provided.
- Maintained the `lsp` subcommand to launch `shrimpl-lsp`, wiring stdin/stdout/stderr through to support editor LSP integrations without additional setup.

### Sample app (`app.shr`)

- Extended the demo program to exercise new control-flow and boolean features:
  - `func classify_age(age)` using `if / elif / else` and numeric comparisons.
  - `func describe_adult_us(age, country)` combining comparisons with `and` / `or`.
  - `func flag_to_bool(flag)` mapping `"yes"` to `true` and other values to `false`.
  - `func repeat_greet(name, n)` using `repeat number(n) times: ...` to build repeated greetings.
- Added new endpoints showcasing these helpers:
  - `/age-category?age=...` → `"child" | "teen" | "adult"`.
  - `/adult-us?age=...&country=...` → `"adult-us" | "adult-non-us" | "not-adult"`.
  - `/flag?flag=...` → `true` / `false`.
  - `/repeat-greet?name=...&n=...` → repeated greeting string.
- Improved `/welcome` endpoint:
  - Uses an `if` expression to provide:
    - A generic message when `name` is empty.
    - A personalized uppercase greeting including name length when `name` is present.

### Internal fixes

- Resolved Rust non-exhaustive pattern warnings/errors by:
  - Updating `docs::collect_vars_expr` to handle all `Expr` variants, including `Bool`, `If`, and `Repeat`.
- Tightened error messaging across parser and evaluator paths to make syntax and runtime issues clearer for beginners.
