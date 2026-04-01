import {
  createScratchDir,
  createClient,
  defaultSoundfontPath,
  resolveMidiFixture,
} from "./_shared.ts";

const soundfont = await defaultSoundfontPath();
const client = await createClient();
const dir = await createScratchDir("meridian-example-audio-");
const midiPath = await resolveMidiFixture("piano/burgmuller-op100-no13-consolation.mid");
const output = `${dir}/render.wav`;

try {
  const result = await client.audio.render({
    midiPath,
    output,
    sampleRate: 22050,
    channels: 2,
    soundfonts: [soundfont],
  });
  console.log(JSON.stringify(
    {
      framesWritten: result.frames_written,
      output,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
