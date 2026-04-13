# stdio Mode

The examples below assume your shell already has the required tools available.
On Linux, `nix-shell` is the easiest way to get that environment.

`stdio` is Meridian's long-lived line-delimited JSON protocol mode.

```bash
cargo run -p meridian-cli -- stdio
```

Use it when you want to drive Meridian from another process and keep the CLI
alive between requests. That is the right fit for custom frontends, automation,
and integration testing.

## When To Use It

- Use `stdio` when you want a persistent protocol connection.
- Use `json` when you want to send one raw request and exit.
- Use the TypeScript SDK when you want a higher-level client API on top of the
  same protocol.

## Mental Model

`stdio` speaks Meridian's protocol over standard input and output:

- requests are sent as line-delimited JSON
- responses and events come back on stdout as protocol JSON
- stderr is where normal CLI diagnostics should go

That means `stdio` is good for integrations, but it is not a human-facing
interactive shell.

## What It Can Drive

The same protocol path behind `stdio` can load MIDI, analyze files, inspect
files, process MIDI, merge files, and start audio or video renders.

For ad hoc experimentation, the one-shot raw helper is also available:

```bash
cargo run -p meridian-cli -- json "{\"protocol_version\":1,\"id\":1,\"command\":{\"type\":\"get_render_video_status\"}}"
```

That example sends a single render-status request and exits after printing the
response.

## Practical Notes

- If you are building a real frontend, prefer the TypeScript SDK over manual
  JSON assembly.
- Keep your own app logs off stdout when you are using `stdio`, because stdout
  is part of the protocol transport.
- For CLI-only work, the curated commands in the other pages are easier to use
  than speaking the protocol directly.
