# dlux - hardware brightness control
dlux (pronunced like "deluxe") is a tiny hardware brightness control daemon that
automatically sets hardware display brightness based on the time of day.


dlux requires a monitor that supports [ddc/ci](https://en.wikipedia.org/wiki/Display_Data_Channel#DDC/CI).
Unlike [f.lux](https://justgetflux.com/) or [redshift](https://github.com/jonls/redshift), dlux:
  * has zero performance impact
  * runs in ~2 KiB
  * works with any Linux setup and without a graphical session

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
dlux 0.1.0

USAGE:
    dlux [FLAGS] [OPTIONS] --brightness <brightness> --lat <lat> --long <long>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --brightness <brightness>    out of 100, the brightness to target for the screen
        --height <height>             [default: 0.0]
        --lat <lat>
        --long <long>
```

After compiling, provide your latitude, longitude, and desired nighttime brightness:
```
$ dlux --brightness 40 --lat 40 --long="-120"
found 3 devices, beginning loop
updating brightness to 100
Waiting 6h 34m 4s until 2020-11-01 16:58:41.100 -08:00
```
