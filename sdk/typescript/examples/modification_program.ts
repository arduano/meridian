import { createClient, createScratchDir, resolveMidiFixture } from "./_shared.ts";

const client = await createClient();
const dir = await createScratchDir("meridian-example-program-");
const midiPath = await resolveMidiFixture();
const output = `${dir}/program.mid`;

try {
  const result = await client.modification.program({
    input: midiPath,
    output,
    force_program: 0,
    startup_programs: [{ channel: 0, program: 0 }],
  });
  console.log(JSON.stringify({
    tool: "program",
    input: result.input,
    output,
    outputTrackCount: result.output_track_count,
  }, null, 2));
} finally {
  await client.close();
}
