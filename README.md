# rimd [![Build Status](https://travis-ci.org/nicklan/rimd.svg?branch=master)](https://travis-ci.org/nicklan/rimd)

rimd is a set of utilities to deal with midi messages and standard
midi files (SMF).  It handles both standard midi messages and the meta
messages that are found in SMFs.

rimd is fairly low level, and  messages are stored and accessed in
their underlying format (i.e. a vector of `u8`s).  There are some
utility methods for accessing the various pieces of a message, and
for constructing new messages.

For a description of the underlying format of midi messages see [here](http://www.midi.org/techspecs/midimessages.php)
For a description of the underlying format of meta messages see [here](https://web.archive.org/web/20150217154504/http://cs.fit.edu/~ryan/cse4051/projects/midi/midi.html#meta_event)

## Docs

Most public functions have docs in the source.  To build the docs do

    cargo doc

and then point your browser at /path/to/rimd/target/doc/rimd/index.html

## Installation

Use [Cargo](http://doc.crates.io/) and add the following to your Cargo.toml

```
[dependencies.rimd]
git = "https://github.com/RustAudio/rimd.git"
```

## Building

To build simply do

    cargo build

## License

MIT (see LICENSE file)
