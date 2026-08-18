NTFY configuration
------------------
NTFY is configured via a list of different targets.

Each target has the same structure as the JSON request (see [ntfy docs](https://docs.ntfy.sh/publish/#publish-as-json); NOTE: actions aren't supported).

In addition to the ntfy json fields:


| field      | type                | description                                                  |
|------------|---------------------|--------------------------------------------------------------|
| base       | Url                 | Url of the NTFY server.                                      |
| filter     | [Filter](filter.md) | Filter to apply for the target.                              |
| auth_token | String              | Authentication token to be used for authenticated endpoints. |

Additionally, `title` and `message` are formatted strings with the following arguments:

| arg            | description                                                                                    |
|----------------|------------------------------------------------------------------------------------------------|
| component_id   | id of the component that triggered the Notification                                            |
| element_id     | id of the element that was changed                                                             |
| reason_short   | short version of the reason (meant for titles/etc.)                                            |
| reason_long    | long version of the reason (contains old & new value of attribute changes)                     |
| attr_new_value | when an attribute was changed (or created) contains the string representation of the new value |
| attr_old_value | when an attribute was changed (or deleted) contains the string representation of the old value |
| attr_id        | when an attribute was changed contains the id of the attribute                                 |
| status_new     | when the online status changed contains the new status (`online`/`offline`)                    |
| status_old     | when the online status changed contains the old status (`online`/`offline`)                    |

# Example
```toml
[[ntfy.notify]]
base = "https://ntfy.sh/"
filter.deny.state = [ "create", { attribute.event="any" } ]
title = "{element_id} {reason_short}"
message = "{component_id} {element_id} {reason_long}"
```