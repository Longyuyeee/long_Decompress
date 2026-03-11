fn main() {
    sevenz_rust::decompress_file_with_password("test.7z", "out", "password".as_bytes().into());
}
