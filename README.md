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

### Try It Out

There are a few different ways to try out the library! They are all documented in the peer repository that hosts
the source code for a CLI tool that uses this library. You can find it at


[https://github.com/cerfcast/brooks-cli](https://github.com/cerfcast/brooks-cli)


### Goals

We want Brooks to

1. Be a reliable, correct library for parsing, manipulating and interpreting MEL.
2. Demonstrate the utility of MEL and CDNI Processing Stages (by using Brooks in an HTTP server).
3. Be a place to experiment with novel mechanisms for using commodity hardware (e.g., GPUs) for processing MEL.

Of course, those goals are all _very_ aspirational at this point!

### Features

The library has optional features that can be enabled/disabled depending on the environment being used to host the library.

#### Integrations

The library has built-in integrations with several popular web servers, and more are coming soon! If you would like to
let us know which web servers to prioritize integrating, please open an [issue](https://github.com/cerfcast/brooks/issues)!

##### Nginx

Brooks can be built with support for exposing the Processing Stages interpreter to nginx. Select the `nginx`
feature when importing this library to select that feature. When the `nginx` feature is enabled, the library will
configure your environment for building an Nginx module that uses brooks to handle HTTP requests/responses
according to [`HostMetadata`](https://datatracker.ietf.org/doc/html/rfc8006). To do that, the library will

1. clone the Nginx source code into the library's `nginx/nginx` directory.
1. configure the Nginx build system
    - to build the brooks Nginx module, and
    - to install the built `nginx` binary into `nginx/install`.

The brooks Nginx module source code is in `integrations/nginx/module`.

Additional documentation for this feature is coming soon and will be located in the [`./integrations/nginx/module`](./integrations/nginx/module/) directory.

##### Caddy

Brooks can be built with support for exposing the Processing Stages interpreter to Caddy. Select the `caddy`
feature when importing this library to select that feature. When the `caddy` feature is enabled, the library will
configure your environment for building a version of Caddy that contains a module so that it will use brooks to
handle HTTP requests/responses according to [`HostMetadata`](https://datatracker.ietf.org/doc/html/rfc8006).
To do that, the library will

1. build some "shim" shared libraries (to solve the chicken/egg build problem between the Caddy module and the Brooks Library).
1. use go's build system (and package manager) to build a version of Caddy with the module builtin.

The brooks Caddy module source code is in `integrations/caddy/module`.

Additional documentation for this feature is coming soon and will be located in the [`./integrations/caddy/module`](./integrations/caddy/module/) directory.

### Documentation

The core of Brooks is a Rust library for parsing and manipulating expressions of the MEL. The most up-to-date
documentation for the library is available online at

[https://cerfcast.github.io/brooks/brooks_lib/index.html](https://cerfcast.github.io/brooks/brooks_lib/index.html)

### Contributing

We would _love_ your contributions! More information on how to contribute will be coming soon!

In the meantime, if you want to get started helping immediately, please file an
[issue](https://github.com/cerfcast/brooks/issues) or contact [Will Hawkins](mailto:whh8b@obs.cr).

Open Source is all about community -- we can't wait to have you join the effort!!

