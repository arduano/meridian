import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-track-route-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/track_route.mid`;

try {
  const result = await client.modification.trackRoute.splitByChannel({
    input: midiPath,
    output,
  });
  console.log(JSON.stringify({
    tool: "track_route",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
