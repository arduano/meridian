import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-velocity-map-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/velocity_map.mid`;

try {
  const result = await client.modification.velocityMap.polyline([
    { input: 0, output: 0 },
    { input: 64, output: 96 },
    { input: 127, output: 127 },
  ], {
    input: midiPath,
    output,
  });
  console.log(JSON.stringify({
    tool: "velocity_map",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
