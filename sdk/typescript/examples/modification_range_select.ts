import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-range-select-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/range_select.mid`;

try {
  const result = await client.modification.rangeSelect({
    input: midiPath,
    output,
    start_ticks: 0,
    end_ticks: 72,
    offset_ticks: 0,
    edge_behavior: "trim",
  });
  console.log(JSON.stringify({
    tool: "range_select",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
