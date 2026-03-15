# Gungnir - 基于STM32的嵌入式操作系统

Gungnir是一个基于STM32H7系列微控制器和Rust语言实现的嵌入式操作系统。它采用异步架构设计，支持多任务处理、内存管理、文件系统和交互式Shell，旨在为嵌入式系统提供现代、安全的操作系统环境。

## ✨ 主要特性

- **异步任务执行器**: 基于Waker机制的自定义异步执行器，支持高效的多任务调度
- **内存管理**: 堆分配器（LockedHeap）支持动态内存分配，提供内存使用情况查询
- **进程间通信（IPC）**:
  - 异步Channel：支持任务间的消息传递
  - 异步Mutex：提供线程安全的共享资源访问
  - 异步Signal：任务同步和事件通知机制
- **文件系统**: 完整的FAT32文件系统实现，支持SD卡读写操作
- **设备驱动**:
  - 异步UART驱动：支持串口通信
  - 异步SDMMC驱动：支持SD卡访问
  - 块设备驱动抽象层
- **交互式Shell**: 支持命令参数传递，内置多种实用命令
- **时间管理**: Duration、Instant、Timer等时间相关抽象

## 🎯 硬件平台

- **微控制器**: STM32H743IIT6 (Cortex-M7, 480MHz)
- **存储器**: 2MB Flash, 1MB RAM
- **外设支持**: USART, SDMMC, GPIO, DMA, SDRAM等
- **开发板**: 基于STM32H7xx系列的开发板

## 🚀 快速开始

### 环境要求

- Rust工具链 (nightly版本)
  ```bash
  rustup toolchain install nightly
  rustup default nightly
  rustup target add thumbv7em-none-eabihf
  ```
- ARM GCC工具链
- st-flash (ST-Link编程工具)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install stlink-tools

  # macOS (Homebrew)
  brew install stlink

  # 从源码编译
  git clone https://github.com/stlink-org/stlink
  cd stlink
  make release
  sudo make install
  ```
- make

### 构建项目

```bash
# 克隆仓库
git clone https://github.com/coliar/Gungnir.git
cd Gungnir

# 构建项目
make
```

构建完成后，生成的二进制文件位于 `build/Gungnir.bin`。

### 烧录到开发板

```bash
# 使用st-flash烧录
st-flash write build/Gungnir.bin 0x08000000
```

**其他常用命令：**
- `st-flash erase` - 擦除整个Flash
- `st-flash read dump.bin 0x08000000 0x10000` - 读取Flash内容
- `st-flash --reset` - 烧录后复位芯片
- `st-flash --help` - 查看所有选项

### 串口连接

默认串口配置：
- 波特率: 115200
- 数据位: 8
- 停止位: 1
- 无校验

## 📁 项目结构

```
Gungnir/
├── kernel/                    # 内核源代码（Rust）
│   ├── src/
│   │   ├── allocator.rs      # 内存分配器实现
│   │   ├── task/             # 任务管理
│   │   │   ├── executor.rs   # 异步任务执行器
│   │   │   └── yield_now.rs  # 任务调度
│   │   ├── ipc/              # 进程间通信
│   │   │   ├── channel.rs    # 异步通道
│   │   │   ├── async_mutex.rs # 异步互斥锁
│   │   │   └── async_signal.rs # 异步信号
│   │   ├── gsh/              # 交互式Shell
│   │   │   ├── gshell.rs     # Shell核心
│   │   │   └── cmds/         # Shell命令
│   │   │       ├── poem.rs   # 诗歌显示命令
│   │   │       ├── uname.rs  # 系统信息命令
│   │   │       └── meminfo.rs # 内存信息命令
│   │   ├── fatfs/            # FAT32文件系统
│   │   ├── driver/           # 设备驱动
│   │   │   ├── usart.rs      # UART驱动
│   │   │   ├── sdmmc.rs      # SD卡驱动
│   │   │   └── block_device_driver.rs # 块设备抽象
│   │   ├── time/             # 时间管理
│   │   └── lib.rs            # 内核主入口
├── board/                    # 板级支持包（C语言）
│   ├── src/                 # 硬件初始化代码
│   └── inc/                 # 头文件
├── hal/                     # STM32 HAL库
├── clib/                    # C标准库函数实现
├── rustlib/                 # Rust库
├── Makefile                 # 构建配置
├── startup_stm32h743xx.s    # 启动汇编代码
└── STM32H743IITx_FLASH.ld   # 链接器脚本
```

## 💻 Shell命令

Gungnir提供了一个交互式Shell，支持以下命令：

| 命令 | 描述 | 示例 |
|------|------|------|
| `help` | 显示所有可用命令 | `help` |
| `poem` | 显示一首古诗 | `poem` |
| `uname` | 显示系统名称和版本 | `uname` |
| `meminfo` | 显示堆内存使用情况（KB） | `meminfo` |

所有命令都支持异步执行和参数传递机制。

## 🔧 核心技术

### 异步架构
- 基于Rust的async/await语法
- 自定义Waker实现的任务唤醒机制
- 无锁任务队列管理
- 支持任务挂起和恢复

### 内存管理
- 链表式堆分配器
- 线程安全的内存分配/释放
- 内存碎片管理
- 运行时内存使用统计

### 文件系统
- 完整的FAT32实现
- 异步文件操作API
- 目录遍历和文件管理
- SD卡块设备支持

### 驱动模型
- 统一的异步驱动接口
- DMA支持的数据传输
- 中断驱动的外设管理
- 硬件抽象层

## 🧪 示例代码

### 创建异步任务
```rust
async fn example_task() {
    println!("Hello from async task!");
}

