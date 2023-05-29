# pageturtle

Pageturtle is a modern and easy to use static site generator, you bring the
markdown and it builds you a nice and clean website, simple as that.

You can [check my personal blog](https://viniciusmuller.github.io/blog/) to see
how it currently looks like.

# Features
- Simple to use: you write markdown, pageturtle takes care of the rest
- Fast: building a blog with dozens of posts takes ~5 milliseconds
- Development server with live reload
- Automatically generates table of contents
- Automatically generates RSS feeds

# Planned Features
- [ ] [Full-text search](https://lunrjs.com/)
- [ ] Friendly error messages
- [ ] Syntax highlighting
- [ ] Incremental compilation
    - [ ] `pageturtle dev` subcommand
    - [ ] `pageturtle build` subcommand
- [ ] Support custom CSS themes
- [ ] Automatically optimize images for the web

# Non-goals
- Support custom layouts

# Usage

```sh
# Start a new blog
pageturtle init -d my-blog

cd my-blog

# Now you can access the development server at localhost:7000
pageturtle dev

# In order to check other commands, see
pageturtle help
```

## Running with Nix

```
nix run github:viniciusmuller/pageturtle help
```
