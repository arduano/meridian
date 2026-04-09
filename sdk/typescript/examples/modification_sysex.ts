import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-sysex-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/sysex.mid`;

try {
  const result = await client.modification.sysex({
    input: midiPath,
    output,
    strip_all: true,
    prepend: [[0x7d, 0x01, 0x02, 0x03]],
  });
  console.log(JSON.stringify({
    tool: "sysex",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
