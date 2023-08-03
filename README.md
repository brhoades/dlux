# dlux - hardware brightness control
dlux (pronunced like "deluxe") is a tiny hardware brightness control daemon that
automatically sets hardware display brightness based on the time of day.


dlux requires a monitor that supports [ddc/ci](https://en.wikipedia.org/wiki/Display_Data_Channel#DDC/CI).
Unlike [f.lux](https://justgetflux.com/) or [redshift](https://github.com/jonls/redshift), dlux:
  * has zero performance impact with a tiny memory footprint
  * reduces electricity use
  * works with any Linux setup, even without a graphical session

## Requirements
dlux requires userspace access to i2c devices through the [i2c-dev](https://www.kernel.org/doc/Documentation/i2c/dev-interface)
module. This module must be loaded prior to launching dlux.


Additionally, your displays must support i2c brightness control (capability `0x10`).
You may preemptively check display capabilities through `ddcutil`. For example:

```bash
$ sudo ddcutil detect
Display 1
   I2C bus:             /dev/i2c-1
   EDID synopsis:
      Mfg id:           DEL
      Model:            DELL U2415
      Serial number:    DEADBEEF1744
      Manufacture year: 2019
      EDID version:     1.4
   VCP version:         2.1

Display 2
   I2C bus:             /dev/i2c-3
   EDID synopsis:
      Mfg id:           DEL
      Model:            DELL U2415
      Serial number:    JKLHF1200919
      Manufacture year: 2019
      EDID version:     1.4
   VCP version:         2.1

Display 3
   I2C bus:             /dev/i2c-6
   EDID synopsis:
      Mfg id:           DEL
      Model:            DELL U2720Q
      Serial number:    ABCD1234
      Manufacture year: 2020
      EDID version:     1.3
   VCP version:         2.1

$ sudo ddcutil getvcp 0x10 --bus 1 # from /dev/i2c-1
VCP code 0x10 (Brightness): current value = 100, max value = 100

$ sudo ddcutil getvcp 0x10 --bus 3 # /dev/i2c-3
VCP code 0x10 (Brightness): current value = 100, max value = 100

$ sudo ddcutil getvcp 0x10 --bus 6 # /dev/i2c-6
VCP code 0x10 (Brightness): current value = 100, max value = 100
```

## Usage
`dlux` has three subcommands: `probe`, `daemon`, and `run`. `run` and `daemon`
are functionally identically except `daemon` takes a config file and `run` uses
entirely CLI parameters.

```
$ dlux --help
dlux 0.1.2
Dynamic hardware monitor brightness adjustment

USAGE:
    dlux <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    daemon
    help      Prints this message or the help of the given subcommand(s)
    probe
    start

```

### Daemon
The daemon takes a yaml configuration file as the first and only parameter:

```
$ dlux daemon --help
dlux-daemon 0.1.2

USAGE:
    dlux daemon <config>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <config>

$ dlux daemon config.yaml
[2020-12-19T22:33:01Z INFO  dlux::daemon] discovered 3 monitors
[2020-12-19T22:33:01Z INFO  dlux::daemon] updating brightness of all displays to nighttime value
[2020-12-19T22:33:01Z DEBUG lib::device] set brightness for /dev/i2c-1 to 40% (absolute 40)
[2020-12-19T22:33:01Z DEBUG lib::device] set brightness for /dev/i2c-3 to 40% (absolute 40)
[2020-12-19T22:33:01Z DEBUG lib::device] set brightness for /dev/i2c-5 to 40% (absolute 40)
[2020-12-19T22:33:01Z DEBUG dlux::daemon] finished setting monitor brightness
[2020-12-19T22:33:01Z INFO  dlux::daemon] sleeping for 1h 50m 55s until 2020-12-19 16:23:57.100 -08:00
```

Minimal configuration which manages all compatible devices automatically:
```yaml
geo:
  latitude: 20
  longitude: -100
day_brightness: 100
night_brightness: 40
```

Configuration file with explicit device matches and per-device overrides of global settings.
Note that all devices must be matched by the list.
```yaml
geo:
  latitude: 20
  longitude: -100
  altitude: 200
logging:
  level: Debug
day_brightness: 100
night_brightness: 40
devices:
  - model: (?i)dell U2145
    day_brightness: 80
    night_brightness: 50
    manufacturer_id: DEL
  - model: DELL U2145
```

Configuration file with explicit device matches and per-device overrides of global settings.
By default, devices matched are overriden but all devices are managed, but `device_match_exclusive`
can make it so only matched devices are handled.
```yaml
geo:
  latitude: 20
  longitude: -100
day_brightness: 100
night_brightness: 40
device_match_exclusive: true
devices:
  - model: (?i)dell U2145
  - model: DELL U2145
```

### Legacy CLI
`dlux start` mirrors classic CLI usage. It will run a daemon indefinitely based on passed parameters.
It does not accept a config file.

```
$ dlux start
dlux-start 0.1.2

USAGE:
    dlux start [OPTIONS] --latitude <latitude> --longitude <longitude>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --altitude <altitude>
            altitude from sea level in meters of your location for sunset calculations [default: 0.0]

    -d, --day-brightness <day-brightness>        percentage of the target screen brightness during day
        --latitude <latitude>                    latitude of your location for sunset calculations
        --log-level <level>
            minimum log level printed to STDERR. Choose from: trace, debug, info, warn, error, off [default: info]

        --longitude <longitude>                  longitude of your location for sunset calculations
    -n, --night-brightness <night-brightness>    percentage of the target screen brightness after sunset
        --log-style <style>
            controls when log output is colored. Choose from: auto, always, and never [default: Auto]
```

After compiling, provide your latitude, longitude, and desired nighttime brightness:
```
$ dlux --day-brightness 100 --night-brightness 40 --latitude 40 --longitude="-120"
[2020-12-19T22:27:51Z INFO  dlux::daemon] discovered 3 monitors
[2020-12-19T22:27:51Z INFO  dlux::daemon] updating brightness of all displays to daytime value
[2020-12-19T22:27:52Z INFO  dlux::daemon] sleeping for 18h 21m 29s until 2020-12-20 08:49:21.100 -08:00
```

### Probe
`dlux probe` provides valid config yaml for all the currently available supported devices:

```
$ dlux probe
---
devices:
  - model: PA278CV
    manufacturer_id: AUS
    serial: M3LMQS362198
  - model: DELL U2720Q
    manufacturer_id: DEL
    serial: F8KFX13
  - model: ASUS PB277
    manufacturer_id: ACI
    serial: ""
```
