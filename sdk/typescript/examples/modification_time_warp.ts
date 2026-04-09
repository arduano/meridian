import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-time-warp-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/time_warp.mid`;

try {
  const result = await client.modification.timeWarp({
    input: midiPath,
    output,
    points: [
      { source_tick: 0, dest_tick: 0 },
      { source_tick: 96, dest_tick: 144 },
    ],
  });
  console.log(JSON.stringify({
    tool: "time_warp",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
