import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-humanize-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/humanize.mid`;

try {
  const result = await client.modification.humanize({
    input: midiPath,
    output,
    start_jitter: 4,
    length_jitter: 8,
    velocity_jitter: 12,
    seed: 7,
    collision_mode: "distinguish_by_note",
  });
  console.log(JSON.stringify({
    tool: "humanize",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
