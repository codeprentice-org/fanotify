# fanotify
An idiomatic Rust wrapper for [`fanotify(7)`](https://www.man7.org/linux/man-pages/man7/fanotify.7.html) on Linux.


## Roadmap
- [X] (Khyber) Implement APIs for [`fanotify_init(2)`](https://man7.org/linux/man-pages/man2/fanotify_init.2.html).
- [X] (Khyber) Implement APIs for [`fanotify_mark(2)`](https://www.man7.org/linux/man-pages/man2/fanotify_mark.2.html).
- [X] (Khyber) Implement event read API and response write API.
- [X] (Khyber) Implement ergonomic and safe response write API.
- [X] (Khyber) Implement async API.
- [X] (All) Add most documentation.
- [ ] (All) Document flags, masks, etc. (from man pages).
- [ ] (All) Review init API.
- [ ] (All) Review mark API.
- [ ] (All) Review event API.
- [ ] (Khyber) Add more strict runtime pre-testing based on init flags.
- [ ] (Asif) Add robust testing.
- [ ] (All) Release 0.2.0.
