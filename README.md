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
```
$ dlux --help
dlux 0.1.1

USAGE:
    dlux [OPTIONS] --brightness <brightness> --latitude <latitude> --longitude <longitude>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --altitude <altitude>        altitude from sea level in meters of your location for sunset calculations
                                     [default: 0.0]
    -b, --brightness <brightness>    percentage of the target screen brightness at sunset
        --latitude <latitude>        latitude of your location for sunset calculations
        --log-level <level>          minimum log level printed to STDERR. Choose from: trace, debug, info, warn, error,
                                     off [default: info]
        --longitude <longitude>      longitude of your location for sunset calculations
        --log-style <style>          controls when log output is colored. Choose from: auto, always, and never [default:
                                     Auto]
```

After compiling, provide your latitude, longitude, and desired nighttime brightness:
```
$ dlux --brightness 40 --latitude 40 --longitude="-120"
[2020-11-01T23:10:34Z INFO  dlux] discovered 3 monitors
[2020-11-01T23:10:34Z INFO  dlux] updating brightness of all displays to 100
[2020-11-01T23:10:34Z INFO  dlux] sleeping for 18h 37m 6s until 2020-11-02 09:47:41.100 -08:00
```
