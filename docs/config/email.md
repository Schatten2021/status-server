E-Mail configuration
--------------------
| field       | type                | Default    | description                                                                                                |
|-------------|---------------------|------------|------------------------------------------------------------------------------------------------------------|
| address     | E-Mail-Address      |            | The address which to use for sending the E-Mails. Is also the username used to log into the E-Mail-Server. |
| password    | String              |            | The password to use to log into the E-Mail account.                                                        |
| server      | String              |            | The url of the server from which to send the E-Mails from. (Must support SMTP)                             |
| name        | String              | "No Reply" | The name of the mailbox from which to send the E-Mails from.                                               |
| subscribers | List of subscribers | []         | The list of subscribers to which to send notifications to                                                  |
| filter      | [Filter](filter.md) | empty      | Global filtering rules to apply to messages.                                                               |

Subscribers can either be the E-Mail-Address or an object with an E-Mail-Address (`email`) and a custom filtering list (`filter`)


# Example
```toml
[email.notify]
address = "noreply@example.com"
password = "Password1234"
server = "mail.example.com"
name = "No Reply"
subscribers = [
    # send john all messages 
    "john@example.com",
    # but only send messages where the server didn't just go online to tim.
    { email="tim@example.com", filter.state.deny = [ { online = true } ] }
]
# filter out all messsages that change an attribute.
filter.state.deny = [ { attribute.event = "any" }, "create" ]
```