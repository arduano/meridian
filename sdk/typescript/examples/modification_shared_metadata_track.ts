import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-shared-metadata-track-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/shared_metadata_track.mid`;

try {
  const result = await client.modification.sharedMetadataTrack({
    input: midiPath,
    output,
    destination: { mode: "create_new" },
    strip_redundant_events: true,
    move_tempo_events: true,
    move_time_signatures: true,
    move_key_signatures: true,
  });
  console.log(JSON.stringify({
    tool: "shared_metadata_track",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
