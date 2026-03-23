# reqtool — Architecture & Design Decisions

This document captures the requirements, constraints, and design decisions reached
through architectural review. It is intended as a reference for contributors and
agents working on this codebase.

---

## Context

`reqtool` is a requirements specification compiler and analyzer. It parses `.req`
source files into an AST, runs multi-pass analysis (reference resolution, type
inference, traceability verification, etc.), and produces a resolved view consumed
by a Tauri desktop application (and potentially other clients).

---

## Hard Constraints

| Constraint | Detail |
|---|---|
| Scale | Single file, up to 10 000 nodes in practice |
| Throughput | Parse + analysis + serialization must sustain ~30fps at 10k nodes |
| Bottleneck | Tauri IPC (webview freeze on large JSON payloads) — not compute |
| Node order | Within-sibling parse order must be preserved throughout |
| Single file | No multi-file / import graph analysis for now |

---

## Problems with the Current Architecture

1. **NodeId is a global counter** incremented across the process lifetime. It is
   non-stable across edits (inserting a node shifts all subsequent IDs) and leaks
   into the public DTO, forcing clients to maintain NodeId-keyed hashmaps.

2. **ReferenceId** is a second global counter serving a similar purpose for
   reference expressions. Same problems.

3. **Analysis is a bag of `HashMap<NodeId, X>`**. Every piece of enriched data
   requires a hashmap lookup with an unwrap. Visitors must carry `&Analysis`
   everywhere and manually chain lookups.

4. **serde and ts_rs derives on internal syntax types** (`Entity`, `Expression`,
   etc.) couple the core library to serialization concerns and expose internals
   in generated TypeScript bindings.

5. **Full re-serialization on every edit** causes the IPC freeze. The client
   currently receives the entire analysis on every mutation.

6. **Rename requires a manual find-and-replace visitor** because
   `Reference.value` stores a raw path string that must be updated at all sites.

---

## Design Decisions

### 1. Stateless processing flow

The library is stateless. Every analysis run is:

```
parse(source) → Analysis::from(&root) → ResolvedView::from(&root, &analysis)
```

`ResolvedView` is a fully owned value with no lifetime. `Entity` and `Analysis`
are dropped after the view is built. The Tauri app sends the full view on initial
load and diffs subsequent views client-side using the persistent ID (PID) it
manages independently.

**Rationale:** parse + analysis at ~100K nodes/s is fast enough for 10k nodes
with a debounce. The stateful in-memory AST buys negligible compute savings
while adding lifetime and synchronization complexity.

---

### 2. NodeId and ReferenceId are eliminated

`NodeId` and `ReferenceId` are deleted from the codebase entirely.

- `Entity` — `id: NodeId` field removed.
- `Expression` — `id: NodeId` field removed.
- `Reference` — `id: NodeId` and `rid: ReferenceId` fields removed. Becomes
  `{ value: String }` only.

**Rationale:** NodeId's only function was as a hashmap key in analysis maps and
as an issue anchor. Both are replaced by better alternatives (see below).

---

### 3. Pointer-based identity in internal analysis

Analysis maps use pointer identity as the key. A safe zero-cost wrapper:

```rust
#[derive(Copy, Clone)]
struct NodeKey<'a>(&'a Entity);

impl Hash for NodeKey<'_> {
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self.0 as *const Entity as usize).hash(s);
    }
}
impl PartialEq for NodeKey<'_> { fn eq(&self, o: &Self) -> bool { std::ptr::eq(self.0, o.0) } }
impl Eq for NodeKey<'_> {}
```

Same pattern for `ExprKey<'a>` (`&'a Expression`) and `RefKey<'a>` (`&'a Reference`).

`Analysis<'a>` carries `PhantomData<&'a Entity>` to tie its lifetime to the root
entity. Since the flow is always parse → analyze → project → drop, this lifetime
is always a local variable — no self-referential struct problem arises.

