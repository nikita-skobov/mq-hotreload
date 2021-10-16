# MQ-hotreload

This library only exists thanks to the great work in [macroquad](https://github.com/not-fl3/macroquad)

and this article:

https://fasterthanli.me/articles/so-you-want-to-live-reload-rust

## What is it?

A library of functions and macros to easily setup a window/graphics system and with hot reloading. **The contents in the window will update every time you save your code**.

![hotreload2](https://user-images.githubusercontent.com/39128800/137549041-0b3508ef-d07c-4bd6-a585-327d241173c9.gif)



## How to use it?:

To get started you need to make a library crate and edit the Cargo.toml.

```sh
cargo new --lib whatever
```

we add dependencies, and lib/bin info:

```toml
[package]
name = "whatever"
version = "0.1.0"
edition = "2018"

[dependencies]
mq-hotreload = { git = "https://github.com/nikita-skobov/mq-hotreload" }
# this is a fork i made of macroquad that has
# public access to a context object
# that we need for this to work properly
macroquad = { git = "https://github.com/nikita-skobov/macroquad", default-features = false }

[lib]
# important: dont change the library's name
# it should be the same as the package
crate-type = ["cdylib"]
path = "src/main.rs"

[[bin]]
name = "whatever"
path = "src/main.rs"
required-features = ["isbin"]

[features]
default = []
isbin = []
prod = []

# The isbin feature is important that it is present in the bin, but NOT in the lib.
# This feature is used for conditional compilation to ensure that only the executable gets a main function,
# and that only the library gets the extern functions.
```

Next, we create the `src/main.rs` file:

```rs
use mq_hotreload::*;
use std::any::Any;
use macroquad::color::*;
use macroquad::ctx_helper::ContextHelper;

const BTN_LEFT: macroquad::input::MouseButton = macroquad::input::MouseButton::Left;

mq_hotreload::mqhr_funcs!(MyState);

// the above macro adds some necessary functions.
// if this is being built as a binary, it adds a main function that contains the following:
// ```
// let host = host_options!();
// host.run();
// ```
// you can provide your own host options by doing the following:
// mq_hotreload::mqhr_funcs!(MyState, {
//    let mut host = host_options!();
//    // edit the host object
//    // the expression HAS to end with the host object:
//    host
// });

#[derive(Debug, Default)]
pub struct MyState {
    pub circles: Vec<[f32; 2]>,
}

pub fn update_inner(state: &mut MyState, mut ctxh: ContextHelper) {
    if ctxh.is_mouse_button_released(BTN_LEFT) {
        let (x, y) = ctxh.mouse_position();
        state.circles.push([x, y]);
    }
    ctxh.draw_rectangle(10.0, 100.0, 130.0, 180.0, RED);
    ctxh.draw_rectangle(0.0, 350.0, 30.0, 80.0, PURPLE);
    ctxh.draw_rectangle(100.0, 210.0, 130.0, 90.0, GREEN);
    for circle in &state.circles {
        ctxh.draw_circle(circle[0], circle[1], 30.0, BLUE);
    }
}

```

Now build the main executable, and run it.
The main executable will build your shared object for you if it is not already built.

```sh
# it will not build without the isbin feature:
cargo build --bin whatever --features isbin
./target/debug/whatever
```

Adding this feature flag is annoying, but because we have hot reloading
enabled, we technically only need to build the host
program once, and afterwards we just run the `./target/debug/whatever` program
and edit our main file and have it rebuild each time.

Alternatively, you can build the library manually with:

```sh
cargo build --lib
```

Click around on the screen, and it should draw green circles.

Try editing the `src/whatever.rs` file and change the color of the rectangle, for example:

```rs
ctxh.draw_rectangle(100.0, 100.0, 30.0, 80.0, RED);
```

Then, once you save it, you should see the window update in a few seconds.
You will also notice your circles are gone. This is because the state cannot persist between reloads (as far as I know. If you figure out how to do that without having the state be known to both host and shared, please let me know).

Finally, we included a `prod` feature to allow us to build for "production" which just means we build a single executable instead of using hot reloading. This is when you are done developing, and want to output a finished project without any dynamic loading/file watching. You do this as follows:

```sh
cargo build --bin whatever --features "isbin prod"
```
