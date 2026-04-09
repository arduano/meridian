import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-control-change-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/control_change.mid`;

try {
  const result = await client.modification.controlChange({
    input: midiPath,
    output,
    strip_controllers: [64],
    remap_controllers: [{ from: 1, to: 11 }],
    scale_controllers: [{ controller: 11, scale: 0.75 }],
    inject_start: [{ channel: 0, controller: 11, value: 100 }],
  });
  console.log(JSON.stringify({
    tool: "control_change",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
