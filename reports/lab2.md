## Lab2

引入地址空间后，os 目录下的源码明显增加，需要花一些功夫，来梳理 rcore 对于地址空间做的抽象等

1. 重写 sys_get_time、sys_trace

这两个系统调用不涉及修改页表，仅仅是借助页表进行查找，从 VA 到 PA。
实现时参考了 translated_byte_buffer 的实现，借助 PageTable::from_token 得到当前应用程序的页表，随后通过 translate 进行翻译，但是注意 translate 并不是将 VA 翻译得到 PA，而是返回一个 Option<PageTableEntry>, 因此，需要手动提取 PPN、Flags，进行拼接等，得到最终的 PA。
同时，还要注意 PTE_U 的检查，对于用户态的遍历需要额外增加该检查。

2. mmap 实现

在映射前，需要先检查是否已经存在映射了，若没有映射再继续执行。PageTable 可以用来获取 PTE，但是无法建立新的映射，因此需要从 MemorySet 入手，需要一个办法修改当前进程的 MemorySet，因此在 task 模块中增加了 current_user_mmap_one_page 函数接口，一路调用到 MemorySet 中。
判断新添加的页的 VPN 和当前已有页面的关系，进行一定的合并。
注意不能直接调用 MapArea.map_one，这个函数不会进行 vpn_range 的维护，要么添加新的 MapArea，要么通过 append_to 函数以及新增加了 append_to_start 函数。
注意：translate 得到的是 PTE，还要手动判断其是否 Valid 等。

3. munmap 实现

在取消映射前，先检查是否存在映射，若检查到存在没有映射的区域则直接返回。
对于 munmap 来说，其和 mmap 的实现类似，从 MemorySet 中找到包含 VA 的 MemoryArea，分析其在 Area 中的位置，如果位于开头或结尾可以直接通过 shrink_to 或新增的 shrink_to_start 进行 unmap，若在中间则要进行将该 Area 拆分成两部分。注意不要直接使用 MapArea.unmap_one，它不会维护 vpn_range。

> mmap 和 munmap 没有进行进一步检查，所以可能会出现没有任何内容的 MapArea，或者存在一些 MapArea 可以合并但是没有合并。



> MemorySet 中提供了 append_to, shrink_to 两个方法，可以用来更细致的管理 MapArea，进行 MapArea 的拆分合并等，这里没有进行相关调用，实现时可以借助这两个函数实现更细致的 mmap、munmap。
>
> 没有提供 mmap、munmap 失败的特殊处理。
