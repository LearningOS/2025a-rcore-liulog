## Lab3

1. 进程创建
spawn 用来创建一个子进程，功能上类似 fork&exec，但是 spawn 直接创建子进程，而无需从父进程拷贝，在实现时参考了 new、fork、exec 三个函数的实现，直接生成一个新的进程，它的地址空间直接由 elf_data 进行构造。

2. stride 调度算法
首先需要为每个 TaskControlBlock 添加新的字段，stride、pass、priority，调度的核心是如何获取下一个进程进入执行阶段，os 中通过 fetch() 函数从 ready_queue 中获取适当的进程，利用 min_by_key 找到当前的 stride 最小的 TaskControlBlock，并将其作为下一个执行的进程。