## Brooks: An Implementation of the Metadata Expression Language

The [Metadata Expression Language (MEL)](https://datatracker.ietf.org/doc/draft-ietf-cdni-metadata-expression-language/)
is one piece of the wider puzzle being put together by the IETF's CDNI working group.
The CDNI working group's goal is to

> The goal of the CDNI Working Group is to allow the interconnection of separately administered
CDNs in support of the end-to-end delivery of content from CSPs through multiple CDNs and
ultimately to end users (via their respective User Agents). (From the [IETF](https://datatracker.ietf.org/group/cdni/about/))

The MEL is the language by which CDN operators can specify whether, when and what metadata
to apply to requests that they serve. It is used, especially, in the specification of
the [CDNI Processing Stages Metadata](https://datatracker.ietf.org/doc/draft-ietf-cdni-processing-stages-metadata/).

### Goals

We want Brooks to

1. Be a reliable, correct library for parsing, manipulating and interpreting MEL.
2. Demonstrate the utility of MEL and CDNI Processing Stages (by using Brooks in an HTTP server).
3. Be a place to experiment with novel mechanisms for using commodity hardware (e.g., GPUs) for processing MEL.

Of course, those goals are all _very_ aspirational at this point!

### Documentation

The core of Brooks is a Rust library for parsing and manipulating expressions of the MEL. The most up-to-date
documentation for the library is available online at

[https://cerfcast.github.io/brooks/brooks_lib/index.html](https://cerfcast.github.io/brooks/brooks_lib/index.html)

### Contributing

We would _love_ your contributions! More information on how to contribute will be coming soon!

In the meantime, if you want to get started helping immediately, please file an
[issue](https://github.com/cerfcast/brooks/issues) or contact [Will Hawkins](mailto:whh8b@obs.cr).

Open Source is all about community -- we can't wait to have you join the effort!

