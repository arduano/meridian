import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-tempo-map-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/tempo_map.mid`;

try {
  const result = await client.modification.tempoMap.flatten(600_000, {
    input: midiPath,
    output,
  });
  console.log(JSON.stringify({
    tool: "tempo_map",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
