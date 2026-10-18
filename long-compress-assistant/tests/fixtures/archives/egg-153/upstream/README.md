# Test data

Sample EGG archives used by the integration tests. Encrypted archives use the
password `test1234`.

All test data in this directory is dedicated to the public domain under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).

More encrypted samples for manual testing (password `jtr`):
[openwall/john-samples](https://github.com/openwall/john-samples/tree/main/ALZip).

`posix.egg` carries the Unix POSIX file-info header (mode/uid/gid/mtime); it was
produced by the Android archiver app, the only build that writes EGG on a
Unix-like system.

`solid.egg` is the only sample here in the solid format (one shared compressed
stream).
