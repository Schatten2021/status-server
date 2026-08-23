Names configuration
-------------------
The `names` component is just a map of *component_id* => *component name*.

This component (or the attribute it sets) is internally used by multiple other components (i.e. [email](email.md), 
[ntfy](ntfy.md), etc.) to set the displayed name of the element.

# Example
```toml
[names]
foo = "Foo webserver"
bar = "Bar Minecraft server"
```