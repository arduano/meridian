import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-quantize-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/quantize.mid`;

try {
  const result = await client.modification.quantize({
    input: midiPath,
    output,
    rounding_ticks: 24,
    mode: "note_start_and_end",
  });
  console.log(JSON.stringify({
    tool: "quantize",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
