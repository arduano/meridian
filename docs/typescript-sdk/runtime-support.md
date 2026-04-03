# Runtime Support

This SDK is documented as **Deno-centric** today, but the underlying design is
not fundamentally Deno-only.

## Current Posture

The repository already contains:

- a Deno runtime adapter
- a Node runtime adapter
- a Bun runtime adapter
- runtime-specific helper factories for high-level and protocol clients

What is Deno-centric right now is the surrounding documentation and validation:

- examples are written for Deno first
- package scripts use `deno check`
- tests use `deno test`

That makes Deno the clearest supported path today, not the only plausible one.

## Runtime Matrix

| Runtime | Status in docs | Source support | Notes |
| --- | --- | --- | --- |
| Deno | Primary | Yes | Main documented path today |
| Node | Secondary | Yes | Adapter exists; docs still need first-class walkthroughs |
| Bun | Secondary | Yes | Adapter exists; docs still need first-class walkthroughs |
| Other runtimes | Theoretical | Not yet | Feasible if they can satisfy the same adapter contract |

## What A Runtime Must Provide

At minimum, a compatible runtime needs to:

- spawn `meridian-cli`
- write request lines to stdin
- read response and event lines from stdout
- surface child-process exit and error events
- work with normal filesystem paths for inputs and outputs

That is the real portability boundary.

## Recommended Guidance For Now

- If you are starting fresh, use Deno.
- If your application already runs on Node or Bun, the repo layout suggests the
  SDK should fit there too.
- If you are documenting or testing another runtime, keep the initialization
  shape the same and swap only the runtime adapter and permissions model.

## Future Docs To Add

- dedicated Node getting-started page
- dedicated Bun getting-started page
- runtime compatibility table with tested versions
- custom runtime adapter guide
