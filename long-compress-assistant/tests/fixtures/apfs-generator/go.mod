module long-decompress-apfs-fixture

go 1.26.4

require github.com/go-filesystems/apfs v0.0.0

require (
	github.com/go-compressions/lzfse v0.3.0 // indirect
	github.com/go-fde/apfs v0.0.0-20260620062418-22bb63627e03 // indirect
	github.com/go-filesystems/interface v0.0.0-20260620062526-43b8c95ba733 // indirect
	github.com/go-volumes/gpt v0.0.0-20260622072431-e1d6ba3b531c // indirect
	github.com/go-volumes/safeio v0.0.0-20260622072324-7f8eb19f6f8c // indirect
	golang.org/x/crypto v0.50.0 // indirect
	golang.org/x/sys v0.43.0 // indirect
)

replace github.com/go-filesystems/apfs => ../../../test-results/apfs-tool/source
