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
  const envPath = Deno.env.get("MERIDIAN_CLI_BIN");
  if (envPath) {
    return envPath;
  }
  return filePath("../../../target/debug/meridian-cli");
}

export async function defaultSoundfontPath(): Promise<string | null> {
  const envPath = Deno.env.get("MERIDIAN_TEST_SOUNDFONT") ??
    Deno.env.get("MERIDIAN_SOUNDFONT");
  if (envPath) {
    return envPath;
  }

  return await firstExistingPath([
    filePath(
      "../../../assets/soundfonts/freepats-upright-kw-small/UprightPianoKW-small-20190703.sfz",
    ),
  ]);
}

export async function ensureExecutable(path: string): Promise<void> {
  try {
    const stat = await Deno.stat(path);
    if (stat.isFile) {
      return;
    }
  } catch {
    // build below
  }

  const command = new Deno.Command("nix-shell", {
    cwd: new URL("../../..", import.meta.url).pathname,
    args: [
      "--run",
      "PATH=/run/current-system/sw/bin:$PATH cargo build -p meridian-cli",
    ],
    stdout: "inherit",
    stderr: "inherit",
  });
  const status = await command.spawn().status;
  if (status.code !== 0) {
    throw new Error(`Failed to build meridian-cli, exit code ${status.code}`);
  }
}

export async function hasCommand(command: string): Promise<boolean> {
  try {
    const status = await new Deno.Command(command, {
      args: ["-version"],
      stdout: "null",
      stderr: "null",
    }).spawn().status;
    return status.code === 0;
  } catch {
    return false;
  }
}

function fixtureCandidates(name: string): URL[] {
  return [
    new URL(`../../../assets/midis/${name}`, import.meta.url),
    new URL(`../../../assets/midis/test/${name}`, import.meta.url),
    new URL(`../../../assets/midis/piano/${name}`, import.meta.url),
  ];
}

export async function resolveMidiFixture(
  name: string,
  fallbackBytes: Uint8Array,
): Promise<string> {
  for (const candidate of fixtureCandidates(name)) {
    try {
      const stat = await Deno.stat(candidate);
      if (stat.isFile) {
        return decodeURIComponent(candidate.pathname);
      }
    } catch {
      // fallback below
    }
  }

  const dir = await Deno.makeTempDir({ prefix: "meridian-sdk-midi-" });
  const path = `${dir}/${name}`;
  await Deno.writeFile(path, fallbackBytes);
  return path;
}

export const TWO_NOTE_MIDI = new Uint8Array([
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
