# Arcaea_server_rs
## YinMo19

正在开发中，相关文档见[Arcaea_server_rs_doc](https://docs.arcaea.yinmo19.top).

## 总览

这是一个关于 Arcaea Server Rust 版本的开发文档。我将使用 `rust/rocket` 进行开发。

先做一点简单的 Q&A。
- 这是什么？

    Arcaea 是一款很好玩的游戏...... 如果你不知道的话，可以去 [官网](https://arcaea.lowiro.com) 看看。

- 是否已经有案例？

    是的。 [Lost-MSth](https://github.com/Lost-MSth) 已经开发了一版 [Arcaea Server](https://github.com/Lost-MSth/Arcaea-Server)。 如果你现在想要使用的话，请前往他的仓库查看。他的项目已经是成熟的，可以游玩。

- 为什么还要开发？

    因为我正在学习 Rust，而且我已经想做一个这样的服务器挺久的了...... 另外，使用 `rust` 预期可以获得更高的性能，并且我还想要基于这个服务器再创建一个不仅仅只是管理员可用的后台，而是可以让所有玩家查询 b30、谱面信息的后台。并且我还预期在这个后台做一个小论坛、可供玩家讨论曲子。


可以看到目标还是很丰满的。当然并非空想，其中小论坛的雏形我已经写了一个简单的版本，可以在 [Chatroom](https://github.com/YinMo19/Chatroom)的 `release` 版本中试用。

如果不出意外的话，我将会在 2025年1月-3月之间积极开发，这个时间段我正好放寒假...... 如果你有兴趣的话可以观望...... 或者成为这个私服的测试者！帮我反馈问题也是对我很大的帮助。关于使用相关内容将不会在这里讲述。如果你想要使用本服务器，你可能需要一些简单的逆向知识。Arcaea 客户端对服务器地址进行了加密，你无法进行简单的更改。我的 [博客](https://blog.yinmo19.top/2024/11/13/Arcaea-API-%E5%9C%B0%E5%9D%80%E9%80%86%E5%90%91/) 中简单介绍了这部分的一点点内容，如果你有兴趣的话，可以参考一下。

本 Book 使用 `mdbook` 进行编写，这是大部分 rust 语言文档使用的方案，所以如果你已经是一个 rusty 的用户，你应该很熟悉！

最后将要非常非常非常感谢 [Lost-MSth](https://github.com/Lost-MSth) 以及他的服务器，他真的帮助了我很多！


关于我
---
如果你有什么关于这个项目的想法、建议、问题，可以通过这个邮箱联系我！
[Arcaea@yinmo19.top](mailto:Arcaea@yinmo19.top)