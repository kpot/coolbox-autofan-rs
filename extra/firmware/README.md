# Coolbox Autofan Pro firmware

This directory contains latest firmware for Coolbox Autofan Pro board (PCB rev 1031h).

The firmware here was [published by the developers of the board] (https://static.insales-cdn.com/files/1/5891/22165251/original/m328p_1031_1271.hex) for anyone to use and also [distributed](https://t.me/c/1670158184/5157) through the official Telegram channel.

In order to flash the board, connect it by USB and flash using `avrdude` command line tool.

First, make sure any processes working with the device are turned off.
Then, from this directory, assuming the device is connected to /dev/ttyUSB0, run this command:

``` shell
sudo avrdude -p m328p -c arduino -C avrdude.conf -P /dev/ttyUSB0 -b 9600 -U flash:w:m328p_1031_1271.hex:a
```

If this doesn't work for you, you might find some help in [this Telegram group](https://t.me/c/1670158184/8473).
