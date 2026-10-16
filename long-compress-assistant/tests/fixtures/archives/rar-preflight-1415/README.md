# RAR 预检查固定样本

`stored.rar` 是 libarchive 的原始 RAR5 stored 测试包，没有自行重写格式。

- 上游提交：`19ff56da4f4790064346579a1a7f18a0230b0ac6`
- 来源：https://raw.githubusercontent.com/libarchive/libarchive/19ff56da4f4790064346579a1a7f18a0230b0ac6/libarchive/test/test_read_format_rar5_stored.rar.uu
- UU 源 SHA256：`ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8`
- 解码后的包：109 字节，SHA256 `35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7`
- 一个条目 `helloworld.txt`，未压缩 29 字节，无密码。

沿用项目原有 `scripts/fetch-archive-test-fixtures.mjs` 的固定来源和校验，仅将该小样本随仓库保存，避免本机缓存成为用例前置条件。上游授权见该提交的 libarchive `COPYING`。这证明真实 RAR 元数据与控制检查，不代表海量条目性能或单次 SDK 调用可被立即打断。
