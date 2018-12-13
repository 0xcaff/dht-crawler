# dht-crawler

![CircleCI branch](https://img.shields.io/circleci/project/github/0xcaff/dht-crawler/master.svg)

A tool to collect information about nodes and info-hashes in the DHT. Built
using [tokio].

## Running

To run, install a [rust development environment][dev] and run

    cargo run

to start crawling.

## Tests

There are a number of tests in the project to make sure things work. After
setting up rust, just run:

    cargo test

[dev]: https://doc.rust-lang.org/1.27.2/book/second-edition/ch01-01-installation.html
[tokio]: tokio.rs
