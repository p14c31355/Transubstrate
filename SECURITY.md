# Security Policy

## Scope

Transubstrate runs simulations and (eventually) local model inference and
world-boundary adapters. The WABIX capability model is explicitly designed so
that cognition backends can only act through typed, validated capabilities —
never free-form output execution.

## Reporting

Open a private security advisory via GitHub, or contact the maintainer
through the repository's issue tracker for initial contact.

## Hard rules for contributors

- Never commit credentials, tokens, or `.env` files.
- Never let model output execute as shell commands or raw hardware ops.
- Simulation worlds must not read the network or filesystem during `step`.
