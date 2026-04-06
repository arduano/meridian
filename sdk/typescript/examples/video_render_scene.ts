import type { SceneConfig } from "../src/index.ts";
import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-video-scene-");
const midiPath = await resolveMidiFixture(
  "piano/mozart-kv457-sonata-no14-fragment.mid",
);
const output = `${dir}/render-scene.mp4`;

const scene: SceneConfig = {
  scene_type: "three_d",
  projector: "piano_trail_classic",
  background: {
    source: "none",
  },
  same_width_notes: true,
  fov: Math.PI / 3,
  view_height: 0.58,
  view_offset: 0.52,
  view_pan: 0.18,
  cam_ang: 0.72,
  cam_rot: 0.06,
  cam_spin: 0,
  viewdist: 14,
  viewback: 0.25,
  vertical_notes: false,
  note_down_speed: 0.7,
  note_up_speed: 0.22,
  box_notes: true,
  light_shade: false,
  show_keyboard: true,
  tilt_keys: true,
  eat_notes: false,
  aura_strength: 0.35,
  aura_enabled: true,
  notes_change_size: false,
  notes_change_tint: true,
  use_vel: false,
  palette: {
    source: "zenith_palette",
    palette: { kind: "random_gradients" },
    randomize: true,
  },
  aura_image: {
    source: "builtin",
    name: "ring",
  },
};

try {
  const result = await client.video.render({
    midiPath,
    output,
    fps: 24,
    width: 640,
    height: 360,
    scene,
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
