# Table Collections and Data Connection Containers

Status: proposed product and implementation requirements.

This document specifies two related canvas objects:

1. a **table collection** created by splitting one table by one or more
   columns in Wrangle; and
2. a **data connection container** that owns one database connection and can
   produce several connected tables.

They should share a dense tabbed presentation where that saves interface and
implementation work. They must not share a data model merely because their
cards look alike. A split collection is a dynamic, homogeneous dictionary of
tables governed by one pipeline. A connection container is a stable catalogue
of heterogeneous query results, each of which is an ordinary table with its
own pipeline.

The requirements below use **table** for the user-facing concept and **frame**
for the existing `FrameObject` model.

## Product purpose

### Split collections

A person should be able to take one table such as:

```text
Region  Month  Revenue  Cost
North   Jan    120      80
South   Jan    95       70
North   Feb    130      82
South   Feb    101      71
```

and choose **Split by Region** in Wrangle. The result is one object that reads
as a keyed dictionary:

```text
North -> table containing the North rows
South -> table containing the South rows
```

The person may tab through the tables, but edits the shared transformation
once. Adding `Profit = Revenue - Cost` applies it to North, South, and every
region that appears later. This is the reason for the feature: it replaces a
workbook full of copied regional sheets with one live definition.

### Data connection containers

A person should be able to add one database connection to the canvas, define
several named queries beneath it, and receive ordinary connected tables:

```text
Warehouse
  Orders
  Customers
  Monthly sales
```

Connection details and status belong in one place. Each resulting table keeps
its own schema, refresh recipe, Wrangle chain, lineage, and stable object and
column identities. The feature removes repeated connection setup; it does not
turn a database into an editable spreadsheet or make remote writes implicit.

## Shared interface principles

Both objects may use one reusable dense tabbed-container shell, but that shell
is a view component rather than a common domain type.

- The active table fills the card. Tabs spend one compact line above it.
- The card header shows the object name and a quiet count, for example
  `Sales by Region · 4 tables` or `Warehouse · 3 tables`.
- Up to a small number of members may appear as literal tabs. Larger
  collections use a searchable selector without mounting every member.
- Tabs must not each repeat refresh, save, delete, help, or status controls.
  Commands that apply to the object live once in its header or inspector.
- The selected tab is view state. Switching tabs must never change formulas,
  lineage, or the meaning of a reference.
- The card virtualizes rows exactly as an ordinary frame card does. It loads
  only the active member unless a later explicit comparison view asks for
  more.
- Errors appear on the tab or Wrangle row that owns them, as ordinary text.

Do not reuse `CanvasView.tabObjectIds` for either feature. Existing canvas tabs
mean alternate presentations of the same single-source subject and carry
strict lineage rules. These containers have their own member selection and
must not weaken those rules.

---

## Part I: table collections created by Split

### 1. Split is a Wrangle command

**Split by** is offered from Wrangle and from a selected column's contextual
actions. Both gestures perform the same canonical operation.

Applying Split creates a new table-collection object and leaves the source
frame unchanged. It does not change an existing frame pipeline from returning
a frame to returning a collection midway through that pipeline.

The collection's Wrangle view shows its derivation as a fixed first row, in
the same spirit as a joined table showing its relationship:

```text
1  Split Sales by Region
2  Sort by Month
3  Profit = Revenue - Cost
```

Steps after Split are the collection's shared member pipeline. They apply to
every member table. They are not copied into one pipeline per key.

### 2. Model

Add a first-class object kind, provisionally `FrameCollectionObject`:

```text
FrameCollectionObject
  id
  name
  sourceFrameId
  keyColumnIds[]
  steps[]
```

The exact Rust names may follow the existing model, but the following are
invariants:

- `sourceFrameId` names exactly one source frame.
- `keyColumnIds` is non-empty and contains stable column IDs from the source
  schema at the split boundary.
- Multi-column keys are supported in the model from the first version, even
  if the first pointing gesture starts with one column.
- `steps` is one shared Wrangle chain evaluated after the source is
  partitioned.
- A collection owns no literal rows and is always read-only.
- A collection is not a `ContainerObject`. Generic containers intentionally
  hold arranged values, lists, and nested containers, not frames.
- A collection may feed downstream objects. Dependency validation and cycle
  detection treat it as reading its source frame plus anything referenced by
  its shared steps.

At runtime the value is a typed dictionary:

```text
KeyTuple -> lazy member frame
```

Do not serialize it as a JSON object. Keys may be strings, numbers, dates,
booleans, nulls, or tuples, and JSON object keys would erase those distinctions.
Use the existing scalar representation for each key component and a stable
canonical encoding where view state needs to identify a member.

