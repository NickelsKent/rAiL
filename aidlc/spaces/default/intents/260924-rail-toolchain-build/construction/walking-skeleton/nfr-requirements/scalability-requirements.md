# Scalability Requirements — walking-skeleton (U1)

The skeleton is a local, single-user command-line tool and stdio server. It has no fleet, no traffic and no data growth, so "scalability" here means **bounded work per request**: one request can never grow without limit and take the process down.

## Sources

- `requirements.md`: NFR1 (the scale targets that belong to later units), FR13.1, FR27.
- `functional-spec.md` for this unit: WF5 (one session per process), WF1–WF4.
- `security-requirements.md` for this unit: the input limits (NFR4.4) and run time limit (NFR5.5).

## Load model

| Dimension | Skeleton value | Growth |
|---|---|---|
| Protocol sessions per server process | 1 | Fixed by design (stdio) |
| Requests in flight per session | 1: requests are handled one at a time, in arrival order | Concurrency arrives with U5 (cancellation, deadlines) |
| Modules per request | 1 | Workspaces with many modules arrive with U4 |
| Definitions per module | A handful (fixtures) | Bounded by the source-size limit |
| Program runs | One child process at a time | — |

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR3.4 | Requests in one session are processed one at a time, in arrival order, and each response is written before the next request is read. Responses therefore arrive in request order. | Protocol test sending several requests in one write |
| NFR4.6 | Memory and time used by one request are bounded by the input limits: 16 MiB per message and per source file, a JSON nesting depth of 64, a rAiL nesting depth of 256, and a 10-second program run. Over-limit input is rejected, never processed. | Boundary tests (see NFR4.4) |
| NFR9.5 | The server holds no state that grows across requests, apart from the session's workspace root. A long session uses the same memory after a thousand identical requests as after one, within normal allocator variation. | Soak-style test: 1,000 `check.run` requests in one session, compared against the first |

## Explicit non-requirements

- No parallel checking or building (U4, U6).
- No caching across requests (U4, U5).
- No scaling targets from §11.2 (the 10 K- and 100 K-line build and check targets belong to U4, U5, U6 and U12).
