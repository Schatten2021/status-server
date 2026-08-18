Dataminer configuration
-----------------------
- A Map of miner ids → miner configuration

| field   | type                                                                                      | description                                |
|---------|-------------------------------------------------------------------------------------------|--------------------------------------------|
| timeout | [Duration](https://kellnr.fms.nrw/docs/utils/0.5.2/doc/utils/duration_parsing/index.html) | How long to wait until the miner times out |


# Example
```toml
[miner.foo.status]
timeout = "5s"
```