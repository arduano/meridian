## Provenance

The files in `assets/midis/piano/` are a curated subset downloaded from
Mutopia on 2026-04-01.

Mutopia home page:
- https://www.mutopiaproject.org/

Mutopia pages for these scores state that the sheet music was placed in the
public domain by the typesetter and is free to distribute, modify, and perform.
These MIDI files are vendored here for repository-local examples and smoke
tests.

Files:

0. `smoke-two-notes.mid`
Origin:
- repository-local deterministic smoke fixture
Notes:
- intentionally tiny two-note MIDI used where tests need exact counts and very
  fast execution
- not sourced from Mutopia; kept alongside the vendored public piano pieces so
  every non-UI test fixture lives under one tree

1. `burgmuller-op100-no4-the-little-party.mid`
Source MIDI:
- https://www.mutopiaproject.org/ftp/BurgmullerJFF/O100/25EF-04/25EF-04.mid
Source score page:
- https://www.mutopiaproject.org/ftp/BurgmullerJFF/O100/25EF-04/25EF-04-let.pdf
Composer:
- Johann Friedrich Franz Burgmuller
Notes:
- short solo-piano etude; good default smoke/example input

2. `burgmuller-op100-no13-consolation.mid`
Source MIDI:
- https://www.mutopiaproject.org/ftp/BurgmullerJFF/O100/25EF-13/25EF-13.mid
Source score page:
- https://www.mutopiaproject.org/ftp/BurgmullerJFF/O100/25EF-13/25EF-13-let.pdf
Composer:
- Johann Friedrich Franz Burgmuller
Notes:
- slower lyrical piano piece; useful for render and timing examples

3. `mozart-kv457-sonata-no14-fragment.mid`
Source MIDI:
- https://www.mutopiaproject.org/ftp/MozartWA/KV457/sonata3/sonata3.mid
Source score page:
- https://www.mutopiaproject.org/ftp/MozartWA/KV457/sonata3/sonata3-a4.pdf
Composer:
- Wolfgang Amadeus Mozart
Notes:
- larger piano work fragment; useful when examples need denser musical content
