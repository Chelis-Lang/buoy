## Planned Implementation Relations

| Requirement ID | Relation ID | Consumer | Logical target selector | Action | Owner | Focused Buoy command |
|---|---|---|---|---|---|---|
| `RTP-001` | `RTP-TRACE-COMMAND` | `Chelis-Lang/buoy` | `src/cli.rs::trace_command` | add | CLI owner | `buoy trace --requirement-id RTP-001 --format machine` |
| `RTP-002` | `RTP-TRACE-QUERY` | `Chelis-Lang/buoy` | `blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4` | add | core owner | `buoy trace --record blake3-256:0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4` |