### 3. Members are virtual, not document objects

Do not mint one `FrameObject`, `CanvasView`, or operation-history entry for
every distinct key. Persist the collection definition; discover members from
the current source data.

This is deliberately data-dependent:

- a new key appearing upstream adds a member;
- a key disappearing upstream removes its member;
- refreshing a source may change the member count without rewriting the
  document model; and
- opening or refreshing the member index performs a full-data distinct-key
  query in the engine, never a scan of the rendered page.

Member identity is the typed key tuple, not the current ordinal and not a
new UUID. Display ordering is deterministic:

1. use declared categorical order when one exists;
2. otherwise sort by the key values with nulls last; and
3. for composite keys, compare components from left to right.

Null keys form a real member labelled **Blank**. A person who does not want
that member filters null keys before Split.

### 4. Member schema and rows

Every member begins with the same schema as the source at the split boundary.
The split key columns remain in each member. They are constant within that
member, but retaining them makes formulas, combining, lineage inspection, and
round-tripping unambiguous. A display option may hide constant split columns;
it must not remove them from the data plan.

The shared steps must produce the same declared schema for every member. A
step whose output schema depends independently on each member's values is not
allowed in the first version. In particular, a per-member pivot must either
use one output schema baked from the whole collection or fail with an
explanation; it must not give North and South incompatible columns.

Source steps run before Split. Collection steps run after each key filter:

```text
source frame data layer
  -> select rows for key
  -> collection shared steps
  -> active member display layer
```

Each member may have view-local scroll, selection, formatting, and display
sort/filter state. These are keyed view records owned by the collection view,
not independent data objects. Implementations may retain state only for
members the person has opened, so a high-cardinality collection does not
inflate the document.

### 5. Applying operations to a collection

Wrangle commands on a collection apply to every member by default. The UI says
this plainly at the point where ambiguity exists, for example **Calculate in
each Region table**.

The first version should support the ordinary row-preserving and
schema-stable frame operations:

- calculated columns;
- select/rename columns;
- filter;
- sort;
- summarize with a fixed output schema;
- joins against one shared lookup frame; and
- stable reshape operations whose output schema is known for the collection.

An implementation may compile compatible work as one grouped Polars plan
rather than executing one plan per key. That is an optimization only. The
observable semantics are the dictionary of independently transformed member
tables, and the engine must avoid rescanning a remote or artifact source once
per visible tab when one grouped plan can answer the request.

### 6. Combining and extracting

Two explicit escape hatches are required:

- **Combine tables** creates an ordinary derived frame by concatenating all
  members and retaining the split-key columns.
- **Extract this table** creates an ordinary derived frame filtered to the
  selected typed key. The new frame has its own Wrangle chain and may diverge.

The active tab itself is never an implicit formula reference. A later formula
surface may support an explicit address such as a collection plus a typed key,
but changing tabs must not retarget an existing formula.

### 7. Collection UI

The compact default presentation is:

```text
Sales by Region                                      4 tables
North  South  East  West
-------------------------------------------------------------
Region  Month  Revenue  Cost  Profit
North   Jan    120      80    40
...
```

Requirements:

- Creating the collection selects its first deterministic member.
- The selected key is stored in the canvas view, not the collection object.
- If the selected key disappears, select the next available member and show
  one inline explanation. An empty collection shows **No groups** and remains
  a valid object.
- Composite keys render compactly, for example `North · Actual`.
- The inspector exposes the source, key columns, member count, shared schema,
  and shared Wrangle chain.
- Comparison/small-multiple presentation is a later display mode. It must not
  be required to ship the collection model.
- High cardinality must remain usable. After the tab threshold, show a search
  control and count rather than shrinking hundreds of labels into the card.

### 8. Operations, history, and automation

Add canonical operations for at least:

- creating a collection from a source and key columns;
- changing its key columns;
- replacing its shared Wrangle chain;
- extracting one typed member as a frame; and
- combining the collection into a frame.

Names should follow the canonical `Operation` enum rather than a second UI or
MCP model. Every public operation must be discoverable through
`describe_operations` and accepted by `apply_operation`. Undo restores the
previous collection definition or removes the derived object it created; it
does not snapshot member data.

### 9. Collection acceptance criteria

The feature is not complete until tests prove all of the following:

- Splitting a frame with four regions produces four members from the complete
  dataset, including keys outside the first rendered page.
- String, numeric, date, null, and composite keys retain their types and have
  deterministic order.
