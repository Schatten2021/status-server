Minecraft configuration
-----------------------
Minecraft currently only supports java via the `minecraft-java` config value.

`minecraft.java` is a map of server-id -> server-config

| field    | type                                                                                                  | description                                                    |
|----------|-------------------------------------------------------------------------------------------------------|----------------------------------------------------------------|
| url      | string                                                                                                | The url to which to connect to.                                |
| port     | u16                                                                                                   | The port to which to connect to.                               |
| interval | [Duration]([Duration](https://kellnr.fms.nrw/docs/utils/0.5.2/doc/utils/duration_parsing/index.html)) | The interval in which to ping the server to update the status. |


# Example
```toml
[minecraft.status.java.foo]
url = "exaple.com"
#port = 25565 # usually not necessary; 25565 is the default java port.
interval = "5s"
```