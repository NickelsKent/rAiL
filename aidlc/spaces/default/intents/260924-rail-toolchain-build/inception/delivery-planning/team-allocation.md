# Team Allocation — rAiL Toolchain

A **Bolt** is one build pass over one unit of work that ends in something that runs and is tested. The 17 Bolts are listed in `bolt-plan.md`. In a larger project, a **mob** (a small group working together on one piece of work) owns each Bolt. This project has one builder, so there is one mob.

Team Formation was not part of this plan. Every Bolt is therefore built by the AI developer agent (`aidlc-developer-agent`), working in this session, with the project owner as the only human. The owner approves each plan, each pull request (after the automated AI review), and each unit checkpoint. The owner also signs off the walking-skeleton checkpoint: the first Bolt's thin end-to-end slice. Contributors may join later; if they do, this allocation is revisited.

| Bolt | Unit | Built by | Approved by | Specialist input during design stages |
|---|---|---|---|---|
| 1 | walking-skeleton (U1) | aidlc-developer-agent (AI) | Owner (skeleton checkpoint) | Architect |
| 2 | acceptance-harness (U2) | aidlc-developer-agent (AI) | Owner | Quality engineer |
| 3 | benchmark-corpus (U16) | aidlc-developer-agent (AI) | Owner | Quality engineer (fairness review) |
| 4 | syntax (U3) | aidlc-developer-agent (AI) | Owner | Architect |
| 5 | checker-and-lints (U4) | aidlc-developer-agent (AI) | Owner | Architect |
| 6 | agent-loop (U5) | aidlc-developer-agent (AI) | Owner | Architect, security engineer |
| 7 | execution-core (U6) | aidlc-developer-agent (AI) | Owner | Architect, security engineer (runtime memory safety) |
| 8 | release-backend (U12) | aidlc-developer-agent (AI) | Owner | Architect |
| 9 | system-and-stdlib (U7) | aidlc-developer-agent (AI) | Owner | Security engineer (capabilities) |
| 10 | measurement-tools (U17) | aidlc-developer-agent (AI) | Owner | Quality engineer |
| 11 | concurrency-and-system-std (U8) | aidlc-developer-agent (AI) | Owner | Architect |
| 12 | crypto-and-net (U9) | aidlc-developer-agent (AI) | Owner | Security engineer |
| 13 | supply-chain (U10) | aidlc-developer-agent (AI) | Owner | Security engineer, compliance specialist (licensing) |
| 14 | first-party-packages (U11) | aidlc-developer-agent (AI) | Owner | Developer |
| 15 | deep-analysis (U13) | aidlc-developer-agent (AI) | Owner | Security engineer, quality engineer |
| 16 | debugger (U14) — may slip | aidlc-developer-agent (AI) | Owner | Designer (debugging experience) |
| 17 | wasm-sandbox (U15) — may slip | aidlc-developer-agent (AI) | Owner | Security engineer |

"Specialist input" is the expert perspective the Construction design stages bring in for that unit. It is not a separate owner.

## Assumptions & Open Questions

- [assumption] One mob means Bolts never run in parallel. This matches the serial execution setting.
- Open question: when contributors join, whether some Bolts (for example first-party packages or the debugger) move to them.