- One shared calculated column appears in every member and updates after an
  upstream edit or connector refresh.
- Adding or removing a source key changes membership without minting or
  deleting document objects.
- Switching tabs changes only view state.
- Extracting one member produces a stable derived frame that no longer follows
  the collection's later shared-step edits.
- Combining members retains the split keys and reproduces the collection's
  rows without accidental duplication.
- Save/load, undo/redo, dependency deletion guards, and MCP operation parity
  cover the new object kind.
- A mounted interaction test proves the Split gesture emits the canonical
  operation and that tab selection is accessible by name.
- A native e2e workflow splits tutorial data, changes one shared Wrangle step,
  visits two members, edits the source, and observes the correct live result.

---

## Part II: data connection containers

### 10. Connection object and local profile

Add a first-class object kind, provisionally `DataConnectionObject`:

```text
DataConnectionObject
  id
  name
  profileId
  memberFrameIds[]
```

The document object is a portable reference and an organizational home. The
actual URI, credentials, executable details, and secrets remain in the
machine-local connection store, as they do today. Credentials must never enter
the `.fw` file, operation events, generated SQL diagnostics, logs, or MCP
responses.

`profileId` identifies the local grant the document expects. Opening the
document on another machine may yield **Connection not configured** and an
explicit mapping action. It must never silently choose another connection.

Each member is an ordinary `FrameObject` with:

- its own stable frame and column IDs;
- a database `ConnectorRecipe` containing the portable source name and query;
- its own cached artifact;
- its own Wrangle and display layers; and
- a reference to its owning connection object for organization and refresh.

This extends the current database connector rather than replacing its proven
artifact, refresh, schema-reconciliation, and ownership behavior. Existing
standalone connected frames remain valid and may be moved into a connection
container explicitly.

### 11. Connection membership and presentation

Unlike split members, connection members are real frames because their
schemas, queries, refresh histories, and downstream dependencies may differ.
Membership order is explicit and persisted on the connection object.

The connection card shows those frames as compact tabs:

```text
Warehouse                                             3 tables
Orders  Customers  Monthly sales  + Query
-------------------------------------------------------------
active connected frame
```

Requirements:

- **Add table/query** asks for a source name and SQL, stages the result, and
  creates one normal connected frame in the container.
- A member may be opened as another canvas view without cloning its data or
  giving it a second identity. Closing that view does not remove it from the
  connection.
- Removing a member from the connection does not delete a frame that has
  downstream dependants. The same dependency-safe deletion rules apply as for
  every other frame.
- Renaming the connection does not rename member frames or physical database
  sources.
- Each member has its own Wrangle chain. Editing Orders must not apply the
  same steps to Customers merely because they share a connection.
- Connection status is shown once in the container header. Query-specific
  failures remain on the affected member tab and table.

The generic `ContainerObject` remains unchanged. A connection container is a
purpose-built data object allowed to organize frames without relaxing the rule
that ordinary presentational containers cannot own frames.

### 12. Refresh behavior

Support both **Refresh table** and **Refresh connection**.

Refreshing one table follows the existing connector path: execute its stored
query through the locally configured profile, stage a new artifact, reconcile
physical source fields, and publish one artifact swap.

Refreshing the connection stages every requested member before publishing any
of them. The default all-members refresh is atomic at the document level:

- if every query succeeds, publish one prepared operation/event that swaps all
  artifacts;
- if any query fails, retain every previous artifact and identify the failed
  query; and
- if a member recipe changes while staging, reject the stale refresh rather
  than installing an answer to an old query.

An advanced action may refresh selected members independently. Partial refresh
must be explicit; it must not be the accidental result of a failed **Refresh
connection** command.

Cached artifacts remain the deterministic data read by the document. Merely
opening a document never contacts the database, executes shared SQL, or changes
the current answers.

### 13. SQL execution and query pushdown

There are three separate concerns and they must not be presented as one vague
promise of database integration.

#### 13.1 Stored source query

The first version persists the explicit read query for each member and runs it
when that member is created or refreshed. This is already meaningful
pushdown: the person may select, filter, join, and aggregate remotely in SQL
before rows cross the connection.

The UI must say that stored SQL is part of the document and may be visible to
collaborators. It must discourage secrets in SQL literals. A shared document
never auto-runs a query, and a newly mapped connection requires an explicit
first refresh.

Database profiles should be read-only by default. Where the backend permits
validation, reject multiple statements and non-read statements from the
ordinary query surface. Credential permissions remain the final security
boundary; SQL text inspection is not a substitute for a read-only grant.

#### 13.2 Automatic Wrangle pushdown

