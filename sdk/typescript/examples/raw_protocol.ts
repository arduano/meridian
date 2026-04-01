import { createDenoProtocolClient } from "../src/runtime/deno_client.ts";
import { defaultExecutablePath, resolveMidiFixture } from "./_shared.ts";

const client = await createDenoProtocolClient(defaultExecutablePath());
const midiPath = await resolveMidiFixture();

try {
  const parsed = await client.request({
    type: "load_parsed_midi",
    path: midiPath,
  });
  const shutdown = await client.request({ type: "shutdown" });

  console.log(JSON.stringify(
    {
      parsedEventTypes: parsed.map((event) => event.type),
      shutdownEventTypes: shutdown.map((event) => event.type),
    },
    null,
    2,
  ));
} finally {
  await client.close();
}
