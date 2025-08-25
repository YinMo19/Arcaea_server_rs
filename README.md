# Arcaea Server Rust Edition

> 实现中...
> BUILDING...

这是一个使用 rust 实现的 arcaea 服务器，用于模拟 Arcaea 的主要功能。逻辑基本上完全重写 lost 大佬的 [Arcaea Server](https://github.com/Lost-MSth/Arcaea-server/)。水平一般，测试也少，总之可以当成玩具项目。 不过从性能上说应该会比 flask 版本更强一些，虽然本项目大概率也用不上高并发什么的。

## 开发环境
- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- MariaDB/MySQL database

怎么装就不说了。装好之后需要确认你已经启动了数据库，并创建一个专门用于这个后端的账号密码。之后
```sh
cd <this_proj>

# 拷贝完不要忘记修改里面的对应的内容
# 尤其是数据库的连接要记得改，默认账号密码是我自己的测试环境随便设置的
# 相信你一眼就知道这些内容是做什么的
cp .env.example .env
cp Rocket.toml.example Rocket.toml

# 用 cargo 装一个管理数据库的工具
cargo install sqlx

# 完成这步之前必须确认你的数据库已经好了
source .env && sqlx database create && sqlx migrate run

# 做完这一切之后，需要先初始化数据库，然后再开始跑
cargo run --bin init_db
cargo run
```
至于怎么部署上云，
```sh
cargo build --release
```
之后去 `target/release/<binary>`找到对应二进制 scp 到服务器上，数据库，配置文件，乐曲数据，热更新包等等都放到对应位置了，用你喜欢的方式持久化运行这个二进制就行了。

---

**注意**： 这是一个 Arcaea 的服务器实现，仅用于教育与展示目的。请**不要**用于商业目的，这不是强制要求，只是一个提醒和警告。

**Note**: This is a reimplementation of the Arcaea game server for educational and performance purposes. **DO NOT** use for commercial purposes, this is not a mandatory requirement, just a reminder and warning.

---
贡献代码？
---
真的有人想要和我一起写这个东西吗..... 有的话联系 arcaea@yinmo19.top，感激不尽。
目前代码问题不少，还在比较初期的阶段。不过登录功能以及最基础的一些功能已经完善了，框架也基本上搭好了，接下来就是按部就班的写(抄)一些 crud 就是了。

目前 linkplay 怎么写还没想好，不过车到山前必有路，实在不会问 ai。关于怎么 pua ai，我在 prompt目录下放了一个 txt，有兴趣可以看看这个文件以及这个文件的历史，里面是关于我 pua ai 帮我改代码的提示词。内容不少，挺好使的。

关于客户端的事情不要问我，请上网查找，真的很多的相信我。憋不住了可以给我发邮件 arcaea@yinmo19.top，但是我也不一定能解决。
