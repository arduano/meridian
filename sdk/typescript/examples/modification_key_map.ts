import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-key-map-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/key_map.mid`;

try {
  const result = await client.modification.keyMap({
    input: midiPath,
    output,
    mappings: [{ from: 60, to: 67 }],
    fold_to_range: { min: 48, max: 84 },
  });
  console.log(JSON.stringify({
    tool: "key_map",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
