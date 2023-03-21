# `mc-ping`: Simple Minecraft pinger for 1.7+

`mc-ping` is a simple Minecraft pinger for Minecraft versions 1.7+ with support for four notification methods.

The following features have to be enabled in order to use related notification methods:
- `firebase`: Firebase Cloud Messaging
- `discord`: Discord Webhook message
- `slack`: Slack Webhook message
- `custom`: Custom HTTP service

## Usage

```
$ ./mc-ping <hostname> [port]
```

In order to run `mc-ping` you need to pass a valid hostname or IP address to it, followed by a optional port number. Please not that `mc-ping` resolves `SRV` DNS records.

## Notifications

Each notification methods requires its own configuration file in the working directory.

Most strings in the configurations can contains placeholder values that will be replaced during runtime.

The following plaseholders are available:
- `%version` - Minecraft version
- `%description` - Description or Motd
- `%online` - Current number of players
- `%max` - The maxium number of players
- `%players` - A list of sample player names. By default separated by a new line.
- `%hostname` - Raw hostname given to the program
- `%host` - Hostname of the server. Normally will be the same as `%hostname` with the exception if a SRV DNS record resolved a different value
- `%port` - Port number of the server

The following configuration samples contain the default values. All non required fields can be safely omitted.

### Firebase

The following configuration must be in a `firebase.json` file in the current working directory:

```json
{
    // This field is required.
    // The key used to authorize requests to Firebase
    "key": "<Firebase Cloud Messaging Server Key>",

    // =====
    // The following fields are a part of the request payload.
    // You can look up valid values in the official documentation

    // The recipiant of the notification
    "to": "/topics/all",

    // List of recipiants of the notification 
    "registration_ids": null,

    // The condition recipiant must meet in order to receive the notification
    "condition": null,

    // An identifier for the messages that allows for grouping them
    "collapse_key": null,

    // The proiority of the notification
    "priority": null,

    // How long (in seconds) the notification should live on the Firebase servers
    "tile_to_live": null,

    // Custom key-value data sent alongside the notification
    "data": null,

    // The normal Notification payload
    "notification": {
        "title": "Status change: %online/%max",
        "body": "Server: %host:%port\nPlayers:\n%players"
    },

    // The Notification payload when there is no players online.
    // If not provided, the normal notification payload will be used instead
    "empty_notofication" : {
        "title": "Status change: %online/%max",
        "body": "Server: %host:%port",
    },
    // =====

    // The separator for the "%players" placeholder.
    "players_separator": "\n"
}
```

For this notification method to work, you need the Firebase Cloud Messaging Server Key. You can read about it [here.](https://firebase.google.com/docs/cloud-messaging/auth-server#authorize-http-requests)


You can find the Notification payload model in [the official documentation.](https://firebase.google.com/docs/cloud-messaging/http-server-ref#notification-payload-support)

### Discord

The following configuration must be in a `discord.json` file in the current working directory:

```json
{
    // This field is required.
    "webhook": "<Discord Webhook URL>",

    // Normal message body
    "message": {
        "username": "%host:%port",
        "content": "Status change: %online/%max\nPlayers:```%players```"
    },

    // Message body send when there is no players online.
    // If not provided, the normal message body will be used instead
    "empty_message": null,

    // The separator for the "%players" placeholder.
    "players_separator": "\n"
}
```

You can find message body model in [the official documentation.](https://discord.com/developers/docs/resources/webhook#execute-webhook-jsonform-params)

> Please not that you can't upload any files

### Slack

The following configuration must be in a `slack.json` file in the current working directory:

```json
{
    // This field is required.
    "webhook": "<Slack Webhook URL>",

    // Normal message body
    "message": {
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "Status change: %online/%max\nPlayers:```%players```"
                }
            }
        ]
    },

    // Message body send when there is no players online.
    // If not provided, the normal message body will be used instead
    "empty_message": null,

    // The separator for the "%players" placeholder.
    "players_separator": "\n"
}
```

You can find message body model in [the official documentation.](https://api.slack.com/reference/messaging/payload)

### Custom

The following configuration must be in a `custom.json` file in the current working directory:

```json
{
    // This field is required.
    // A valid HTTP URL. Placeholders will not be replaced
    "url": "<HTTP Endpoint URL>",

    // Key-Value pairs of headers that will be attached to the requests.
    // Any placeholders in the values will be replaced
    "header": null,

    // Custom data to send alongside the Minecraft server status.
    // Any placeholders in the values will be replaced
    "custom_data": null,

    // The separator for the "%players" placeholder.
    "players_separator": "\n"
}
```

Custom data is simply a JSON object that can contain anything.

Custom notification methods sends the retived Minecraft server status directrly (or with additional data if provided) to a given HTTP endpoint as a PORT request.
