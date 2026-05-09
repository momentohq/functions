# Momento Functions

Momento Functions are how you can extend Momento.

A work in progress, you can learn more about Functions by reaching out to support@momentohq.com

Functions run on Momento's service hosts, and offer a powerful scripting capability.

* Use the Momento host interfaces to interact with Momento features within your cache
* Use the AWS host interfaces to use a managed, hot channel to talk to AWS resources
* Use the HTTP host interfaces to reach out to anything you want

This repository holds crates for Momento Functions guest code.

To see some of what you can do with Functions, you can look at [the examples](./examples/).

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

Pull in only the crates you actually need. For a basic web function, the
guest macro and the off-guest buffer crate are enough:

```toml
[dependencies]
momento-functions-bytes     = { version = "0" }
momento-functions-guest-web = { version = "0" }
```

For a Spawn function, swap `guest-web` for `guest-spawn`. Other capabilities
live in their own focused crates — add them as you go: `momento-functions-cache`,
`momento-functions-http`, `momento-functions-token`, `momento-functions-topic`,
`momento-functions-valkey`, `momento-functions-aws-s3`,
`momento-functions-aws-secrets-manager`, `momento-functions-aws-auth`,
`momento-functions-host-log`.

### Write a Function

The simplest function is a pong response web function. You can put this in `lib.rs`.

```rust
use momento_functions_bytes::Data;
use momento_functions_guest_web::invoke;

invoke!(ping);
fn ping(_payload: Data) -> &'static str {
    "pong"
}
```

`Data` is a buffer that can stay on the host instead of being copied into your
function's memory — useful when you're just passing bodies through. Call
`Data::into_bytes()` when you actually need the bytes.

For typed JSON in/out, swap the payload type for `momento_functions_bytes::encoding::Json<T>`:

```rust
use momento_functions_bytes::encoding::Json;
use momento_functions_guest_web::invoke;

#[derive(serde::Deserialize)]
struct Request { name: String }

#[derive(serde::Serialize)]
struct Response { message: String }

invoke!(greet);
fn greet(Json(request): Json<Request>) -> Json<Response> {
    Json(Response { message: format!("Hello, {}!", request.name) })
}
```

For a portfolio of more substantial v2 Functions — cache, Valkey, HTTP
integrations (Turbopuffer, OpenAI), AWS S3 / Secrets Manager, disposable
token vending, structured logging, Spawn — see [`examples/`](./examples/).
Each is its own minimal crate so you can copy a `Cargo.toml` as a starter.

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

Alternatively, you can use the CLI:

```bash
momento preview function invoke-function \
  --cache-name "$MOMENTO_CACHE_NAME" \
  --name ping
  --data 'ping'
```

### Going further

From here, you should look at [the examples](./examples/). Momento Functions are a limited
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
