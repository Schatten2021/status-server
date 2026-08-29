History configuration
---------------------
The Historization has multiple different backend, which can be configured independently of the main backend.

NOTE: both `elements` and `attributes` filter **new** changes, not existing ones.
Other components (including [API](api.md)) **will** be able to use existing history entries.

| field      | type                                                                                      | description                                             |
|------------|-------------------------------------------------------------------------------------------|---------------------------------------------------------|
| elements   | [SingleFilter](filter.md#single-filter) of String                                         | Filters the elements that will be historized            |
| attributes | [SingleFilter](filter.md#single-filter) of [AttributeMatcher](filter.md#attributematcher) | Filters the attributes that will be historized          |

The configurations for the diferent backends can be accessed like this:

| config key | feature                   | ref                                  | description                                             |
|------------|---------------------------|--------------------------------------|---------------------------------------------------------|
| `sqlite`   | `history-sqlite-backend`  | [Sqlite Config](#sqlite)             | The configuration for the SQLite backend                |
| `fs_json`  | `history-fs-json-backend` | [FileSystem Json](#filesystem-json)  | The configuration for the JSON-Based Filesystem backend |

# Sqlite
A backend for saving into an SQLite3 Database.

| field | type          | default           | description                   | 
|-------|---------------|-------------------|-------------------------------|
| path  | Path          | `history.sqlite3` | path to the sqlite database.  |
| mode  | [Mode](#mode) | see [Mode](#mode) | The mode in which to operate. |

## Mode
These are the different modes currently supported:
- `standard`: a sensible default with 4 tables. See [Standard](#standard)
### Standard
| field                     | type   | Default               | description                                                          |
|---------------------------|--------|-----------------------|----------------------------------------------------------------------|
| element_lookup_table      | String | `elements`            | The table in which to save the `element_id` => internal id lookup.   |
| attribute_lookup_table    | String | `attributes`          | The table in which to save teh `attribute_id` => internal id lookup. |
| online_state_change_table | String | `online_state_change` | The name of the table in which to save the changes in online-state   |
| attribute_change_table    | String | `attribute_change`    | The name of the table in which to save the changes in attributes     |


# Filesystem JSON
Stores the history in the filesystem with each element being saved into a JSON file.

This backend only has the `base_path` field, which denotes the base directory in which to save the history.
Defaults to `history/json`.

# Example
```toml
[history]
elements.deny = ["foo"]
attributes.deny = ["website.last_seen", "minecraft.last_seen"]

[history.sqlite]
path = "db.sqlite3"
mode.standard = {
    element_lookup_table="element",
    attribute_lookup_table="attribute",
    online_state_change_table="online",
    attribute_change_table="attribute"
}
    
[history.fs_json]
base_path = "backup/history"
```
