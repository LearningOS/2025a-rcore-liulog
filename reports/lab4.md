## Lab4

> 文件系统

为了方便在 syscall 处理时进行类型的辨别，是 stdin、stdout 还是普通文件，对 File trait 新增了 as_any 方法，用来区分是否是普通文件

#### linkat 思路：
创建文件的硬链接，硬链接即两个文件指向同一个 inode，从当前目录下（即根目录）先找到 old_name 文件对应的 inode_id，可以通过 find_node_id 这样的方法寻找，创建一个新的 DirEntry，其中名字来自参数中的 new_name，inode 的值来自刚刚查到的 inode_id，并且增加 inode 的引用计数

#### unlinkat 思路；
删除文件的硬链接，需要减少 inode 的引用计数，若引用计数为 0，那么需要删除 inode 对应的所有数据块，然后都要从当前目录中删除 file_name 对应的 DirEntry

#### fstat 思路：
fstat 统计的这些信息对应一个 Inode 也对应一个 DiskInode，所以考虑将引用计数的内容增加到 DiskInode 中，通过接口暴露出来，为了区分 stdin、stdout 和普通文件，这里使用了 file.as_any().downcast_ref::<OSInode>() 来提取 fd_table[fd] 中的 OSInode