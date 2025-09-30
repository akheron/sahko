# Sahko

A simple program for controlling electrical devices by turning GPIO pins on and off on a Raspberry Pi based on the
electricity spot price.

Now also includes an experimental web interface for editing the schedules.

## Prerequisites

- One or more devices you want to control, e.g. a boiler, heaters, etc.
- Raspberry Pi with relays connected to GPIO pins so that you can turn the devices on and off, or configure them in
  some other way
- Internet connection on the Raspberry Pi
- Rust toolchain and [cross] for easy cross compilation on your development machine

## How it works?

The program is meant to be run periodically, for example every minute using cron.

Each time it is run, it will do the following:

- Load the schedules for today. If they don't exist yet, it will fetch the spot prices for today using the
  [spot-hinta.fi] API and compute a schedule for each configured GPIO pin based on the prices.
- If the current time of day is 17:00 or later, it will do the same for tomorrow's schedules.
- Check that each controlled GPIO pin is in the correct state according to today's schedules and turn it on or off if
  necessary.
- If email settings are configured, the program will send an email when a new schedule is computed or when a pin's
  state is changed.

This design aims for robustness and security:

- There's no need to keep the program running and restart it if it crashes or the Raspberry Pi reboots.
- No unnecessary writing to the filesystem: the schedules are only written once per day. This will prolong the life of
  the SD card.
- No unnecessary network traffic: the spot prices are only fetched once per day. If the network connection is not
  available, the request will be retried on the next run.
- No user interface: updates are pushed to the user via email.

## Setup

Create `config.json`, see [below](#config) for reference.

Build for Raspberry Pi using [cross]:

```
$ cross build --bin sahko --target arm-unknown-linux-gnueabihf --release
```

Copy `config.json` and `target/arm-unknown-linux-gnueabihf/release/sahko` to the Raspberry Pi.

Edit crontab:

```
$ crontab -e
```

Assuming you have the binary and the config in Raspberry Pi's home directory,
add the following line to run `sahko` every minute:

```
* * * * *  cd $HOME; ./sahko
```

That's it!

## Config

Example config:

```
{
  "schedules": [
    {
      "name": "Heater",
      "pin": 17,
      "low_limit": 2.0,
      "high_limit": 8.0,
      "min_on_hours": 4,
      "max_on_hours": 18
    }
  ],
  "email": {
    "server": "smtp.yourdomain.com",
    "username": "your_username",
    "password": "your_password",
    "from": "sahko <noreply@yourdomain.com>",
    "to": ["me@mydomain.com"]
  }
}
```

### Schedules

The config file contains a list of schedules, one for each pin you want to control. Optional fields can be set to
`null` or omitted.

- `name`: Name of the schedule, used only for display purposes
- `pin`: GPIO pin (Broadcom numbering)
- `low_limit` (optional): Electricity price limit at or below which the pin is always on, up to `max_on_hours` per day
- `high_limit` (optional): Electricity price limit above which the pin is always off
- `min_on_hours`: Minimum number of hours the pin should be on per day, when the price is between `low_limit` and
  `high_limit`
- `max_on_hours`: Maximum number of hours the device should be on per day, when the price is below `low_limit`
- `min_consecutive_hours`: Minimum consecutive hours to keep the switch on in the middle of the day

### Email

The email section is optional. If it is present, the program will send updates to the specified email addresses.

Fields:

- `server`: SMTP server address
- `username`: SMTP username
- `password`: SMTP password
- `from`: Sender address
- `to`: List of recipient addresses

[cross]: https://github.com/cross-rs/cross

[spot-hinta.fi]: https://spot-hinta.fi

## Development

If the program is build on some other OS than Linux, a mock implementation of the GPIO interface will be used. This
allows the program to be built and tested.

Run tests:

```
$ cargo test
```

### Cross-compiling on macOS

Install the required toolchains via Homebrew, see https://github.com/messense/homebrew-macos-cross-toolchains.

Add the following to your `.zshrc` or `.bashrc`:

```
# See https://github.com/messense/homebrew-macos-cross-toolchains
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc
export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
```

## Minimizing disk writes on Raspberry PI

Edit `/etc/systemd/journald.conf`. Set `Storage=volatile` under `[Journal]` to log to ram.

Edit `/etc/default/tailscaled`. Add `-no-logs-no-support` to `FLAGS` to reduce disk writes.
