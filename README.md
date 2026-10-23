# 专业阶段 - rCore-Tutorial

基于 [LearningOS 2026s rCore 课程模板](https://github.com/LearningOS/2026s-oscamp-professional-2026s-rcore-rCore-Tutorial-Code) 整理，由 [LearningOS](https://github.com/LearningOS) 统一分配学员仓库、自动评测并同步 OpenCamp。

**五项实验：每项 100 分 · 总分：500 分**

## 开始实验

1. 加入 [OpenCamp 秋冬季训练营](https://opencamp.cn/os2edu/camp/2026fall)，并绑定自己的 GitHub 账号。
2. 使用课程方提供的领取链接，点击 **Create** 提交申请；等待机器人回复，然后接受仓库邀请。
3. 克隆分配的作业仓库，切换章节分支，完成实验代码和报告。
4. push 到 `ch3`、`ch4`、`ch5`、`ch6` 或 `ch8`，在 Actions 查看评测；通过后自动上传累计成绩。

分配的仓库包含 `main` 与 `ch1` 至 `ch8`。把下面的 `YOUR_GITHUB_LOGIN` 替换为自己的 GitHub 登录名：

```sh
git clone https://github.com/LearningOS/2026a-rcore-YOUR_GITHUB_LOGIN.git
cd 2026a-rcore-YOUR_GITHUB_LOGIN
git switch ch3
```

依次下载自己的作业仓库、进入目录、切换到第一个计分实验分支。使用 SSH 的同学可从仓库 **Code → Local → SSH** 复制地址；请先在自己的 GitHub 账号配置 SSH 公钥。

## 教材与代码资料

先阅读 [rCore 实验指导](https://learningos.github.io/rCore-Tutorial-Guide/)，需要更完整的原理说明时配合 [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)。实验从基本执行环境逐步发展到内存管理、进程、文件系统与并发。

| 章节 | 上游 OS API 文档 |
| --- | --- |
| 第 1 章 | [ch1 API](https://learningos.github.io/rCore-Tutorial-Code/ch1/os/index.html) |
| 第 2 章 | [ch2 API](https://learningos.github.io/rCore-Tutorial-Code/ch2/os/index.html) |
| 第 3 章 | [ch3 API](https://learningos.github.io/rCore-Tutorial-Code/ch3/os/index.html) |
| 第 4 章 | [ch4 API](https://learningos.github.io/rCore-Tutorial-Code/ch4/os/index.html) |
| 第 5 章 | [ch5 API](https://learningos.github.io/rCore-Tutorial-Code/ch5/os/index.html) |
| 第 6 章 | [ch6 API](https://learningos.github.io/rCore-Tutorial-Code/ch6/os/index.html) |
| 第 7 章 | [ch7 API](https://learningos.github.io/rCore-Tutorial-Code/ch7/os/index.html) |
| 第 8 章 | [ch8 API](https://learningos.github.io/rCore-Tutorial-Code/ch8/os/index.html) |

进一步阅读：[第 9 章 API](https://learningos.github.io/rCore-Tutorial-Code/ch9/os/index.html)、[往期学习资源](https://github.com/LearningOS/rust-based-os-comp2025/blob/main/relatedinfo.md)、[上游教学代码](https://github.com/LearningOS/rCore-Tutorial-Code)。第 9 章作为拓展资料，本仓库的课程分支到 `ch8`。

## 环境配置与运行

按照 [实验指导第零章](https://learningos.github.io/rCore-Tutorial-Guide/0setup-devel-env.html) 准备 Linux、Rust 和 QEMU。章节中的 `rust-toolchain.toml` 指定课程使用的 Rust 版本；本仓库使用 `nightly-2024-05-02`，目标为 `riscv64gc-unknown-none-elf`。

### 固定工具版本与 RustSBI

`ch1` 至 `ch8` 使用以下版本，避免本地环境与课程构建要求不一致：

| 组件 | 版本 |
| --- | --- |
| 内核与用户程序的 Rust 工具链 | `nightly-2024-05-02` |
| `cargo-binutils` | `0.4.0` |
| QEMU | `8.2.x`（Docker 与 GitHub 评测使用 `8.2.2`） |
| 随仓库提供的 RustSBI 固件 | `0.4.1` |

各章节的 `bootloader/rustsbi-qemu.bin` 是已构建的 RustSBI jump 固件，供 QEMU 启动位于 `0x80200000` 的内核，日常实验不需要重新构建固件。来源、构建工具链与 SHA-256 记录在 `bootloader/rustsbi-qemu.toml`；对应依赖锁文件和许可证一并保存在 `bootloader/`。固件自身的构建工具链与上表中的课程内核工具链不同，以该配置文件为准。

`ch3` 至 `ch8` 已同步 SBI System Reset 关机接口适配。不要用旧版本固件替换课程提供的文件；使用旧 Docker 镜像时，需按下面的步骤重新构建镜像，才能使用固定后的构建环境。

### 运行章节代码

在已切换到 `ch3` 的作业仓库根目录，首次下载用户程序：

```sh
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git user
```

这条命令下载用户程序。已经有 `user/` 时复用该目录，不要重复克隆。

```sh
make -C os run
```

`-C os` 在内核目录执行 `make run`，构建本章的内核、用户程序并启动 QEMU。修改代码后再次运行即可。

<details>
<summary>使用仓库提供的 Docker 环境</summary>

安装并启动 Docker 后，在已切换到章节分支的**仓库根目录**执行：

```sh
make build_docker
make docker
```

先从仓库的 Dockerfile 构建环境，再进入挂载了本地仓库的容器。容器内工作目录为 `/mnt`，可运行上面的 `make -C os run`。环境使用 QEMU 8.2.2 和 `nightly-2024-05-02`，首次构建会下载 QEMU 和 Rust 工具链。

新版默认镜像名为 `rcore-docker-qemu8`，容器名为 `rcore-container-qemu8`，不会复用或删除旧的 QEMU 7 容器。旧容器内未放在挂载目录中的文件，请自行保留。

网络配置参考 Docker 官方的[镜像下载代理](https://docs.docker.com/reference/cli/docker/image/pull/#proxy-configuration)和[构建代理](https://docs.docker.com/engine/cli/proxy/#build-with-a-proxy-configuration)。

</details>

## 本地评测

内核能启动后，使用官方检查器验证实验要求。以下是 `ch3` 的操作，在已配置实验环境的仓库根目录执行。首次下载检查器及其测试程序：

```sh
git clone https://github.com/LearningOS/rCore-Tutorial-Checker.git ci-user
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git ci-user/user
```

这两条命令下载官方检查器和测试程序。已有对应目录时复用，不要删除自己的源码或报告。

```sh
make -C ci-user test CHAPTER=3
```

运行第三章测试并检查报告。后续切换到 `ch4`、`ch5`、`ch6`、`ch8` 时，将 `CHAPTER` 改为对应数字。只有全部测试、报告检查均通过且命令成功退出，才算本章完成；查看最终 `N/N` 时也要留意后续报错。

## 分支与实验报告

| 分支 | 内容 | 分值 | 必须提交的报告 |
| --- | --- | ---: | --- |
| `main` | 课程入口与操作指南 | — | — |
| `ch1` | 应用程序与执行环境 | — | — |
| `ch2` | 批处理系统 | — | — |
| `ch3` | 多道程序与分时多任务 | 100 | `lab1` |
| `ch4` | 地址空间 | 100 | `lab1`、`lab2` |
| `ch5` | 进程管理 | 100 | `lab1` 至 `lab3` |
| `ch6` | 文件系统与 I/O | 100 | `lab1` 至 `lab4` |
| `ch7` | 进程间通信 | — | — |
| `ch8` | 并发与同步 | 100 | `lab1` 至 `lab5` |

报告放在 `reports/`，文件名为 `lab1.md` 或 `lab1.pdf` 等。请提交真实实验报告；后续章节仍需保留此前报告。

## 提交代码与报告

以 `ch3` 为例，完成代码并编写 `reports/lab1.md` 或 `reports/lab1.pdf` 后：

```sh
git diff
git add os reports
git commit -m "Complete rCore chapter 3"
git push origin ch3
```

依次检查差异、暂存源码和报告、创建提交、推送到第三章分支。其他章节使用相应的分支名；需要提交其他实验文件时，明确将它们加入本次提交。

各章节分支独立。切换下一章后，把此前报告一并保留；不要为了同步报告合并整条章节分支，以免混入其他章节的内核代码。

## 评分与同步

push 到 `ch3`、`ch4`、`ch5`、`ch6`、`ch8` 自动触发对应章节评测。官方测试全部通过、实验报告齐全且检查器成功退出，才记该章 100 分。重复通过不会重复加分，已通过章节的成绩会保留。

Actions 先执行 **Test chapter and reports**，通过后执行 **Save progress and upload score**。上传日志出现 `OpenCamp accepted the score (result=1).` 表示 OpenCamp 接口接受了成绩；再到学员成绩页面核对显示。

通过记录保存在学员作业仓库 `gh-pages` 分支的成绩文件中。

模板保留待完成的实验代码，直接运行时出现测试失败属于预期结果。`main`、`ch1`、`ch2`、`ch7` 不计分。

## 常见问题

| 现象 | 检查与处理 |
| --- | --- |
| push 没有权限 | 接受仓库邀请，并使用领取仓库的 GitHub 账号授权 |
| 缺少章节分支 | 联系助教；仓库应包含 `main` 和 `ch1` 至 `ch8` |
| push 后没有评测 | 检查提交分支、Actions 是否启用，以及提交说明是否含 `[skip ci]` |
| 测试没有全部通过 | 查看日志，完成代码和报告后重新 push |
| 上传被跳过 | 联系助教核对分配的账号；只有该学员触发的运行上传成绩 |
| 提示未报名或账号不匹配 | 加入秋冬季训练营，绑定领取仓库使用的 GitHub 账号 |
| 凭证缺失、写入成绩失败或接口拒绝 | 把 Actions 运行链接和错误信息交给助教检查配置 |

运行附件包含 `rcore-grade.log` 和 `rcore-result.json`，保留 30 天。修复账号或权限问题后，可以重跑原工作流；已通过章节的记录保留，重试不会重复加分。也可以在 Actions 选择 **Run workflow** 并选择评分分支；选择 `main` 会跳过评分。

本仓库由 LearningOS 组织维护，保留上游源码历史，按 [GPL-3.0](LICENSE) 许可分发。
