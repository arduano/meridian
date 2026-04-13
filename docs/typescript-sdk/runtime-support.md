# Runtime Support

This SDK uses Deno for the checked examples and tests in this repo, but the
underlying design is not Deno-only.

## Current Posture

The repository already contains:

- a Deno runtime adapter
- a Node runtime adapter
- a Bun runtime adapter
- runtime-specific helper factories for high-level and protocol clients

What is Deno-specific today is mostly the validation posture in this repo:

- examples are written for Deno first
- package scripts use `deno check`
- tests use `deno test`

That makes Deno the clearest repo-local path, not the only intended runtime.

## Runtime Matrix

| Runtime        | Status in docs | Source support | Notes                                                  |
| -------------- | -------------- | -------------- | ------------------------------------------------------ |
| Deno           | Primary        | Yes            | Main documented and checked path in this repo          |
| Node           | Secondary      | Yes            | Adapter exists; package surface is designed for it     |
| Bun            | Secondary      | Yes            | Adapter exists; package surface is designed for it     |
| Other runtimes | Theoretical    | Not yet        | Feasible if they can satisfy the same adapter contract |

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

For the quickest start, use the package-level examples in
[`sdk/typescript/README.md`](../../sdk/typescript/README.md) and then swap only
the runtime-specific client factory if you are not on Deno.
