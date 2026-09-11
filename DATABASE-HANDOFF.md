# Existing database handoff

The project must begin from the structure of the existing working SQLite database.

## When the first agent asks for the database

From the repository root:

```sh
mkdir -p .local
cp /path/to/your/current/database.sqlite .local/current-gtd.sqlite
```

Then tell the agent:

> The existing database is available at `.local/current-gtd.sqlite`. Treat it as read-only and continue the database compatibility analysis.

The agent is instructed not to modify the original file and not to copy real row contents into committed documentation.

`.local/` is excluded by `.gitignore` and must remain private.

## Why this happens after startup

The first agent can verify the toolchain and understand the architectural goals before seeing the database, but it is forbidden from making durable GTD schema/CRDT persistence decisions until it has inspected the real database. This prevents a greenfield schema from accidentally becoming the assumed architecture.
