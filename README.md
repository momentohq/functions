# Momento Functions

Momento Functions are how you can extend Momento.

A work in progress, you can learn more about Functions by reaching out to support@momentohq.com

Functions run on Momento's service hosts, and offer a powerful scripting capability.

* Use the Momento host interfaces to interact with Momento features within your cache
* Use the AWS host interfaces to use a managed, hot channel to talk to AWS resources
* Use the HTTP host interfaces to reach out to anything you want

This repository holds crates for Momento Functions guest code.

To see some of what you can do with Functions, you can look at [the examples](./momento-functions/examples/).

## Getting started

### One-time setup

* Install Rust: https://rustup.rs
* Add the Momento Functions compile target: `rustup target add wasm32-wasip2`

### Make a project

`cargo init --lib hello`

### Set up build configuration

Add a file `.cargo/config.toml` that sets the build target, for convenience.

```toml
[build]
target = "wasm32-wasip2"
```

### Set up `Cargo.toml`

Add this to build the right kind of artifact:

```toml
[lib]
crate-type = ["cdylib"]
```

Import the Functions support library and WIT library.

```toml
[dependencies]
momento-functions = { version = "0" }
momento-functions-wit = { version = "0" }
```

### Write a Function

The simplest function is a pong response web function. You can put this in `lib.rs`.

```rust
momento_functions::post!(ping);
fn ping(_payload: Vec<u8>) -> &'static str {
    "pong"
}
```

### Build and deploy

**Build**: `cargo build --release`

**Deploy**

First, base64 encode the function, then upload. Note that the path here includes "manage". The output from
using `curl -v` should include an HTTP status code of 204.

```bash
MOMENTO_CACHE_NAME=your_cache

base64_data=$(cat target/wasm32-wasip2/release/hello.wasm | base64)

curl -v \
  https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/manage/$MOMENTO_CACHE_NAME/ping \
  -XPUT \
  -H "authorization: $MOMENTO_API_KEY" \
  -H "Content-Type: application/json" \
  --data "{\"inline_wasm\":\"$base64_data\"}"
```

Alternatively, you can use the [Momento CLI](https://github.com/momentohq/momento-cli),
which will handle the encoding for you:

```bash
momento preview function put-function \
   --cache-name "$MOMENTO_CACHE_NAME" \
   --name ping \
   --wasm-file target/wasm32-wasip2/release/hello.wasm
```

**Invoke**

Invoke the function by sending a request directly to the function name.

```bash
MOMENTO_CACHE_NAME=your_cache

curl \
  https://api.cache.$MOMENTO_CELL_HOSTNAME/functions/$MOMENTO_CACHE_NAME/ping \
  -H "authorization: $MOMENTO_API_KEY" \
  -d 'ping'
```

### Going further

From here, you should look at [the examples](./momento-functions/examples/). Momento Functions are a limited
environment, but the supported feature set is growing.

## Developing a Function

### Wasi support and standard library

Using `wasm32-wasip2`, you have access to `std::time`. Most other `std` wasip2 interfaces will panic at runtime.

| `std` wasi interface | status                                                                     |
|----------------------|----------------------------------------------------------------------------|
| time                 | `SystemTime` and `Instant` supported                                       |
| environment          | supported, populated from function configuration                           |
| error                | supported, but empty; also unavailable due to lack of io interface support |
| exit                 | unsupported - it does panic though, which may work well enough for you     |
| filesystem_preopens  | unsupported                                                                |
| filesystem_types     | unsupported                                                                |
| stderr               | unsupported                                                                |
| stdin                | unsupported                                                                |
| stdout               | unsupported                                                                |
| streams              | unsupported                                                                |

Other wasi interfaces are not defined and will result in a linking error when you upload your Function.

### Environment details

You are running under a `wasmtime` host. Unless otherwise specified, the host you're running on is undefined.
You are effectively running as a stateless web server.

As the ecosystem matures, new limits may be created and execution location semantics may change.

If you hit an error you don't think you should - e.g., you updated Rust locally and now your Functions don't
link - please reach out to support@momentohq.com
