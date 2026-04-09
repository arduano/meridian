import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-meta-text-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/meta_text.mid`;

try {
  const result = await client.modification.metaText({
    input: midiPath,
    output,
    keep_kinds: ["track_name", "marker"],
  });
  console.log(JSON.stringify({
    tool: "meta_text",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
