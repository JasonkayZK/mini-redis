# mini-redis

[![CI](https://github.com/JasonkayZK/mini-redis/workflows/CI/badge.svg)](https://github.com/JasonkayZK/mini-redis/actions)



## **前言**

tokio 官方文档如下：

-   https://tokio.rs/tokio/tutorial

项目的目录结构如下：

```bash
$ tree ./src/
.
├── bin
│   ├── cli.rs
│   └── server.rs
├── client
│   ├── cli.rs
│   ├── cmd.rs
│   ├── mod.rs
│   └── subscriber.rs
├── cmd
│   ├── get.rs
│   ├── mod.rs
│   ├── ping.rs
│   ├── publish.rs
│   ├── set.rs
│   ├── subscribe.rs
│   ├── unknown.rs
│   └── unsubscribe.rs
├── config.rs
├── connection
│   ├── connect.rs
│   ├── frame.rs
│   ├── mod.rs
│   └── parse.rs
├── consts.rs
├── error.rs
├── lib.rs
├── logger.rs
├── server
│   ├── handler.rs
│   ├── listener.rs
│   ├── mod.rs
│   └── shutdown.rs
└── storage
    ├── db.rs
    ├── mod.rs
    ├── store.rs
    └── traits.rs
```

其中：

-   `bin` 目录：server 和 cli 的命令行入口可执行文件；
-   `client` 目录：客户端具体实现逻辑；
-   `server` 目录：服务端具体实现逻辑；
-   `cmd` 目录：mini-redis 相关命令实现；
-   `connection` 目录：客户端、服务端异步连接实现；
-   `storage` 目录：kv、subscribe 存储实现（本例中直接使用 HashMap 实现，实际生产环境多用 LSM-Tree）；
-   `config.rs`：mini-redis 配置相关；
-   `consts.rs`：mini-redis 常量配置相关；
-   `error.rs`：mini-redis 错误定义；
-   `logger.rs`：mini-redis 日志配置；
-   `lib.rs`：mini-redis 库入口；

总体分为下面几个部分：

-   **存储实现；**
-   **连接实现；**
-   **具体命令实现**
-   **客户端、服务端实现；**

<br/>

## **基本使用**

首先启动server：

```bash
$ cargo run --bin mini-redis-server

[ INFO]: mini_redis::server - mini-redis server started listen on: 0.0.0.0:6379
[ INFO]: mini_redis::server::listener - server started, accepting inbound connections
```

随后可以使用 client：

```bash
$ cargo run --bin mini-redis-cli

mini-redis-cli 0.1.0
Issue Redis commands

USAGE:
    mini-redis-cli [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help                   Print help information
        --hostname <hostname>    [default: 127.0.0.1]
        --port <PORT>            [default: 6379]
    -V, --version                Print version information

SUBCOMMANDS:
    get          Get the value of key
    help         Print this message or the help of the given subcommand(s)
    ping         
    publish      Publisher to send a message to a specific channel
    set          Set key to hold the string value
    subscribe    Subscribe a client to a specific channel or channels
```

<br/>

ping命令测试：

```bash
$ cargo run --bin mini-redis-cli ping   
"PONG"

$ cargo run --bin mini-redis-cli ping abc
"abc"
```

<br/>

get/set 测试：

```bash
$ cargo run --bin mini-redis-cli get foo     
(nil)

$ cargo run --bin mini-redis-cli set foo 123
OK

$ cargo run --bin mini-redis-cli get foo    
"123"
```

过期键测试，设置 5s 过期：

```bash
$ cargo run --bin mini-redis-cli set foo 123 5000
```

获取：

```bash
$ cargo run --bin mini-redis-cli get foo
"123"

$ cargo run --bin mini-redis-cli get foo
(nil)
```

5s后，获取不到 key 值了！

<br/>

pub/sub 测试；

启动三个 subscribe，订阅同一个 channel，ch1：

```bash
$ cargo run --bin mini-redis-cli subscribe ch1

$ cargo run --bin mini-redis-cli subscribe ch1

$ cargo run --bin mini-redis-cli subscribe ch1
```

向 ch1 发布消息：

```bash
$ cargo run --bin mini-redis-cli publish ch1 a-message
Publish OK
```

其他订阅者均收到消息：

```
got message from the channel: ch1; message = b"a-message"
```

<br/>

错误命令测试：

```bash
$ cargo run --bin mini-redis-cli ping get foo

error: Found argument 'foo' which wasn't expected, or isn't valid in this context
```

<br/>

## **相关文章**

系列文章：

-   [《mini-redis项目-1-简介》](https://jasonkayzk.github.io/2022/12/05/mini-redis项目-1-简介/)
-   [《mini-redis项目-2-存储层》](https://jasonkayzk.github.io/2022/12/05/mini-redis项目-2-存储层/)
