# Lab 3：进程管理与 stride 调度

## 实验环境

- 架构与模拟器：RISC-V 64 / QEMU `virt`
- SBI：RustSBI Prototyper 0.4.1（jump 模式）
- Rust 工具链：nightly-2024-05-02

## 功能实现总结

本实验迁移了跨页安全的 `get_time`、`mmap` 和 `munmap`，并实现 `spawn`，直接从目标 ELF 创建具有独立地址空间的子进程，同时建立父子关系并加入就绪队列。调度器改为 stride 算法，默认优先级为 16，拒绝小于 2 的优先级；每次选取环形意义下 stride 最小的进程，并按 `BIG_STRIDE / priority` 累加，支持整数溢出后的正确比较。

## 问答题：stride 算法深入

### 1. 8 位 stride 直接比较时会轮到 p1 吗？

不会。初始时 `p1.stride = 255`、`p2.stride = 250`，因此先选择 p2。p2 执行后按 8 位无符号整数计算得到 `250 + 10 = 4`。若下一次仍使用普通无符号大小比较，会认为 `4 < 255`，错误地再次选择 p2；而在没有溢出的数学整数中，p2 的逻辑 stride 应为 260，所以下一次应选择 p1。问题的根源是普通比较没有考虑计数值的环形表示。

### 2. 为什么 `STRIDE_MAX - STRIDE_MIN <= BIG_STRIDE / 2`？

每次调度前选择当前 stride 最小的进程。该进程执行后只增加一个 pass，而 `priority >= 2` 保证每个进程的 `pass = BIG_STRIDE / priority <= BIG_STRIDE / 2`。增加前它不大于其他任何进程；增加后即使越过当前最大值，也最多比原最小值前进半个 `BIG_STRIDE`。其他进程的 stride 不变。重复这一过程，活动进程之间的逻辑跨度不会超过半个环形整数空间。这一界限使溢出前后的两个值仍可唯一判断先后。

### 3. 溢出安全的 `partial_cmp`

`BinaryHeap` 默认弹出“最大”元素，因此要把逻辑上较小的 stride 判为更大。同时利用任意两个有效 stride 的距离不超过半个整数范围这一条件，对包装减法的最高位进行判断：

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let difference = self.0.wrapping_sub(other.0) as i64;
        if difference < 0 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
```

例如 8 位环形计数中，`125` 与 `255` 的包装差为 `126`，说明 125 在逻辑上更靠后；`129 - 255` 的包装结果为 `130`，按有符号 8 位解释为负数，说明 129 在逻辑上更靠前。实际内核未使用堆，而是用相同的包装差原则线性扫描就绪队列。

## 测试记录

已完成：

```sh
cargo check --offline --target riscv64gc-unknown-none-elf
cargo fmt --check
git diff --check
```

课程完整评测需在测试仓库就绪后执行：

```sh
make -C ci-user test CHAPTER=5
```

## 荣誉准则与协作披露

1. 在完成本次实验的过程中，我使用了 OpenAI Codex 协助分析实验要求、实现代码并起草本报告；提交前我会逐项检查、理解并对最终内容负责。
2. 参考资料包括课程仓库源码、rCore chapter5 实验指导、RISC-V 虚拟内存机制，以及 stride scheduling 的环形计数比较方法。
3. 本报告不代替提交者本人的学习与确认；提交者应在理解实现后，按课程荣誉准则如实补充自己的交流对象、参考资料和完成情况。
4. 未使用破坏或妨碍评测系统的硬编码，未修改测试程序或评分脚本。
