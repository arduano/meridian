import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-channel-remap-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/channel_remap.mid`;

try {
  const result = await client.modification.channelRemap({
    input: midiPath,
    output,
    mappings: [{ from: 0, to: 2 }],
  });
  console.log(JSON.stringify({
    tool: "channel_remap",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
