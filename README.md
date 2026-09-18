# pigeon-slack

The Slack plugin for Done, the orchestrator desk: a message to a webhook you
configure on the plugin's page when a Task comes back to you — done, stuck or
capped. The desk itself never talks to Slack; this plugin does, and only to
the one host its manifest names.

## Install

In Done, open Plugins, *Add from GitHub*, and type `gmrdad82/pigeon-slack`.
The capability sheet lists what the plugin may do — network access to
`hooks.slack.com`, nothing else — and Enter installs the latest release after
its hash is checked against `SHA256SUMS`.

## Configure

Create an incoming webhook in Slack (Slack's own guide: *Incoming Webhooks*
in your workspace's app settings), then open the plugin's page in Plugins,
paste the URL into *Webhook URL* and press *Save*. The page shows the saved
URL masked after the host (`https://hooks.slack.com/…/[redacted]`); the URL is
kept in the plugin's own store on your machine and never appears in a log.
*Send a test message* posts `🕊️ Done. can reach this channel.` so you can see
the channel it lands in.

## The three messages

- `🕊️ *<KEY> is back* — <title> · <agent> handed it back done.`
- `🪨 *<KEY> is stuck* — <title> · <agent>'s session ended without handing back; it is in your hands.`
- `🛑 *<KEY> hit the cap* — <title> · <n> rounds; the loop stops here until you say so.`

Nothing configured means nothing sent: the plugin notes it once in the desk's
log and stays quiet. A webhook that answers anything but success is noted with
its status code, never with the URL.

## Build

```sh
cargo build --release --target wasm32-wasip2
```

The artifact is `target/wasm32-wasip2/release/pigeon_slack.wasm`. A `v*` tag
builds it in CI and attaches `plugin.toml`, `plugin.wasm` and `SHA256SUMS` to
the GitHub release; the hashes are computed from the bytes CI built. `wit/`
mirrors the host's interfaces and is never edited here.
