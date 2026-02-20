[English](../../README.md) | [Русский](../../README.ru.md)

# Coolbox Autofan REST API

[Coolbox AutoFan Pro](https://bitok.shop/automatic-regulyator-oborotov-coolbox-autofan-hiveos/)
is a board used by many crypto-miners to automatically control cooling fans of their mining rigs.
It's supported by HiveOS / RaveOS which are Linux distributions specialized on crypto-mining.

This project however makes it possible to utilize the device in any DIY Linux system,
regardless of its purpose. Moreover, you can install more than one autofan board, something
that is not possible with the original scripts of HiveOS. It's based on reverse engineering
of the original shell scripts and does the same job.

Everything you need is packed into a single executable binary. It connects to the autofan board
and provides a REST API to interact with it. The API is equivalent in functionality to the script
`coolbox` of the board available in HiveOS.

## Quick installation

First, make sure the device is connected to the PC.

Now you need to install the Rust compiler. Follow instructions from [rustup.rs](https://rustup.rs/) or run

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Now build and install this project

```shell
git clone git@github.com:kpot/coolbox-autofan-rs.git
cd coolbox-autofan-rs
cargo build --release
sudo cp target/release/coolbox-rs /usr/local/bin/coolbox-rs
```

If you don't have root privileges, copy the binary into any directory already listed in your `PATH` environment variable.

To ensure the device is accessible to the user that will be running the service, execute

```
sudo usermod -aG dialout $USER
```

In case your Linux is somehow different, and you get permission errors when you launch the server try to follow [this instruction on configuring access to serial ports in Linux](https://support.arduino.cc/hc/en-us/articles/360016495679-Fix-port-access-on-Linux).

## Usage and API documentation

Simply run `coolbox-rs` and then open `http://localhost:65231/docs/` URL from a browser **on the same device**. You'll find complete API documentation there, available to play with through the Swagger UI.

If you're running `coolbox-rs` on a different machine, launch it with `coolbox-rs --api-host 0.0.0.0`,
then you will be able to access the API using `http://<hostname-or-ip>:65231/` URL.
Learn about all other options by running `coolbox-rs --help`. Currently they are:

```shell
$ coolbox-rs --help

Usage: coolbox-rs [-c <coolbox-port>] [-h <api-host>] [-p <api-port>] [-d]

Coolbox Autofan Pro controller with REST API. Tested on firmware 1271 and PCB 1031.

Options:
  -c, --coolbox-port
                    serial port of the Coolbox Autofan board. Default:
                    "/dev/ttyUSB0"
  -h, --api-host    REST API host. Default: 127.0.0.1
  -p, --api-port    REST API port. Default: 65231
  -d, --dummy       a dummy mode, when a fake is used instead of a real device
  --help, help      display usage information
```

## Service Mode and monitoring device's output log

The device has a special "service mode" that allows you to look into some aspects of its
internal life. On HiveOS you could then look into the device's raw output by running `sudo screen -r coolbox`.

This ability is duplicated in this API.

You can activate the service mode by sending a message with [curl](https://curl.se/)

```shell
$ curl -X 'POST' \
  'http://localhost:65231/api/message' \
  -H 'Content-Type: application/json' \
  -d '{"text": "service_mode=1"}'
```

This should output something like `{"device_reply":"service mode ON \n\n"}` in reply.

Now you can watch what's happening on the device by running

```shell
$ curl -N 'http://localhost:65231/api/watch'

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=136 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=137 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=138 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90
```

## Logs

You can view or make the service write logs by [setting some environment variables](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).

## Useful References

The latest firmware for the device can be found [here](./extra/firmware)