Automatic translation of Wrangle into SQL is a later optimization, not a
correctness requirement for the connection container. When added, it follows
these rules:

- Translate only a proven prefix of the chain: projections, compatible
  filters, limits, sorts, supported aggregates, and supported same-connection
  joins.
- Stop at the first unsupported or semantically different step and finish the
  remainder locally in Polars.
- Never translate a function merely because two backends use the same name;
  nulls, dates, rounding, collation, categories, and numeric overflow need
  conformance tests.
- Show the remote/local boundary in the query plan.
- Preserve the saved Wrangle program as the source of truth. Generated SQL is
  an execution detail, not a second editable pipeline.
- Produce the same declared schema and values as local execution within the
  documented backend tolerances.

Until that compiler exists, Wrangle always runs against the cached query
artifact. Do not hide an unbounded database pull behind a claim that a later
filter was pushed down.

#### 13.3 Database writeback or "push up"

Automatic writeback is out of scope. Connected frames remain read-only, with
calculated columns and transformations expressed in Wrangle rather than posted
into source rows.

A later **Publish to database** workflow may explicitly create or replace a
table or view. It must be a separate, confirmed operation with:

- an exact connection, schema, and target name;
- create/replace/append policy;
- generated SQL or upload plan preview;
- schema and row-count preview;
- a transaction where the backend supports one;
- an explicit write-capable local profile; and
- a recoverable failure that does not mutate the FrameWork source object.

Cell edits, refresh, save, and ordinary Wrangle changes must never trigger
database writes.

### 14. Connection operations and automation

Add canonical operations or commands for at least:

- adding/removing/renaming a connection object;
- adding an already connected frame to it;
- creating a member frame from a query;
- changing a member query through the existing source-replacement path;
- reordering and removing members; and
- atomically installing the artifacts from a connection refresh.

Network and credential work stays in the Tauri adapter. `framework-core`
receives staged artifacts and portable recipes through typed operations; it
does not open database connections. MCP exposes the same document operations
but must not reveal local credentials or gain an alternate write path.

### 15. Connection acceptance criteria

The feature is not complete until tests prove all of the following:

- One connection creates at least two member frames with different schemas
  and independent Wrangle chains.
- The `.fw` snapshot and operation events contain profile IDs and portable
  query recipes but no URI, password, token, or driver secret.
- Existing standalone database-connected frames still load, refresh, package,
  freeze, and take ownership exactly as before.
- Refreshing one member preserves surviving column IDs and reports missing
  referenced source fields through the existing reconciliation path.
- Refreshing the whole connection installs all new artifacts together or none
  of them when one query fails.
- Reordering and selecting tabs changes only connection/view state and does
  not rename or recompute member frames.
- A collaborator without the local profile sees **Connection not configured**
  and no query executes automatically.
- Dependency-safe deletion prevents removing a member that a formula, join,
  plot, or derivation still reads.
- Canonical operations have preparation, apply, invert, persistence, generated
  TypeScript bindings, and MCP catalog/parity coverage.
- Mounted interaction tests cover adding and switching members without a
  duplicate frontend data model.
- Connector integration tests cover multi-query staging and atomic failure.
  A native e2e workflow covers the real UI-to-core refresh seam using a safe
  test connection or fixture supported by the existing harness.

---

## Shared implementation sequence

Build the features in this order so the shared visual shell does not dictate
the core model:

1. Add the table-collection model, typed member keys, full-data member-index
   query, canonical operations, persistence, and engine tests.
2. Add the dense collection card and member selector, then the fixed Split row
   and shared Wrangle chain.
3. Add Extract and Combine, automation parity, and one native e2e path.
4. Extract the proven tabbed-frame shell without moving collection semantics
   into it.
5. Add the connection object over the existing connector recipes and artifact
   refresh path.
6. Add multi-query staging, atomic connection refresh, security tests, and the
   connection UI.
7. Consider automatic SQL pushdown only after the local semantics and query
   plan boundary have backend conformance tests.
8. Consider explicit database publishing separately; it is not part of either
   container's first release.

## Explicit non-goals for the first release

- No arbitrary nested table values inside ordinary frame cells.
- No independent copied pipeline per split member.
- No formula whose meaning changes with the selected tab.
- No per-member schema drift inside one split collection.
- No materializing every split key as a document object.
- No general-purpose node graph or cable editor required to use either object.
- No weakening of ordinary canvas-tab lineage rules.
- No credentials in documents.
- No query execution on document open.
- No automatic database writeback.
- No promise that every Wrangle expression can be translated to SQL.
