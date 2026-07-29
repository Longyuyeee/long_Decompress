package main

import (
	"fmt"
	"os"
	"path/filepath"

	apfs "github.com/go-filesystems/apfs"
)

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: apfs-generator <output.apfs> <payload>")
		os.Exit(2)
	}

	output := os.Args[1]
	payloadPath := os.Args[2]
	payload, err := os.ReadFile(payloadPath)
	if err != nil {
		panic(err)
	}
	if err := os.WriteFile(output, nil, 0o600); err != nil {
		panic(err)
	}
	if err := apfs.FormatContainer(output, 8<<20, "LongTest"); err != nil {
		panic(err)
	}

	container, err := apfs.OpenContainerRW(output)
	if err != nil {
		panic(err)
	}
	defer container.Close()

	volume, err := container.OpenVolume(0)
	if err != nil {
		panic(err)
	}
	if _, err := volume.CreateFile(2, filepath.Base(payloadPath), payload); err != nil {
		panic(err)
	}
	if err := container.Commit(); err != nil {
		panic(err)
	}
}