// 在executor中运行
executor.spawn(example_task());
```

### 使用异步Channel
```rust
let (tx, rx) = channel::bounded(10);
executor.spawn(async move {
    tx.send("Hello").await.unwrap();
});

executor.spawn(async move {
    let msg = rx.recv().await.unwrap();
    println!("Received: {}", msg);
});
```

### 文件操作
```rust
let fs = FileSystem::new(buf_stream, FsOptions::new()).await?;
let root = fs.root_dir();
let mut file = root.create_file("test.txt").await?;
file.write(b"Hello, World!").await?;
file.flush().await?;
```

## 📈 项目状态

| 模块 | 状态 | 说明 |
|------|------|------|
| 任务调度 | ✅ 稳定 | 支持多任务并发执行 |
| 内存管理 | ✅ 稳定 | 支持动态内存分配 |
| 文件系统 | ✅ 稳定 | FAT32读写支持 |
| Shell | ✅ 稳定 | 交互式命令行界面 |
| IPC机制 | ✅ 稳定 | Channel/Mutex/Signal |
| 驱动框架 | ✅ 稳定 | UART/SDMMC驱动 |
| 网络协议 | 🔄 计划中 | 未来扩展 |

## 🤝 贡献指南

欢迎贡献代码！请遵循以下步骤：

1. Fork本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启Pull Request

### 开发规范
- 使用Rust fmt进行代码格式化
- 编写清晰的文档注释
- 为新功能添加测试用例
- 遵循现有的代码风格

## 📄 许可证

本项目基于开源许可证发布。第三方库的许可证信息请查看：
- HAL库：`hal/STM32H7xx_HAL_Driver/LICENSE.txt`
- CMSIS：`hal/CMSIS/LICENSE.txt`

## 📞 联系方式

如有问题或建议，请通过以下方式联系：
- GitHub Issues: [项目Issues页面](https://github.com/coliar/Gungnir/issues)
- 项目维护者: [coliar](https://github.com/coliar)

## 🙏 致谢

- [Rust Embedded WG](https://github.com/rust-embedded) - Rust嵌入式工作组
- [embedded-fatfs](https://github.com/MabezDev/embedded-fatfs) - 嵌入式FAT文件系统库
- [linked-list-allocator](https://github.com/rust-osdev/linked-list-allocator) - 链表内存分配器
- STMicroelectronics - STM32 HAL库

---

*Gungnir - 为嵌入式世界带来现代操作系统的力量*