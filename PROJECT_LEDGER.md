# Active integration findings

Raydio isolated source pull on Oracle reproduced a 3.94-second request stall.
Independent libopus decoding found no malformed durations, mid-song silence,
clipping, or nonfinite PCM. Two socket traces did not reproduce that long wait.
A separate matched compressed-input experiment measured six reads over 20 ms
(max 244.864 ms) when streamed, versus zero (max 0.957 ms) after anonymous-file
staging. Startup increased from 0.603 to 2.286 seconds for 3,433,755 bytes.
This justifies an opt-in bounded staging implementation; larger/live sources
remain streaming and the bot still requires downstream and endurance evidence.
Evidence is in Raydio evidence/ORACLE-ENDURANCE.md and the source audit JSON.

Implementation: opt-in Unix anonymous-file staging, default disabled, 64 MiB
ceiling and one temporary 64 KiB copy buffer. Media HTTP (16), media unit
(20 plus anonymous-file lifecycle), YouTube (38), remote HTTP (10), formatting,
Clippy, cargo audit, cargo deny, and cargo vet (171 audited) pass. Existing
live/environment test exclusions remain unchanged. No dependency changes.
