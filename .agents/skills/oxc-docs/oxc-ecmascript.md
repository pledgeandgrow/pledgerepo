# ECMAScript Specification & Grammar

Guide to reading the ECMAScript specification and understanding JavaScript grammar quirks
for building a parser in Rust.

---

## Specification

Reference: [ECMAScript Language Specification](https://tc39.es/ecma262/)

### Notational Conventions (Chapter 5.1.5)

#### Recursion

Lists are presented recursively:
```
ArgumentList :
    AssignmentExpression
    ArgumentList , AssignmentExpression
```

Means: `a, b = 1, c = 2` is parsed as nested `ArgumentList` + `AssignmentExpression`.

#### Optional (`_opt_` suffix)

```
VariableDeclaration :
    BindingIdentifier Initializer_opt
```

Means `Initializer` is optional — both `var x;` and `var x = 1;` are valid.

#### Parameters (`[Return]`, `[In]`, etc.)

```
ScriptBody : StatementList[~Yield, ~Await, ~Return]
```
`~` means the parameter is false — top-level yield, await, return not allowed in scripts.

```
ModuleItem : StatementListItem[~Yield, +Await, ~Return]
```
`+` means the parameter is true — top-level await IS allowed in modules.

### Source Text (Chapter 11.2)

- **Script Code**: Not strict by default; `"use strict"` directive enables strict mode
- **Module Code**: Automatically strict mode
- HTML: `<script src="...">` (script) vs `<script type="module" src="...">` (module)

### Automatic Semicolon Insertion (ASI)

Rules for omitting semicolons. Implementation:

```rust
pub fn asi(&mut self) -> Result<()> {
    if self.eat(Kind::Semicolon) || self.can_insert_semicolon() {
        return Ok(());
    }
    let range = self.prev_node_end..self.cur_token().start;
    Err(SyntaxError::AutoSemicolonInsertion(range.into()))
}

pub const fn can_insert_semicolon(&self) -> bool {
    self.cur_token().is_on_new_line
        || matches!(self.cur_kind(), Kind::RCurly | Kind::Eof)
}
```

Call `asi()` manually where applicable (e.g., end of statements).

---

## Grammar Quirks

JavaScript does NOT have a nice LL(1) grammar. The only practical approach is a hand-written recursive descent parser.

### Identifiers (3 types)

```
IdentifierReference[Yield, Await] :
BindingIdentifier[Yield, Await] :
LabelIdentifier[Yield, Await] :
```

In `var foo = bar`:
- `foo` is a **BindingIdentifier** (declaration)
- `bar` is an **IdentifierReference** (usage)

Declaring these separately in the AST simplifies downstream tools:

```rust
pub struct BindingIdentifier { pub node: Node, pub name: Atom }
pub struct IdentifierReference { pub node: Node, pub name: Atom }
```

### Class and Strict Mode

Classes are always strict mode. Need extra parser state since class declarations don't have a scope.

### Legacy Octal and Use Strict

`"\01"` is a legacy octal escape — syntax error in strict mode. If `"use strict"` appears in a function body with non-simple parameters, it's a syntax error.

### Non-simple Parameter and Strict Mode

```js
function foo(value = (function() { return "\01"; }())) {
    "use strict";
    return value;
}
```
This is a syntax error: `FunctionBodyContainsUseStrict` is true but `IsSimpleParameterList` is false.

### Parenthesized Expression

`((x))` should semantically be just `IdentifierReference` — parens have no meaning. But `(fn) = function() {}` changes `fn.name` at runtime. acorn/babel added `preserveParens` option for compatibility.

### Function Declaration in If Statement

Annex B allows function declarations in if-statement positions (non-strict mode):
```js
if (x) { function foo() {} } else function bar() {}
```

### Label Statements

Labelled statements are legit in modern JavaScript:
```jsx
<Foo bar={() => { baz: "quaz"; }} />
//           ^^^^^^^^^^^ LabelledStatement
```

### `let` is not a keyword

`let` can appear anywhere unless grammar explicitly forbids it:
```js
let a; let = foo; let instanceof x; let + 1; while (true) let; a = let[0];
```
Parser must peek at the token after `let` to decide what to parse.

### For-in / For-of and the [In] context

Two obstacles: `[lookahead ≠ let]` and `[+In]`:
- `for (let` — check if next is `in` (disallowed), or `{`/`[`/identifier (allowed)
- `[+In]` context — the `in` operator in relational expressions is disambiguated from `for-in`
- `[lookahead ∉ { let, async of }]` — forbids `for (async of ...)`

This is the **only** application of `[In]` in the entire specification.

### Block-Level Function Declarations

In Annex B.3.2, `FunctionDeclaration` in a block is treated as `var` if inside a function scope:
```js
function foo() { var bar; function bar() {} } // OK - same scope
function foo() { if (true) { var bar; function bar() {} } } // Error - redeclaration
```

### Grammar Context (5 parameters)

`[In]`, `[Return]`, `[Yield]`, `[Await]`, `[Default]` — track with bitflags during parsing.

### AssignmentPattern vs BindingPattern

estree uses a generic `Pattern` for both, but the spec distinguishes:
- **AssignmentExpression** `{ foo } = bar` → `IdentifierReference`
- **VariableDeclarator** `var { foo } = bar` → `BindingIdentifier`

Using a generic `Pattern` creates ambiguity. Better to follow the spec and use separate types.

### Cover Grammar

Three cover grammars in the spec:

#### 1. CoverParenthesizedExpressionAndArrowParameterList

```js
let foo = (a, b, c);       // SequenceExpression
let bar = (a, b, c) => {}; // ArrowExpression
//          ^^^^^^^^^ CoverParenthesizedExpressionAndArrowParameterList
```

Parse as `Vec<Expression>` first, then convert to `ArrowParameters` if `=>` is found. Each `Expression` must be converted to `BindingPattern`.

If building scope tree during parsing, create a temporary scope and drop it if not an arrow function (esbuild's approach: `popAndDiscardScope()` or `popAndFlattenScope()`).

#### 2. CoverCallExpressionAndAsyncArrowHead

```js
async (a, b, c);       // CallExpression (async is function name)
async (a, b, c) => {}; // AsyncArrowFunction
//     ^^^^^^^^^^^^^^^ CoverCallExpressionAndAsyncArrowHead
```

`async` is not a keyword — the first `async` is a function name.

#### 3. CoverInitializedName

```js
({ prop = value } = {}); // ObjectAssignmentPattern
({ prop: value });       // ObjectLiteral with SyntaxError
```

Parser must parse `ObjectLiteral` with `CoverInitializedName`, throw syntax error if it doesn't reach `=` for `ObjectAssignmentPattern`.
