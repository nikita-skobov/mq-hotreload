# MQ-hotreload

This library only exists thanks to the great work in [macroquad](https://github.com/not-fl3/macroquad)

and this article:

https://fasterthanli.me/articles/so-you-want-to-live-reload-rust

## What is it?

A library of functions and macros to easily setup a window/graphics system and with hot reloading. **The contents in the window will update every time you save your code**.

## How to use it?:

To get started you need to make a library crate and two files.

```sh
cargo new --lib something
```

First, edit the Cargo.toml file to add dependencies, and lib/bin info:

```toml
[dependencies]
mq-hotreload = { path = "../mq-hotreload/" }
# this is a fork i made of macroquad that has
# public access to a context object
# that we need for this to work properly
macroquad = { git = "https://github.com/nikita-skobov/macroquad", default-features = false }

[lib]
# this name is important. you need to remember what
# you name your library:
name = "whatever"
crate-type = ["cdylib"]
path = "src/whatever.rs

[[bin]]
name = "main"
path = "src/main.rs"
```

Next, the two files:

```rs
// this will be the host program
// src/main.rs

use mq_hotreload::*;

pub fn main() {
    // libwhatever.so comes from the name lib + whatever + .so
    // whatever is defined in the Cargo.toml, you can name this
    // however you want. if your library name is xyz, then you
    // need this to say "./target/debug/libxyz.so"
    let host = HostOptions::new("./target/debug/libwhatever.so");
    host.run();
}
```

```rs
// this will be the shared object that gets reloaded
// by the host
// src/whatever.rs

use std::any::Any;
use macroquad::color::*;
use macroquad::ctx_helper::ContextHelper;

const BTN_LEFT: macroquad::input::MouseButton = macroquad::input::MouseButton::Left;

mq_hotreload::mqhr_funcs!(MyState);

#[derive(Debug, Default)]
pub struct MyState {
    pub circles: Vec<[f32; 2]>,
}

pub fn update_inner(state: &mut MyState, mut ctxh: ContextHelper) {
    if ctxh.is_mouse_button_released(BTN_LEFT) {
        let (x, y) = ctxh.mouse_position();
        state.circles.push([x, y]);
    }
    ctxh.draw_rectangle(100.0, 100.0, 30.0, 80.0, PURPLE);
    for circle in &state.circles {
        ctxh.draw_circle(circle[0], circle[1], 30.0, GREEN);
    }
}
```

Now build the main executable, and run it.
The main executable will build your shared object for you if it is not already built.

```sh
cargo run --bin main
```

Click around on the screen, and it should draw green circles.

Try editing the `src/whatever.rs` file and change the color of the rectangle, for example:

```rs
ctxh.draw_rectangle(100.0, 100.0, 30.0, 80.0, RED);
```

Then, once you save it, you should see the window update in a few seconds.
You will also notice your circles are gone. This is because the state cannot persist between reloads (as far as I know. If you figure out how to do that without having the state be known to both host and shared, please let me know).
