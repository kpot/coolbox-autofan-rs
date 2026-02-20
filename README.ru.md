# Coolbox Autofan REST API

[Coolbox AutoFan Pro](https://bitok.shop/automatic-regulyator-oborotov-coolbox-autofan-hiveos/) — это плата, которую используют многие майнеры криптовалют для автоматического управления вентиляторами охлаждения майнинговых ферм.
Плата поддерживается только в HiveOS / RaveOS — это специализированные на майнинге дистрибутивы Linux.

Данный проект, однако, делает возможным использование устройства в любой самодельной системе Linux,
независимо от её назначения. Кроме того, вы можете установить и одновременно использовать
более одной такой платы, что невозможно сделать с оригинальными скриптами HiveOS.
Этот проект основан на реверс-инжиниринге оригинальных скриптов и выполняет ту же работу.

Всё необходимое упаковано в один исполняемый файл. Он подключается к плате autofan
по указанному последовательному порту и предоставляет REST API для взаимодействия с платой.
API эквивалентен по функциональности [скрипту `coolbox`](https://github.com/minershive/hiveos-linux/blob/master/hive/opt/coolbox/coolbox) платы из состава HiveOS.

## Быстрая установка

Сначала убедитесь, что устройство подключено к компьютеру.

Теперь вам нужно установить компилятор Rust. Следуйте инструкциям на [rustup.rs](https://rustup.rs/) или выполните

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Теперь соберите и установите этот проект

```shell
git clone git@github.com:kpot/coolbox-autofan-rs.git
cd coolbox-autofan-rs
cargo build --release
sudo cp target/release/coolbox-rs /usr/local/bin/coolbox-rs
```

Если у вас нет прав суперпользователя, скопируйте бинарный файл в любой каталог, уже указанный в переменной среды `PATH`.

 Чтобы устройство было доступно пользователю, который будет запускать данный сервис, выполните

```
sudo usermod -aG dialout $USER
```

Если ваш Linux не совсем типичный, и вы получаете ошибки доступа при запуске сервера, попробуйте следовать [этой инструкции по настройке доступа к последовательным портам в Linux](https://support.arduino.cc/hc/en-us/articles/360016495679-Fix-port-access-on-Linux).

## Использование и документация API

Просто запустите `coolbox-rs`, затем откройте в браузере адрес `http://localhost:65231/docs/` **на том же устройстве**. Там вы найдёте полную документацию API, с которой можно поиграться через Swagger UI.

Если вы запускаете `coolbox-rs` на другой машине, запустите его с параметром `coolbox-rs --api-host 0.0.0.0`,
тогда вы сможете получить доступ к API по адресу `http://<имя-хоста-или-ip>:65231/`.
Узнайте обо всех остальных параметрах, выполнив `coolbox-rs --help`. Вот они, на данный момент:

```shell
$ coolbox-rs --help

Usage: coolbox-rs [-c <coolbox-port>] [-h <api-host>] [-p <api-port>] [-d]

Контроллер Coolbox Autofan Pro с REST API. Протестировано на прошивке 1271 и PCB 1031.

Options:
  -c, --coolbox-port
                    последовательный порт платы Coolbox Autofan. По умолчанию:
                    "/dev/ttyUSB0"
  -h, --api-host    хост REST API. По умолчанию: 127.0.0.1
  -p, --api-port    порт REST API. По умолчанию: 65231
  -d, --dummy       режим имитации, когда используется фейковое устройство вместо реального
  --help, help      показать информацию о использовании
```

## Режим обслуживания и мониторинг вывода устройства

У устройства есть специальный «режим обслуживания», который позволяет заглянуть в некоторые аспекты его
внутренней работы. В HiveOS вы могли посмотреть необработанный вывод устройства, выполнив `sudo screen -r coolbox`.

Эта возможность продублирована в данном API.

Вы можете активировать режим обслуживания, отправив сообщение с помощью [curl](https://curl.se/)

```shell
$ curl -X 'POST' \
  'http://localhost:65231/api/message' \
  -H 'Content-Type: application/json' \
  -d '{"text": "service_mode=1"}'
```

В ответ должно вывестись что-то вроде `{"device_reply":"service mode ON \n\n"}`.

Теперь вы можете наблюдать за происходящим на устройстве, выполнив

```shell
$ curl -N 'http://localhost:65231/api/watch'

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=136 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=137 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90

OCR0B=20 OCR0A(max)=200 fan_pwm=10 auto_mode=1 min_t=65 max_t=66 targ_t=70 pwm_add=-3 cnt=138 osccal=-2
min_mem_t=76 max_mem_t=80 targ_mem_t=90
```

## Логи

Вы можете просмотреть логи сервера, [установив некоторые переменные среды](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).

## Полезные ссылки

 Последнюю прошивку для устройства можно найти [здесь](./extra/firmware)
