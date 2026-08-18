Filter Configuration
--------------------

Many Items require filters. 
Due to that there is a unified interface for filtering elements.

# Filter
A Filter is made up of 3 separate [Single-Filters](#single-filter). These are

| Filter       | filtered type               | description                                     |
|--------------|-----------------------------|-------------------------------------------------|
| component    | String                      | The ids of the component sending a Notification |
| entities     | String                      | The ids of the enties changed                   |
| state_change | [StateChange](#statechange) | Changes in state                                |

### Example
```toml
[filter]
components.allow = ["foo", "bar"]
components.deny = ["buf"]
entities.allow = ["foo", "bar"]
entities.deny = ["suf"]
changes.allow = [
    { online = "offline" }, # whitelist any change that results in an element being offline
    { attribute.event="create", attribute.id="foo.bar" } # allow creation of "foo.bar" to pass while deying every other attribute change event
]
changes.deny = [
    "create", # filter out creation of new elemnts
    { attribute = "any" } # filter out any attribute change
]
```

## StateChange
This is an enum where only one can be selected at a time

| change    | data                                       | description                                                                                         |
|-----------|--------------------------------------------|-----------------------------------------------------------------------------------------------------|
| create    |                                            | Creation of entities                                                                                |
| attribute | [AttributeChange](#attributechange)        | Changes to the Attribute of an entity                                                               |
| online    | Online state (`online`, `offline` or `any) | Changes to the online state. If no boolean is supplied all changes to the online state are matched. |

### Example
```toml
change1 = "create" # matches the creation of elements
change2.attribute.event = "any" # matches any change to an attribute
change3.attribute = { event="create", id="foo.bar" } # matches the creation of the "foo.bar" attribute
change4.online = "any" # matches any online status change
change5.online = "offline" # matches any online status change where the new status is `online`
```

## AttributeChange
| config key | type           | description                                                                     |
|------------|----------------|---------------------------------------------------------------------------------|
| id         | string         | the ID of the attributes to match                                               |
| exact      | bool           | whether to exactly match the id or also match subattributes. Defaults to `true` |
| event      | AttributeEvent | The event to match                                                              |

## AttributeMatcher
| config key | type   | description                                              |
|------------|--------|----------------------------------------------------------|
| id         | string | the ID of the attributes to match                        |
| exact      | bool   | whether to exactly match the `id` or also match children |

NOTE: `exact` is `true` by default!
### AttributeEvents

| event  | description                             |
|--------|-----------------------------------------|
| any    | Matches all events                      |
| create | Matches the creation of a new attribute |
| change | Matches the change of an attribute      |
| delete | Matches the deletion of an attribute    |
### Example
```toml
change = { event="any" } # match any change in an attribute
change2 = { id="foo.bar", exact=true, event="any" } # match any change to the attribute "foo.bar"
change3 = { event="create" } # match all creation events for any attribute
```

# Single-Filter
Each Single-Filter has a whitelist and a blacklist.

The whitelist can be identified via any of these names: `allow`, `enable`, `whitelist`, `accept`

The blacklist can be identified via any of these names: `disallow`, `disable`, `blacklist`, `deny`

You can also set the mode via the `mode` (or `default`) keyword. 
This can specify the behavior as
- `explicit-blacklist` (or `allow`): Allows everything by default and only rejects something if it is blacklisted (**and** not in the whitelist)
- `explicit-whitelist` (or `deny`): Disallows everything by default and only accepts something if it is whitelisted (**and** not in the blacklist)

