import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-change-ppq-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/change_ppq.mid`;

try {
  const result = await client.modification.changePpq(192, {
    input: midiPath,
    output,
  });
  console.log(JSON.stringify({
    tool: "change_ppq",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
