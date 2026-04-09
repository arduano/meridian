import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-extract-track-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/extract_track.mid`;

try {
  const result = await client.modification.extractTrack(0, {
    input: midiPath,
    output,
  });
  console.log(JSON.stringify({
    tool: "extract_track",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
