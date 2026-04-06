import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-process-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/processed.mid`;

try {
  const result = await client.modification.keyMap({
    input: midiPath,
    output,
    mappings: [{ from: 60, to: 62 }],
    drop_unmapped: false,
  });
  console.log(JSON.stringify(
    {
      input: result.input,
      outputTrackCount: result.output_track_count,
      output,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
