import { createClient, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const midiPath = await resolveMidiFixture();

try {
  const analysis = await client.analysis(midiPath, {
    file: true,
    summary: true,
    events: true,
    notes: true,
    tempo: true,
    buckets: 4,
  });
  console.log(JSON.stringify(
    {
      totalNotes: analysis.total_notes,
      noteOnEvents: analysis.events.note_on_events,
      buckets: analysis.buckets.length,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
