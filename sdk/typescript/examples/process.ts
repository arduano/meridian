import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-process-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/processed.mid`;

try {
  const result = await client.modification.rangeSelect({
    inputs: [midiPath],
    output,
    event_kinds: ["note"],
    config: {
      notes: {
        velocity_scale: 0.75,
      },
    },
  });
  console.log(JSON.stringify(
    {
      inputCount: result.input_count,
      outputTrackCount: result.output_track_count,
      output,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