**Rationale:** O(1) hash, no global state, no ID generation, no unsafe counter.
`Entity` and `Expression` stay syntax-isomorphic (no identity fields). Structural
`PartialEq` (already excluding `id`) is unaffected.

---

### 4. Three-layer architecture

```
Layer 1 — Syntax          Entity, Expression, Reference
                          No serde, no ts_rs, no ID fields.
                          Owned by the parser. Immutable during analysis.

Layer 2 — Analysis        Analysis<'a> with NodeKey/ExprKey/RefKey maps.
                          Internal only. Never serialized.
                          Provides ResolvedNode<'a> view for later passes.

Layer 3 — View            ResolvedView (owned, Path-keyed, fully joined).
                          serde and ts_rs derives live here only.
                          This is the public API surface.
```

#### Layer 2.5 — ResolvedNode (internal ergonomic view)

```rust
pub struct ResolvedNode<'a> {
    entity: &'a Entity,
    analysis: &'a Analysis<'a>,
}
```

Provides methods like `.path()`, `.restriction() -> Option<ResolvedNode>`,
`.expr_type(expr) -> Option<&TypeKind>`, `.children()`. Used in analysis passes
that run after reference and type resolution. Zero new types, zero duplication of
`Expression` variants.

---

### 5. ResolvedView structure

```rust
pub struct ResolvedView {
    pub entities: IndexMap<Path, ResolvedEntity>,  // ordered, Path-keyed
    pub issues: Vec<ResolvedIssue>,                // separated for streaming
}

pub enum ResolvedEntity {
    Package(ResolvedPackage),
    Part(ResolvedPart),
    Requirement(ResolvedRequirement),
    Attribute(ResolvedAttribute),
}

pub struct ResolvedPackage {
    pub path: Path,
    pub meta: EntityMeta,
    pub imports: Vec<Path>,       // resolved
    pub children: Vec<Path>,      // ordered, direct children only
}

pub struct ResolvedRequirement {
    pub path: Path,
    pub meta: EntityMeta,
    pub restriction: Option<Path>,
    pub allocation: Option<Path>,
    pub traceability: Vec<ResolvedTraceLink>,
    pub forward: Vec<Path>,       // from SubjectMap
    pub backward: Vec<Path>,
    pub inhibitor: Option<TraceabilityInhibitor>,
    pub variant: ResolvedRequirementVariant,
}

pub struct ResolvedIssue {
    pub path: Path,               // issuing entity — no NodeId
    pub severity: IssueSeverity,
    pub kind: IssueKind,
}
```

Key properties:
- **No `NodeId` anywhere in the public API.**
- **Path is the primary key.** `IndexMap` preserves insertion (parse) order.
- **Children are `Vec<Path>`**, not nested `Vec<ResolvedEntity>`. Container nodes
  (Package, Part) list their direct children by path. The flat `IndexMap` is the
  lookup structure; the children vec is the ordered traversal index.
- **Issues are separated** from the entity map to allow streaming them after the
  main view is delivered (avoids IPC freeze from large error lists).
- **Issues carry `Path`** of the issuing node inline — client never needs a
  secondary lookup to display an error.

---

### 6. Persistent ID (PID) is a client concern

The Tauri app embeds a `# pid <n> #` tag in source files to maintain stable
identity across renames and re-parses. The library:
- Parses the tag as a regular `Tag { id: "pid", value: Some("...") }`.
- Does **not** assign, generate, or modify PIDs.
- Does **not** expose a `pid` concept in its types.

The client maps `PID → Path` after receiving `ResolvedView` and manages its own
diff/cache logic. The library stays PID-free and general-purpose.

---

### 7. Rename API

```rust
pub fn rename(root: &mut Entity, old_path: &Path, new_label: &str) -> Vec<Path>
```

Walks the tree in-memory, updates the target entity's label, and updates all
`Reference.value` strings that resolved to `old_path`. Returns the list of paths
that changed (for client cache invalidation).

