# NTM Tracker RPC Schema

JSON Schema definitions for the NTM Tracker RPC protocol. These schemas are the source of truth for both the Rust daemon and TypeScript frontend.

## Structure

```
schema/
├── rpc.json              # JSON-RPC 2.0 envelope definitions
├── errors.json           # Application error codes
├── types.json            # Shared data types (Session, Pane, Event, etc.)
├── methods/              # Per-method request/response schemas
│   ├── core.json         # health.get, capabilities.get, snapshot.get
│   ├── sessions.json     # sessions.list, sessions.get
│   ├── panes.json        # panes.get, panes.outputPreview
│   ├── events.json       # events.list, subscribe, escalations.*
│   ├── stats.json        # stats.summary, stats.hourly, stats.daily
│   ├── actions.json      # actions.sessionKill, actions.paneSend, attach.command
│   └── admin.json        # config.*, detectors.* (admin-only)
└── events/               # Push notification schemas
    └── notifications.json # Session, Pane, Event, Stats notifications
```

## Usage

### TypeScript (Frontend)

Generate types using `json-schema-to-typescript`:

```bash
cd app
npx json-schema-to-typescript --input ../shared/schema/types.json --output src/lib/generated/types.d.ts
```

Or run the generation script:

```bash
npm run generate:types
```

### Rust (Daemon)

Types can be generated using `typify` or `schemars`. Add to `daemon/build.rs`:

```rust
use typify::TypeSpace;
use std::fs;

fn main() {
    let schema = fs::read_to_string("../shared/schema/types.json").unwrap();
    // Generate Rust types from schema
}
```

## Schema Conventions

1. **Naming**: Use camelCase for all property names (matches JSON serialization)
2. **Timestamps**: Unix seconds as integers (`Timestamp` type)
3. **IDs**: Strings for entity IDs, integers for event cursors
4. **Nullability**: Use `required` array; optional fields are simply not required
5. **Enums**: Define as string enums with explicit values

## Adding New Methods

1. Add schema to appropriate file in `methods/`
2. Regenerate types for both Rust and TypeScript
3. Update RPC handler to match schema
4. Run schema validation in CI

## Validation

Validate a message against the schema:

```typescript
import Ajv from 'ajv';
import schema from '../shared/schema/types.json';

const ajv = new Ajv();
const validate = ajv.compile(schema.definitions.Session);
const valid = validate(sessionData);
```

## Version History

- Schema version 1: Initial release
