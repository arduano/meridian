import { createDenoMeridianClient } from "../mod.ts";

const executablePath = Deno.env.get("MERIDIAN_CLI_BIN") ??
  new URL("../../../target/debug/meridian-cli", import.meta.url).pathname;
const midiPath = new URL(
  "../../../assets/midis/piano/burgmuller-op100-no4-the-little-party.mid",
  import.meta.url,
).pathname;

const client = await createDenoMeridianClient(executablePath);

try {
  const analysis = await client.analysis(midiPath, {
    file: true,
    summary: true,
    notes: true,
  });

  console.log(JSON.stringify(
    {
      midiPath,
      totalNotes: analysis.total_notes,
      keysWithNotes: analysis.summary.keys_with_notes,
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
