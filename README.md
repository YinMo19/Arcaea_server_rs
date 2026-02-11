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

# 如果你要单独跑 linkplay 服务（UDP + TCP）
cargo run --bin linkplayd
```
至于怎么部署上云，
```sh
cargo build --release
```
之后去 `target/release/<binary>`找到对应二进制 scp 到服务器上，数据库，配置文件，乐曲数据，热更新包等等都放到对应位置了，用你喜欢的方式持久化运行这个二进制就行了。

### Link Play 独立进程配置
`linkplayd` 通过环境变量读取配置，推荐直接在 `.env` 里配置。关键项如下：

- `LINKPLAY_HOST`（默认 `0.0.0.0`）
- `LINKPLAY_UDP_PORT`（默认 `10900`）
- `LINKPLAY_TCP_PORT`（默认 `10901`）
- `LINKPLAY_AUTHENTICATION`
- `LINKPLAY_TCP_SECRET_KEY`

更多参数见 `.env.example` 里的 `Link Play Daemon Configuration` 段。

---

**注意**： 这是一个 Arcaea 的服务器实现，仅用于教育与展示目的。请**不要**用于商业目的，这不是强制要求，只是一个提醒和警告。

**Note**: This is a reimplementation of the Arcaea game server for educational and performance purposes. **DO NOT** use for commercial purposes, this is not a mandatory requirement, just a reminder and warning.

---
贡献代码？
---
真的有人想要和我一起写这个东西吗..... 有的话联系 arcaea@yinmo19.top，感激不尽。
目前代码问题不少，还在比较初期的阶段。不过登录功能以及最基础的一些功能已经完善了，框架也基本上搭好了，接下来就是按部就班的写(抄)一些 crud 就是了。

现在已经加了一个独立的 `linkplayd` 进程（`src/bin/linkplayd.rs`），用于把 Link Play 从主服务拆出来。当前已经实现控制面（TCP）与核心 UDP 二进制 parser（房间状态机、命令队列、倒计时流转），后面继续做逐条行为对齐和回归验证。

关于客户端的事情不要问我，请上网查找，真的很多的相信我。憋不住了可以给我发邮件 arcaea@yinmo19.top，但是我也不一定能解决。

代码架构
---
相信你看完我的 prompt 已经对这个项目有一些了解了，下面讲讲我对 rust 写 crud 的理解。采用的 rocket 框架确实是一个非常好写的框架，使用依赖注入的方式可以实现对各种 service 的全局管理，在需要的地方直接注入到对应的路由使用。项目类似于 django（但不同）的三级分层，route、 service、 model 层。即使我采用的不是 orm 架构，我依然把所有的数据库相关的数据结构专门用一个 model 层存起来。这一层专门用于构建结构体来对应数据库结构，以及构建一些返回体，包括实现一些这些模型的互相转换之类的方法。至于路由层和服务层想来不言自明。

rust 的 sqlx 框架相对别的语言都没有的一个最大的优点，是利用 rust 的宏机制实现编译期检查 sql 语句的正确性。他会在编译期连接一个真实的数据库，通过模拟代码中使用的 sql 来判断语句正确性。几乎可以这样说，只要能通过编译，那么写出来的 sql 语句就没有语法错误（但是性能/正确性两说，这些烂了谁也救不了）。再比如下面的代码中，
```rs
/// get user's stamina
async fn get_user_stamina(&self, user_id: i32) -> ArcResult<i32> {
    let stamina_info = sqlx::query!(
        "select max_stamina_ts, stamina from user where user_id = ?",
        user_id
    )
    .fetch_one(&self.pool)
    .await?;

    let stamina = Stamina {
        stamina: stamina_info.stamina.unwrap_or(12),
        max_stamina_ts: stamina_info.max_stamina_ts.unwrap_or(0),
    };

    Ok(stamina.calculate_current_stamina(12, 1800000))
}
```
使用 `sqlx::query!` 宏可以静态检查这句 sql 语句的返回值，他会自动把返回值的字段组合一个结构体，里面元素 `max_stamina_ts, stamina` 的类型则通过数据库中的字段类型来确定。如果数据库中的初始定义类型没有指定非 null，那么这个字段则会被自动解析为 `Option<T>`。这也算强类型的好处，因为在 python 中可能就得
```py
def select(self):
    '''获取用户体力信息'''
    self.c.execute('''select max_stamina_ts, staminafrom user where user_id = :a''',
                    {'a': self.user.user_id})
    x = self.c.fetchone()
    if not x:
        raise NoData('The user does not exist.')
    self.set_value(x[0], x[1])
```
使用 `x[0], x[1]` 这样的下表来借代每个返回值，对于长一些的查询语句就不太友好了，并且对于空值的处理有时候也会疏忽，可阅读性在这里反而 rust 会更高一些。

另外一个不错的点是错误类型。使用 thiserror 库可以实现很优秀的错误类型管理。只需要实现统一返回类型，并实现了每种可能出现的 error 到自定义 error 的 From 方法，那么使用起来就非常轻松。只需要在代码里面抛问号，错误就会留给框架自动序列化为特定的 json 丢回去给前端，这些所有内容都是可预见的，并且易于实现的。例如上面的案例中数据库查询最后 `.await?` 在失败的时候会抛出 `sqlx::Error`，而
```rs
/// Main error type for the Arcaea server
#[derive(Error, Debug)]
pub enum ArcError {
    ...

    /// Database error
    #[error("Database error: {message}")]
    Database { message: String },

    ...
}

impl From<sqlx::Error> for ArcError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database {
            message: err.to_string(),
        }
    }
}
```
既然已经实现了 From 方法，在可能错误的地方直接丢问号就行，错误自动就会序列化成我想要的模样。这也是强类型的一种好处吧。

最后是一个关于鉴权的内容。这是 rocket 提供的 auth 方案，他通过实现请求守卫的方式来进行所有需要对请求头的解析操作以及鉴权操作。这是一个非常有意思的点，因为实际上这样在使用上非常方便。例如我已经实现了对已登录用户的可访问守卫，那么对于任何想要让用户访问的 api，只需要在路由函数的参数中加上这个守卫就自动可以完成这个功能。又比如一些路由需要获取客户端 ip 和一些从 header 解析的信息，专门实现这种请求头之后直接在需要的函数参数中调用即可。这点和 python 的装饰器有点类似，不过这是 rust 的 rocket 框架的宏提供的功能，只能说宏还是太魔法了。

关于代码大概也就讲这些内容吧..... 这是我写过最大的后端项目，也是第一次采取这样的结构进行管理，也算是一种新的尝试。我以前写过 django，虽然并不喜欢，但是在新的项目中还是会不自觉的带上了那样的思维模式。虽然后端项目想来也大同小异，不过我自觉这样的代码写起来也算能看且实用，hah

最后，如果看到咕咕了大概率是我在忙忙，等我忙完了可能想起来就会继续更。这个暑假更了万多行代码，也算不错的进展了。

By YinMo19.
