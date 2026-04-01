import { createClient, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const midiPath = await resolveMidiFixture();

try {
  const parsedMidiId = await client.resources.loadParsedMidi(midiPath);
  const loadedMidi = await client.resources.loadAudioMidi(midiPath);
  console.log(JSON.stringify(
    {
      parsedMidiId,
      loadedPath: loadedMidi.path,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
