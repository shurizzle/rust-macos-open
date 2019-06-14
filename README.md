# macos-open

A simple way to use /usr/bin/open features in the programmatically way.

This is a wrapper around Core Foundation, Launch Services and File Metadata
frameworks.

Contributions are welcome.

# tl;dr

Just use it like this
```rust
use macos_open::open;

open("http://www.example.com/");
```
