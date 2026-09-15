# pigeon-slack

The Slack plugin for Done, the orchestrator desk: a message to a webhook you configure on the plugin's page when a Task comes back to you, done, stuck or capped; the desk itself never talks to Slack.

Under construction: nothing to install yet. When it ships, a tagged
release carries three files, `plugin.toml`, `plugin.wasm` and
`SHA256SUMS`, and Done installs it from this repository by name,
verifying the hash before loading anything. Official: the owner of this
repository is the owner of the desks.