**Does not touch source files.** The caller (Tauri app) serializes the mutated
tree back to source via the formatter.

---

### 8. Expression<T> — parameterized child type

To support multiple view types (type-annotated, bound-computed, etc.) without
duplicating the variant structure, `ExpressionVariant` and all its recursive
sub-types are parameterized over the child type T:

```rust
pub enum ExpressionVariant<T = Box<Expression>> {
    BinOp(BinOp<T>),
    UnaryOp(UnaryOp<T>),
    Forall(Forall<T>),
    // ... all recursive variants
    Identifier(Identifier),  // leaves: unchanged, no T
    Number(Number),
    Boolean(Boolean),
    Undefined,
}

pub struct BinOp<T = Box<Expression>> {
    pub operator: BinaryOperator,
    pub left: T,    // T carries its own indirection (Box or Idx)
    pub right: T,
}
```

`Expression` uses the default `T = Box<Expression>` — structurally identical to
today, no Box<T> inside structs (T itself is Box<Expression>).

Each view type is a **concrete struct** (not generic) with its own annotation
fields:

```rust
pub struct TypedExpression {
    pub type_kind: TypeKind,
    pub variant: ExpressionVariant<Box<TypedExpression>>,
}

pub struct BoundedExpression {
    pub type_kind: TypeKind,
    pub bounds: Option<AttributeBounds>,
    pub variant: ExpressionVariant<Box<BoundedExpression>>,
}
```

`ExpressionVariant` exposes a `map<U>(self, fn: T → U) → ExpressionVariant<U>`
functor method for transforming between view types.

#### Entity<T> for container children

`Package` and `Part` get the same treatment (low cost, 2 structs only):

```rust
pub struct Package<T = Entity> {
    pub imports: Vec<Reference>,
    pub children: Vec<T>,
}
```

In the view: `ResolvedPackage` uses `children: Vec<Path>` (effectively
`Package<Path>`).

---

### 9. Serde and ts_rs placement

**serde** is derived on `ExpressionVariant<T>` with an explicit bound:

```rust
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub enum ExpressionVariant<T = Box<Expression>> { ... }
```

`ExpressionVariant<Box<Expression>>` is `Serialize` only if `Expression: Serialize`
— which is never derived. Raw syntax types stay non-serializable.

**ts_rs** cannot be derived on a generic type without requiring `T: TS`, which
would contaminate `Expression`. The solution is a **newtype wrapper** in the view
module per concrete instantiation:

```rust
#[derive(Serialize, TS)]
pub struct TypedExprVariant(ExpressionVariant<Box<TypedExpression>>);
```

A real type definition is required (type aliases do not support `#[derive]`).
The newtype is built once during the projection pass via `From`.

All `serde` and `ts_rs` derives on internal syntax types (`Entity`, `Expression`,
`Reference`, etc.) are **removed**. They exist only on `ResolvedView` and its
constituent types.

---

### 10. Arena pattern — deferred

The arena / typed-index-store pattern was evaluated. It would benefit dense
expression→entity cross-referencing (expressions can reference many attributes
and parts via `Identifier` nodes). However, the added complexity (build-phase /
read-phase split, `RefCell` or unsafe, incompatibility with `Expression<T>` map
functor) is not justified for the current scale and use cases.

**Decision:** proceed without arena. Revisit if multi-pass graph traversal over
the expression+entity graph becomes a bottleneck.

---

## Open Questions

- **ts_rs `#[ts(concrete)]`**: check whether the current ts_rs version supports
  this attribute on generic types. If yes, the newtype wrapper (§9) can be
  replaced with a single annotation on `ExpressionVariant<T>`.
- **Issue streaming**: exact IPC mechanism for sending issues separately from the
  main `ResolvedView` (Tauri event vs. second command response) is not yet decided.
- **Incremental diff**: the client diffs `ResolvedView` snapshots by PID. The
  library does not compute diffs. Whether the library should expose a diff helper
  in the future is deferred.
