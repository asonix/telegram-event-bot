# Telegram Event bot

This bot exists to manage social events happening for connected chats on Telegram

### Usage

You can talk to this bot on Telegram to get it added to your chats. The bot can be found at [t.me/coconuts_event_bot](https://t.me/coconuts_event_bot).

#### If you're the admin of a chat, and want to use this bot, you can follow the following steps.

1. Create a channel where the bot can announce events. Add this bot as an admin of your channel.
2. In your channel, issue the `/init` command. This will tell the bot that you want it to keep track of your channel.
3. Add the bot as an admin of your chat. This way, the bot can keep track of who exists in the chat, and grant permissions to add/modify/delete events only to users present in your chat.
4. Get the ID of your chat. You can do this by issuing the `/id` command in the chat.
5. In your channel, issue the command `/link id` where `id` is the chat Id you got from the previous step. This tells the bot that users in your chat are allowed to create events for this channel.

#### If you are in a chat that uses this bot, you can use the following steps to create an event

1. Send a message in the group chat (if you haven't already). The event bot uses messages to determine who is present in a chat, since Telegram doesn't offer an API that exposes this information.
2. Open a private chat with the bot and issue the `/new` command. The bot will ask you which channel associated with your chats you'd like to create an event for.
3. Select the channel you want to create an event for, the bot will generate a one-time-use link to a web form that will allow you to create an event.
4. Use the link to create the event.

Available commands:
```
/init - Initialize an event channel
/link - link a group chat with an event channel (usage: /link [chat_id])
/id - get the id of a group chat
/events - get a list of events for the current chat
/new - Create a new event (in a private chat with the bot)
/edit - Edit an event you're hosting (in a private chat with the bot)
/delete - Delete an event you're hosting (in a private chat with the bot)
/help - Print this help message
```

### Development

If you want to help develop this bot, you'll need to know how it works. This bot is backed by a Postgres database, and uses the `tokio-postgres` crate for database interaction, but it uses the `diesel_cli` application to manage migrations. You'll need to `cargo install diesel_cli` and then run the migrations through diesel to get a dev environment set up.

This bot uses dotenv to help manage environment variables. A sample `.env` file has been provied as `.env.sample`. Copy this file to `.env` and then set variables that make sense for your setup. This means you'll need to have a postgres database with an events table.

```
# .env.sample

# The following variables are used by telegram-event-bot
DB_HOST="localhost"
DB_PORT="5432"
DB_USER="events"
DB_PASS="events"
DB_NAME="events"
TEST_DB_NAME="events_test"
EVENT_URL="localhost:8000"
TELEGRAM_BOT_TOKEN="your bot token"

# This variable is used by diesel_cli
DATABASE_URL="postgres://events:events@localhost:5432/events"
```

The application is sectioned into three parts, the model in `src/model`, the actors in `src/actors`, and the `main.rs` file. The model defines functions that execute database queries, the actors manage application state and hold the application logic, and the main file starts the actors with the required arguments.

There are currently 5 actors comprising this application.
 - DbBroker, which manages access to the database connections
 - EventActor, which handles interaction with the Web UI
 - TelegramActor, which recieves updates from, and sends messages to Telegram
 - Timer, which manages notifying when events are soon, starting, and ending.
 - UsersActor, which is an in-memory cache of useful relations between users, chats, and channels

The model has 6 modules that are useful
 - chat, which handles interaction with the chats table
 - chat_system, which handles interaction with the chat_systems table. This table knows the channel associated with a given set of chats
 - edit_event_link, which stores one-time-use links generated to edit events. This interacts with the edit_event_links table
 - event, which stores information about events that have been created. This interacts with the events table
 - new_event_link, which stores one-time-use links generated to create events. This interacts with the new_event_links table
 - user, which interacts with the users table. This is used for keepng track of which users are in which chats

Additionally, there is a crate within this repository dedicated to handling the Web UI. That crate creates a series of actors to serve web requests, and communicates back to the EventActor with user-provided information.

### Contributing
Feel free to open issues for anything you find an issue with. Please note that any contributed code will be licensed under the GPLv3.

### License

Copyright © 2018 Riley Trautman

Telegram Event Bot is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

Telegram Event Bot is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. This file is part of Telegram Event Bot.

You should have received a copy of the GNU General Public License along with Telegram Event Bot. If not, see [http://www.gnu.org/licenses/](http://www.gnu.org/licenses/).
