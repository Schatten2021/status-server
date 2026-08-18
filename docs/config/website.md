Website configuration
---------------------
| field    | type                                                                                                  | description                                                                                    |
|----------|-------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------|
| url      | Url                                                                                                   | The url to request (with method)                                                               |
| interval | [Duration]([Duration](https://kellnr.fms.nrw/docs/utils/0.5.2/doc/utils/duration_parsing/index.html)) | The interval in which to request the url.                                                      |
| status   | [SingleFilter](filter.md#single-filter) of status-codes                                               | A Filter to apply to the returned status codes. 200-299 codes are accepted unless blacklisted. |


# Example
```toml
[website.status.foo]
url = "https://example.com/"
interval = "1m"
status.accept = [200]
```