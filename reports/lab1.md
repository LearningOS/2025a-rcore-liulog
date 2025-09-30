## Lab1

> 实现 sys_trace

这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：

- 如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。

- 如果 trace_request 为 1，则 id 应被视作 *mut u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。

- 如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。

- 否则，忽略其他参数，返回值为 -1。

因此这里使用 match 进行匹配，处理不同的 trace_request.

提示中写道：可以扩展 TaskManagerInner 中的结构来维护新的信息。

起初我想是不是把 Syscall_counters 放到 TaskContext 或者 TaskControlBlock, 他们是每个任务（进程）的相关信息（status + context）
想到 trace 系统调用可以追踪其他任务（进程）的信息，因此放到 TaskManagerInner 或许更加合适

除此之外：对于 syscall_counters 采用了 hashmap, 可以不加修改适配更多的系统调用的情况，同时看到 os 中已经实现了 heap，因此可以放心的使用它

这里选择了 hashbrown， A Rust port of Google's SwissTable hash map.

