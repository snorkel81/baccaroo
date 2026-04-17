# Baccaroo

Secure, server-authoritative internet gaming framework written in Rust. Baccaroo provides an abstraction layer for card and table games, with Blackjack as the first implementation.

## Design Principles

Every design decision is driven by the 15 security rules defined in [DesignRules.md](DesignRules.md):

- **Server owns reality** -- the client sends intent, never state
- **Finite State Machine** enforcement for every game session
- **Atomic and idempotent** actions with unique action IDs
- **Deterministic engine** backed by an append-only event log
- **Cryptographic RNG** with commit-reveal for provable fairness
- **Strict concurrency control** -- one action per session at a time
- **Explicit trust boundary** -- all client input is validated
- **Observability** -- every action is logged with timing

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Start the server (development)
cargo run
```

The server starts on `http://localhost:3000`. Open it in a browser to play the test UI.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/session/new` | Start a new game session |
| `POST` | `/api/session/{id}/bet` | Place a bet |
| `POST` | `/api/session/{id}/action` | Submit a game action (hit/stand/double) |
| `GET`  | `/api/session/{id}/state` | Get current visible game state |
| `GET`  | `/api/session/{id}/reveal` | Get RNG seed reveal (after game ends) |
| `GET`  | `/api/balance/{player_id}` | Check player balance |

All API requests require an `X-Player-Id` header (placeholder auth).

## Project Structure

```
src/
  main.rs                  # Server startup
  lib.rs                   # Module re-exports
  config.rs                # Application configuration
  api/                     # HTTP routes, DTOs, middleware
  game/
    traits.rs              # GameEngine trait, GamePhase FSM
    state_machine.rs       # FSM transition validation
    session.rs             # Session management, concurrency
    blackjack/             # Blackjack implementation
  core/
    events.rs              # Event types, in-memory event store
    wallet.rs              # Balance system (event-sourced)
    rng.rs                 # CSPRNG with commit-reveal
    idempotency.rs         # Action ID deduplication
  risk/                    # Risk detection stubs
  observability/           # Structured logging, metrics stubs
static/                    # Web UI for testing
tests/integration/         # Integration and adversarial tests
```

## Architecture

The framework is designed so that adding new games (baccarat, roulette, etc.) requires only implementing the `GameEngine` trait. The session management, API layer, wallet, event store, and security infrastructure are shared.

## License

MIT
