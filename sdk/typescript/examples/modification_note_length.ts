import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-note-length-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/note_length.mid`;

try {
  const result = await client.modification.noteLength({
    input: midiPath,
    output,
    scale: 1.5,
    min_ticks: 24,
  });
  console.log(JSON.stringify({
    tool: "note_length",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
