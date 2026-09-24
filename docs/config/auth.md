Auth configuration
------------------
The Authentication component has, similar to the [Historization](history.md) component multiple different backends.
These backends currently include:


| config key                | feature               | ref            | description                                                                    |
|---------------------------|-----------------------|----------------|--------------------------------------------------------------------------------|
| N/A (configured "flatly") | `auth-config-backend` | [ref](#config) | Basic backend allowing the deffinition of users & roles directly in the config |


Generally each backend provides a way to log in (with that specific backend, methods are _additive_, **not** replacing.).
The sessions are "global", meaning that logging in via the config backend grants roles from the Sqlite Backend (not yet implemented) as well.

Each User has a set of roles associated with them & each role as a set of attributes (though these can be overwritten on a per-user basis).

> NOTE: The `admin` role is special in that it grants a bypass to basically everything.
> Admins will be able to see all statuse & more (in the future).

> NOTE: The `ignores_api_rules` attribute is special in that, if set to `true` any user with that attribute also ignores
> Any restrictions put on the API (& websockets).


# Backends
## Config
| key              | type                                                                                      | description                                       |
|------------------|-------------------------------------------------------------------------------------------|---------------------------------------------------|
| users            | List of [User](#user)                                                                     | Hardcoded list of users                           |
| roles            | List of [Role](#role)                                                                     | Hardcoded list of roles that exist.               |
| session_duration | [Duration](https://kellnr.fms.nrw/docs/utils/0.5.2/doc/utils/duration_parsing/index.html) | The duration after which a session turns invalid. |

### User

| key        | type                   | decription                                                  | 
|------------|------------------------|-------------------------------------------------------------|
| username   | String                 | The username (ID) of the user                               |
| password   | [Password](#password)  | The password of the user                                    |
| attributes | Map of string => Value | Custom attributes that are defined on a per-user basis      |
| roles      | List of role ids       | The roles that the user posesses. Note: `admin` is special! |

#### Password
Either "hashed" (recommended; see `cargo run -- auth generate`) or "plaintext". 
Both are just a string.


Examples:
```toml
password.plaintext = "foo"
password2.hash = "$argon2id$v=19$m=19456,t=2,p=1$Qw1IBA6u0yk3auqi4FQKaw$NGF3rhTtcG3yKDFLA5glerSqyAl5D0Zm7Fm/rmESoG0" # also "foo"
```
### Role
| key        | type                | description                    |
|------------|---------------------|--------------------------------|
| attributes | map String => value | Attributes for the given role. |

# Example
```toml
[roles.buzz]
attributes.ignore_api_rules = true
[[users]]
username = "foo"
# password: "bar"
password.plaintext = "$argon2id$v=19$m=19456,t=2,p=1$+q2JzFPvjYjQyozrh0wPkg$h1rN/Nt7ZqWvXd9EBcMSFkEUAwwD0Pk+/8r+t1qalzs"
roles = ["buzz"] # user is of role buzz => ignores api rules & can see everything.

# example admin user
[[users]]
username = "admin"
password.plaintext = "admin" # PLEASE DON'T DO THIS
roles = ["admin"]
```