<!-- This page owns the rules for the mirror of the host WIT worlds. -->
# WIT interface mirror

`host.wit`, `ui.wit` and `modules.wit` are the host's `pito:host` package,
copied byte for byte from the plugin host that ships in the desks (the
toolbox's `plughost` crate). Change an interface in its owning repository,
never here; re-copy on the host's next release and rebuild.

This plugin imports `log`, `kv`, `net` (`post`) and `ui` from it and
nothing from `modules.wit`.
