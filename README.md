Status Server
-------------
[![Clippy Check](https://github.com/Schatten2021/status-server/actions/workflows/clippy-check.yml/badge.svg)](https://github.com/Schatten2021/status-server/actions/workflows/clippy-check.yml)
[![Tests](https://github.com/Schatten2021/status-server/actions/workflows/test-check.yml/badge.svg)](https://github.com/Schatten2021/status-server/actions/workflows/test-check.yml)

This is my own status-monitoring server.

# Structure
It is built around the concept of [Components](#component), [Notification-Provider](#notification-provider) and [Elements](#elements).

## Component

A Component is defined as some part that can either keep track of the state of an `Element`, 
send notifications (in which case it's also a `NotificationProvider`) or provide some other service
for the status server (e.g. the frontend).

Every component has its own `ID` which is used to identify the configuration for this configuration (see [Configuration]())

## Notification-Provider

A Notification-Provider provides a means to notify users. 
How that happens exactly is dependent on the provider, but some (implemented) ways could be E-Mail, Push & Websockets.

These are required to actually ensure that notifications reach their targets.

## Elements

An Element is a single "unit" of status. 
Each element is either online or offline and has a set of additional attributes that can be set by [Components](#component)

# Configuration

The program accepts a set of command-line arguments for very basic configuration (see `status-server --help` for reference).
These are mainly:
- `-p`: Sets the port
- `-b`: binds to the given address
- `-c`: selects a different configuration file
- `-h`: prints the help
- `-V`: prints the version

Any further configuration is done inside the `config.toml` (or whichever toml file you passed to `-c`).

"Global" Configuration (i.e. ignored components) are in the `global` key for the config. 
(i.e. `global.ignored.components`)

On Unix platforms you can send `SIGUSR1` signal to the program to trigger a reload of the configuration file.

## Components
you can enable and disable [Components](#component) (works at runtime) by putting their ids in the `ignored.components` 
global config field (so `global.ignored.components`).

**WARNING!**: Ignoring a dependency of another [Component](#component) will not allow that dependency to be added to the 
server, even if that breaks other [Component](#component). Deleting a [Component](#component) by removing it and 
reloading the configuration also removes any dependants.

Each [Component](#component) is configured via its `ID` and usually a suffix (so that e.g. future E-Mail status support
can use `email.status`; NOTE: `notify` are also aliased as `notifications`). The ids for the default components are:

| Component  | ID          | suffix     | feature             | config reference                 |
|------------|-------------|------------|---------------------|----------------------------------|
| api        | `api`       | `frontend` | api                 | [ref](docs/config/api.md)        |
| websockets | `sockets`   | `notify`   | websockets          | [ref](docs/config/websockets.md) |
| frontend   | `frontend`  | none       | frontend            | no config                        |
| ntfy       | `ntfy`      | `notify`   | ntfy-notifications  | [ref](docs/config/ntfy.md)       |
| email      | `email`     | `notify`   | email-notifications | [ref](docs/config/email.md)      |
| minecraft  | `minecraft` | `status`   | minecraft-status    | [ref](docs/config/minecraft.md)  |
| website    | `website`   | `status`   | website-status      | [ref](docs/config/website.md)    |
| dataminer  | `miner`     | `status`   | dataminer-status    | [ref](docs/config/dataminer.md)  |
| names      | `names`     | none       | names               | [ref](docs/config/names.md)      |
| history    | `history`   | none       | history (+ backend) |                                  |

Additionally, many configurations use [filters](docs/config/filter.md) to provide a uniform filtering interface.

# Versioning 
This project uses [semantic versioning](https://semver.org/) for its binaries.
This means that the released compiled binaries (and source-code) are backwards compatible between minor version changes.

This is ensured via config-tests. Any change to any of the config tests is considered a breaking change.

What counts as a non-breaking change:
- changes to the frontend UI
- changes to some internal behavior, that does not change exported types

What counts as a breaking/feature change:
- changes to the API
- changes to some exported types in a crate
- changes to the configuration