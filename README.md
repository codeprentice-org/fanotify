# fanotify
An idiomatic Rust wrapper for [`fanotify(7)`](https://www.man7.org/linux/man-pages/man7/fanotify.7.html) on Linux.


## Roadmap
- [X] (Khyber) Implement APIs for [`fanotify_init(2)`](https://man7.org/linux/man-pages/man2/fanotify_init.2.html).
- [X] (Khyber) Implement APIs for [`fanotify_mark(2)`](https://www.man7.org/linux/man-pages/man2/fanotify_mark.2.html).
- [ ] (All) Add documentation
- [ ] (All) Review init and mark APIs.
- [ ] (Saquib) Implement `read` API for reading file events.
- [ ] (Rickson) Implement `write` API for writing file permissions.
- [ ] (Asif) Add robust testing.
- [ ] (Asif) Add more complete documentation.
