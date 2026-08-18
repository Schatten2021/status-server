Websocket configuration
-----------------------
| field             | type                | description                                                            |
|-------------------|---------------------|------------------------------------------------------------------------|
| path              | String              | The path where the Websockets are reachable. **MIND THE LEADING '/'!** |
| filter            | [Filter](filter.md) | Filters the messages sent via the WebSockets                           |

## Example
```toml
[sockets.notify]
path = "/ws"
filter.changes.deny = [{ attribute.id = "minecraft.players", attribute.exact = false }]
```