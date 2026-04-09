import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-pitch-bend-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/pitch_bend.mid`;

try {
  const result = await client.modification.pitchBend({
    input: midiPath,
    output,
    scale: 0.5,
    offset: 256,
    min_bend: -2048,
    max_bend: 2048,
  });
  console.log(JSON.stringify({
    tool: "pitch_bend",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
