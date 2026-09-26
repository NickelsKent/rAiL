# Discovered Rules — rAiL Toolchain

## Mandated

- ALWAYS update the spec in the same change when a decision changes what the spec says.
- ALWAYS pass CI on both macOS and Linux before any merge to `main`.
- ALWAYS include a test that reproduces the defect with every defect fix.
- ALWAYS state a reason for every new Rust dependency and get the owner's approval before adding it.

## Forbidden

- NEVER lower an acceptance-suite pass criterion, its recorded CI volume, or a coverage floor to make a build pass.
- NEVER commit signing keys, model API keys or other secrets to the repository.
- NEVER put timestamps, absolute paths, hostnames or random seeds into compiler output.
- NEVER move or delete a release tag once it has been pushed.
- NEVER publish to a public package registry as part of this work.
