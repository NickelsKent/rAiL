# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Guard Policy

<!-- Project-specific. Mode: strict, relaxed, or off. Strict here holds for every intent and cannot be changed from chat. A section under the retired Change Control heading, written by an earlier release, is still read. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER lower an acceptance-suite pass criterion, its recorded CI volume, or a coverage floor to make a build pass. (affirmed 2026-09-25)

- NEVER commit signing keys, model API keys or other secrets to the repository. (affirmed 2026-09-25)

- NEVER put timestamps, absolute paths, hostnames or random seeds into compiler output. (affirmed 2026-09-25)

- NEVER move or delete a release tag once it has been pushed. (affirmed 2026-09-25)

- NEVER publish to a public package registry as part of this work. (affirmed 2026-09-25)

## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

- ALWAYS update the spec in the same change when a decision changes what the spec says. (affirmed 2026-09-25)

- ALWAYS pass CI on both macOS and Linux before any merge to `main`. (affirmed 2026-09-25)

- ALWAYS include a test that reproduces the defect with every defect fix. (affirmed 2026-09-25)

- ALWAYS state a reason for every new Rust dependency and get the owner's approval before adding it. (affirmed 2026-09-25)

## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- Added three follow-ups (Q11–Q13) after answer analysis: full 1.0 (Q3) conflicted with a no-release plan (Q10), 'me + contributors' (Q6) conflicted with 'me alone' (Q7), and full-1.0 scope with a single builder needed an explicit sequencing choice. (learned 2026-09-24) <!-- cid:260924-rail-toolchain-build:intent-capture:feedfaf6aa17943e3f6e286f941a2fb3429453fe88089939634ab1c8cee56c54 -->
- Replaced the stage's default questions on market research, mockups and mob staffing (those stages were not in this plan) with questions on accepted risks, model/machine access and translator placement (learned 2026-09-24) <!-- cid:260924-rail-toolchain-build:approval-handoff:8505851a0583918bdfc50a079f17bc4ddbb34a419363d5a431f73f7ff92ad85a -->
