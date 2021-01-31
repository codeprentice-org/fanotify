# fanotify

![License: MIT](https://img.shields.io/github/license/codeprentice-org/fanotify.svg)
![active development](https://img.shields.io/badge/active%20dev-yes-brightgreen.svg)
![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/codeprentice-org/fanotify.svg)
![Build test](https://img.shields.io/github/workflow/status/codeprentice-org/fanotify/Rust)

An idiomatic Rust wrapper for [`fanotify(7)`](https://www.man7.org/linux/man-pages/man7/fanotify.7.html) on Linux.


## Development
There is a `justfile` (using [`just`](https://github.com/casey/just)) 
containing some common commands for easier development.

Run `cargo install just` to install `just`, 
and then just run `just` to see the available commands.

## Roadmap
- [X] (Khyber) Implement APIs for [`fanotify_init(2)`](https://man7.org/linux/man-pages/man2/fanotify_init.2.html).
- [X] (Khyber) Implement APIs for [`fanotify_mark(2)`](https://www.man7.org/linux/man-pages/man2/fanotify_mark.2.html).
- [X] (Khyber) Implement event read API and response write API.
- [X] (Khyber) Implement ergonomic and safe response write API.
- [X] (Khyber) Implement async API.
- [X] (All) Add most documentation.
- [ ] (Rickson) Document flags, masks, etc. (from man pages).
- [ ] (All) Review init API.
- [ ] (All) Review mark API.
- [ ] (All) Review event API.
- [ ] (Khyber) Add more strict runtime pre-testing based on init flags.
- [ ] (Rickson) Add robust testing.
- [X] (Rickson) Setup CI for `clippy` and testing.
- [ ] (All) Release 0.2.0.
