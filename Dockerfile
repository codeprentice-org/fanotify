FROM rust

WORKDIR /usr/src/fanotify

RUN cargo install just