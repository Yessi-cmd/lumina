# AGENTS.md — Lumina

AI agent onboarding guide.

## Overview

Lumina is a Tauri 2 desktop companion for League of Legends.

- **Backend** (`src-tauri/`): Rust, tokio. Owns all state and all network access (LCU, SGP, Live Client Data).
- **Frontend** (`src/`): Vue 3 + Pinia + vue-router + Tailwind CSS v4. Presentation only.
- **Package manager**: pnpm. **Target platform**: Windows x64.

Architecture and milestones: `docs/ARCHITECTURE.md`. Read it before structural changes.

## Rules

- The frontend never talks to LCU/SGP directly and never sees auth tokens. It calls
  `invoke()` commands and listens to Tauri events.
- Rust `commands/` are thin wrappers; logic lives in `services/`, protocol code in `clients/`.
- Never read or write game memory or inject into game processes.
- Do not add a heavy UI component library; build small components with Tailwind.

## Verification

- Frontend: `pnpm build` (runs `vue-tsc` then `vite build`).
- Rust: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` in `src-tauri/`.
  The maintainer's machine may not have Rust; in that case CI (`.github/workflows/ci.yml`)
  is the source of truth — say so explicitly instead of claiming Rust code compiles.

## Commits

Scoped messages, e.g. `feat(lcu): discover client via lockfile`.
