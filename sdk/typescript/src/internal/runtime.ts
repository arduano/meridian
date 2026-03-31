export interface SpawnHandlers {
  onLine(line: string): void;
  onError(error: Error): void;
  onExit(code: number | null, signal: string | null): void;
}

export interface MeridianSubprocess {
  sendLine(line: string): Promise<void>;
  kill(): Promise<void>;
}

export interface MeridianRuntimeAdapter {
  name: string;
  spawn(
    executablePath: string,
    args: string[],
    handlers: SpawnHandlers,
  ): Promise<MeridianSubprocess>;
}
