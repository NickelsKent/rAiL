# Scalability Design — walking-skeleton (U1)

The skeleton serves one agent over one stdio connection. Its scalability design keeps the work of each request bounded and stops state from piling up across requests (NFR3.4, NFR4.6, NFR9.5).

## Sources

- `scalability-requirements.md`: NFR3.4, NFR4.6, NFR9.5.
- `security-requirements.md`: NFR4.4 (input limits).
- `functional-spec.md` for this unit: WF5 (protocol session loop).

## Request loop (NFR3.4)

```
loop:
  message = read_frame(stdin)          // bounded: 1 KiB header, 16 MiB body
  if end_of_input: exit 0
  response = handle(message)           // one operation, runs to completion
  if response is not None: write_frame(stdout, response); flush
```

- One thread reads, handles and writes. Each response is written and flushed before the next frame is read, so responses always come back in request order.
- The only extra threads are the two short-lived pipe readers of a program run (see `security-design.md`, boundary 4). They end when the program ends, before the response is written.

## Bounded work per request (NFR4.6)

| Resource | Bound | Mechanism |
|---|---|---|
| Message memory | 16 MiB body + 1 KiB header | Framing limits; oversized bodies are discarded in 64 KiB chunks |
| JSON structure | Depth 64 | Explicit depth counter in the parser |
| Source memory | 16 MiB per module | Bounded read |
| rAiL nesting | 256 levels | Explicit counter in the parser |
| Program time | 10 seconds | Run-operation time limit |
| Program output | 16 MiB captured | Capture cap, then kill |
| Build output | One executable and one object file per module | Fixed paths under `.rail/build/dev/` [Q1 of NFR design] |

## No state growth across requests (NFR9.5)

- The session keeps exactly two values: its state (`awaiting_initialize` or `ready`) and the canonical workspace root.
- Everything a request allocates (tree, types, IR, object code, captured output) is owned by that request's scope and freed when its response is written. There are no global caches, interners or registries that grow with requests.
- The rule registry and the operation table are fixed tables built once at start-up.

**Verification design.** A soak test starts `rail rap` as a child process and sends `initialize`, then 100 `check.run` requests on the `skeleton.answer` fixture, and records the child's resident memory. It then sends 900 more requests and records it again. On Linux it reads `VmRSS` from `/proc/<pid>/status`; on macOS it uses `ps -o rss= -p <pid>`. The test fails if the second reading exceeds the first by more than 10% or 1 MiB, whichever is larger.

## Deliberately absent

- Concurrent requests, cancellation and deadlines: U5.
- Caches keyed by `defhash`: U4 and U5.
- Parallel checking and building: U4 and U6.
