# 独立 WIM 原件

由未改动的 wimlib-imagex 1.14.5 x64 捕获自行生成的小数据，不含 Windows 系统镜像。覆盖 Copy、XPRESS、LZX 双镜像和 LZMS 固实；有效压缩载荷不是全部回退 Copy 的随机数据。每个原件在生成时由 wimlib verify/apply 完整回读、核对文件 SHA256，空目录/空文件保持。

`manifest.json` 固定原件摘要、工具/官方包来源、图像数量、每个逻辑路径的独立摘要和目录。多镜像以 1/2 目录表示，7-Zip 暴露的 `[1].xml` 是原 XML 资源，其摘要由 wimlib `info --extract-xml` 独立取得。单镜像不附带此虚拟文件。旁边 XML 保留独立抽取观察，不是归档输出。

维护者重建脚本为 `scripts/generate-wim-1527.py --tool <wimlib-imagex.exe> --output <新目录>`，拒绝覆盖已有固定原件。时间/GUID 可变化，新字节必须重新记录；wimlib 仅用于独立测试，不随产品分发，不新增运行时依赖。本版只处理普通 WIM 数字切片，不据此宣称支持原生 SWM 或系统部署。
