# Usage

```sh
cargo build --release

./target/release/pageturtle_cli init

# Start the development server
./target/release/pageturtle_cli dev

# Build the blog
./target/release/pageturtle_cli build

# For more information, see the help
./target/release/pageturtle_cli help
```

# TODO
- [ ] [Full-text search](https://lunrjs.com/)
- [X] [Generate RSS feeds](https://validator.w3.org/feed/check.cgi)
- [X] Great CLI interface
- [ ] Nice error messages
- [ ] Automatically build table of contents
- [X] Development server and live reload
- [ ] Incremental compilation on development server
