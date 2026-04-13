# Getting Started

This page focuses on the parts that matter most early: what must exist on the
machine, how the SDK talks to Meridian, and how to initialize a client cleanly.

The examples here use Deno because it is the easiest repo-local path to run and
verify. The same initialization model carries over to Node, Bun, or any other
runtime that can satisfy the same subprocess and filesystem contract.

## How The SDK Works

The TypeScript SDK is a thin client around the `meridian-cli` executable. The
SDK does not embed the renderer or MIDI engine directly. Instead, it:

1. starts `meridian-cli`
2. speaks JSON over stdio
3. exposes a typed client on top of that protocol

That design is why initialization matters more than individual helper methods:
if the runtime can launch the CLI and the CLI has access to the files it needs,
most SDK flows follow naturally.

## Requirements

### 1. A Meridian CLI binary

You need a working `meridian-cli` executable. In local development that will
usually be one of these:

- a repo-built debug binary such as `target/debug/meridian-cli`
- a release binary you built separately
- a binary path supplied through your own app configuration

The SDK always needs the explicit executable path at client creation time.

### 2. A runtime with subprocess support

The SDK assumes the runtime can spawn a child process and keep stdin/stdout open
for request and event exchange.

Deno is the primary documented target today. Node and Bun are also represented
in the source tree and should follow the same model.

### 3. Filesystem access

Most SDK operations reference normal file paths:

- MIDI inputs
- output paths for processed MIDI
- output paths for audio renders
- output paths for video renders
- optional soundfont paths for audio work

Your runtime needs permission to read inputs and write outputs.

### 4. Extra external tools for specific workflows

Not every feature needs the same environment:

- MIDI analysis and MIDI processing primarily need the CLI binary and file
  access
- audio rendering also needs usable soundfont assets
- video rendering may depend on external video tooling such as `ffmpeg`, based
  on how `meridian-cli` is configured on the host

Those workflow-specific dependencies should be documented in detail later. For
now, treat them as environment prerequisites rather than SDK prerequisites.

## Deno Permissions

If you use Deno, expect to grant permissions for the things the SDK actually
does:

```bash
deno run \
  --allow-env \
  --allow-read \
  --allow-write \
  --allow-run \
  your_script.ts
```

Why these matter:

- `--allow-run`: required to launch `meridian-cli`
- `--allow-read`: required for MIDI inputs, soundfonts, and other source assets
- `--allow-write`: required for processed MIDI, audio outputs, video outputs,
  and temp files
- `--allow-env`: useful when your app reads `MERIDIAN_CLI_BIN` or other
  environment-based configuration

If you lock permissions down further later, document those exact path-level
rules next to the application that owns them.

## Import Shape

Meridian does not publish the SDK to JSR or npm. The stable public entrypoint is
`sdk/typescript/mod.ts`.

Recommended tagged GitHub import shape:

```ts
import { createDenoMeridianClient } from "https://raw.githubusercontent.com/<owner>/meridian/v0.1.0/sdk/typescript/mod.ts";
```

Practical repo-local shape:

```ts
import { createDenoMeridianClient } from "../sdk/typescript/mod.ts";
```

For shipped code, pin a Git tag or commit. Do not import from `main`.

## Initialization

Initialization comes down to three decisions:

1. Which runtime helper you are using
2. Which `meridian-cli` binary to launch
3. Whether you want the high-level client or the lower-level protocol client

### Deno-first high-level client

This is the default starting point for most apps:

```ts
import { createDenoMeridianClient } from "../sdk/typescript/mod.ts";

const executablePath = Deno.env.get("MERIDIAN_CLI_BIN") ??
  "./target/debug/meridian-cli";

const client = await createDenoMeridianClient(executablePath);

try {
  const analysis = await client.analysis("input.mid", {
    file: true,
    summary: true,
    notes: true,
  });

  console.log(analysis.total_notes);
} finally {
  await client.close();
}
```

Important points:

- `createDenoMeridianClient()` chooses the Deno runtime adapter for you
- the executable path is still your responsibility
- the default subprocess arguments are intended for stdio mode
- `await client.close()` should always run, even on failure

### Generic initialization path

If you do not want the runtime-specific helper, the generic constructor path is
the conceptual model:

```ts
import { createMeridianClient } from "../sdk/typescript/mod.ts";
import { denoRuntimeAdapter } from "../sdk/typescript/src/runtime/deno.ts";

const client = await createMeridianClient({
  executablePath: "./target/debug/meridian-cli",
  runtime: denoRuntimeAdapter,
});
```

Use this path when you want to be explicit about the runtime adapter, or when
you are experimenting with another runtime implementation.

### Lower-level protocol initialization

If you want raw protocol commands and events instead of the higher-level
workflow helpers, initialize the protocol client directly:

```ts
import { createDenoProtocolClient } from "../sdk/typescript/mod.ts";

const client = await createDenoProtocolClient("./target/debug/meridian-cli");

try {
  const events = await client.request({
    type: "shutdown",
  });

  console.log(events);
} finally {
  await client.close();
}
```

Choose this level when:

- you are prototyping against new protocol commands
- you need direct event inspection
- the high-level SDK layer has not wrapped a workflow yet

## Recommended Initialization Pattern

For application code, keep setup centralized:

1. Resolve the binary path once
2. Select the runtime-specific client factory once
3. Pass the client through your app instead of re-creating it ad hoc
4. Close the client on shutdown

A small wrapper module is usually enough:

```ts
import { createDenoMeridianClient } from "../sdk/typescript/mod.ts";

export async function openMeridianClient() {
  const executablePath = Deno.env.get("MERIDIAN_CLI_BIN") ??
    "./target/debug/meridian-cli";

  return createDenoMeridianClient(executablePath);
}
```

For a package-level quick start and the smallest runnable examples, also see
[`sdk/typescript/README.md`](../../sdk/typescript/README.md).

That keeps runtime assumptions and path lookup out of the rest of the app.

## First Workflows To Try

Once initialization is working, these are the lowest-friction next steps:

1. MIDI analysis
2. resource loading
3. MIDI modification
4. audio rendering
5. video rendering
6. raw protocol requests

The examples page links to one file for each of those flows.

## Troubleshooting Checklist

- If the client fails immediately, verify the `meridian-cli` path first.
- If Deno rejects the call, check missing permissions before debugging SDK code.
- If rendering features fail, confirm host-specific dependencies such as
  soundfonts or video tooling.
- If protocol requests hang or exit early, run the CLI directly in stdio mode to
  confirm the binary itself is healthy.

## Next Pages

- [MIDI Modification Tools](./midi-modification-tools.md)
- [Audio Rendering](./audio-rendering.md)
- [Video Rendering](./video-rendering.md)
- [Runtime Support](./runtime-support.md)
- [API Overview](./api-overview.md)
- [Examples](./examples.md)
