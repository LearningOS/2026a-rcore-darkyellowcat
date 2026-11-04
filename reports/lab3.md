# chapter5练习

## 功能总结

迁移上一章的 sys_get_time, sys_mmap 和 sys_munmap 以适应新的进程结构。实现 spawn，
直接创建并加载子进程；实现带优先级的 Stride 调度和
set_priority 系统调用。调度器选择当前 stride 最小的就绪进程，
并按优先级计算步长。

## 问答作业

### stride 算法深入

#### 实际情况是轮到 p1 执行吗？为什么？

不会。因为 250 + 10 = 260 > 255，会发生回绕导致 p2 的 stride 变为 5，p2 的 stride 小于 p1 的 stride， p2 执行。

#### 为什么？尝试简单说明（不要求严格证明）。
```
pass = BigStride / priority <= BigStride / 2
```
增加后，该进程最多比原来的最小值大 BigStride / 2；其他进程的 stride 不变，而新的最小值不会
小于原来的最小值。

#### 补全下列代码中的 partial_cmp 函数

BinaryHeap 是大顶堆，需要进行翻转

```
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    // 先按 u64 回绕再转为 i64，保证回绕后 stride 小的进程优先级高
    let diff = self.0.wrapping_sub(other.0) as i64;

    // 使逻辑 stride 较小的元素获得更高优先级。
    Some(diff.cmp(&0).reverse())
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```
## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   使用 ChatGPT 辅助学习 rCore 第五章的 进程管理。

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   https://learningos.cn/rCore-Tutorial-Guide/index.html

   https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html

   https://lrl52.top/1301/2025s-rcore-ch0-8/

   https://www.bookstack.cn/read/ucore_os_docs/lab5-lab5_3_1_lab_steps.md


3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。