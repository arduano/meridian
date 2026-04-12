import { createDenoMeridianClient } from "../src/index.ts";

const TWO_NOTE_FIXTURE = new Uint8Array([
  0x4d,
  0x54,
  0x68,
  0x64,
  0x00,
  0x00,
  0x00,
  0x06,
  0x00,
  0x00,
  0x00,
  0x01,
  0x00,
  0x60,
  0x4d,
  0x54,
  0x72,
  0x6b,
  0x00,
  0x00,
  0x00,
  0x1b,
  0x00,
  0xff,
  0x51,
  0x03,
  0x07,
  0xa1,
  0x20,
  0x00,
  0x90,
  0x3c,
  0x64,
  0x30,
  0x90,
  0x40,
  0x64,
  0x30,
  0x80,
  0x3c,
  0x40,
  0x30,
  0x80,
  0x40,
  0x40,
  0x00,
  0xff,
  0x2f,
  0x00,
]);

function filePath(relative: string): string {
  return decodeURIComponent(new URL(relative, import.meta.url).pathname);
}

async function firstExistingPath(candidates: string[]): Promise<string | null> {
  for (const candidate of candidates) {
    try {
      const stat = await Deno.stat(candidate);
      if (stat.isFile) {
        return candidate;
      }
    } catch {
      // keep searching
    }
  }
  return null;
}

export function defaultExecutablePath(): string {
  return Deno.env.get("MERIDIAN_CLI_BIN") ??
    filePath("../../../target/debug/meridian-cli");
}

export async function resolveMidiFixture(
  name = "piano/burgmuller-op100-no4-the-little-party.mid",
): Promise<string> {
  const path = await firstExistingPath([filePath(`../../../assets/midis/${name}`)]);
  if (path) {
    return path;
  }

  const dir = await Deno.makeTempDir({ prefix: "meridian-example-midi-" });
  const midiPath = `${dir}/fixture.mid`;
  await writeFixtureMidi(midiPath);
  return midiPath;
}

export async function createScratchDir(prefix: string): Promise<string> {
  return Deno.makeTempDir({ prefix });
}

export async function writeFixtureMidi(path: string): Promise<void> {
  await Deno.writeFile(path, TWO_NOTE_FIXTURE);
}

export async function createFixtureMidi(
  prefix: string,
): Promise<{ dir: string; midiPath: string }> {
  const dir = await Deno.makeTempDir({ prefix });
  const midiPath = `${dir}/fixture.mid`;
  await writeFixtureMidi(midiPath);
  return { dir, midiPath };
}

export async function defaultSoundfontPath(): Promise<string> {
  const envPath = Deno.env.get("MERIDIAN_EXAMPLE_SOUNDFONT") ??
    Deno.env.get("MERIDIAN_SOUNDFONT");
  if (envPath) {
    return envPath;
  }

  const bundled = await firstExistingPath([
    filePath(
      "../../../assets/soundfonts/freepats-upright-kw-small/UprightPianoKW-small-20190703.sf2",
    ),
  ]);
  if (bundled) {
    return bundled;
  }

  throw new Error(
    "Bundled example soundfont is missing; set MERIDIAN_EXAMPLE_SOUNDFONT or MERIDIAN_SOUNDFONT",
  );
}

export async function createClient() {
  return createDenoMeridianClient(defaultExecutablePath());
}
