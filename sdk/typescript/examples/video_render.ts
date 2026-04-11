import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-video-");
const midiPath = await resolveMidiFixture(
  "piano/mozart-kv457-sonata-no14-fragment.mid",
);
const output = `${dir}/render.mkv`;

try {
  const result = await client.video.render({
    midiPath,
    output,
    container: "mkv",
    fps: 24,
    width: 640,
    height: 360,
    renderer: "flat",
    viewRange: 4,
    timeSpace: "time",
    firstKey: 21,
    lastKey: 108,
    ffmpegArgs: ["-pix_fmt", "yuv420p"],
  });
  console.log(JSON.stringify(
    {
      totalFrames: result.total_frames,
      output,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
