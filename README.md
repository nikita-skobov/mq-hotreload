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

## Thread local storage

According to the article at the top of this readme, one issue you will run into with live reloading is if your shared object uses thread local storage, you effectively won't be able to re-load the shared object. It will load once just fine, but then refuse to reload the newly built object. in other words, the shared object will stay in memory even after being modified and rebuilt. If you wan't to load the new shared object, you will have to close your program and run it again, which defeats the purpose of hot reloading.

**However, there is a workaround. Keep in mind this is only necessary if your shared object uses thread local storage**.

By builing your binary with the "leaky" flag, we add in code that stubs out the `__cxa_thread_atexit_impl` function, which will prevent your shared object from being cached, but the cost is that it will leak memory. **This only leaks memory on each hot reload, so for some use cases this isn't that bad**. And again, this is only for debugging. A production build won't use this, so it won't affect the final product.

## When do I need to use "leaky"

**Most programs won't need to use the leaky feature**. You will only need to enable it if you:

- your program's shared object uses thread local storage
- you want to be able to hot reload

If both of the above are true, then you will want to use the leaky feature flag as follows:

First, edit your Cargo.toml to add the leaky feature:

```toml
[features]
default = []
isbin = []
prod = []
leaky = []
```

Next, build your host executable again with:

```sh
cargo build --bin whatever --features "isbin leaky"
```

Just like before, you only need to build the host executable once.

## One use case for "leaky"

One use case I needed for this "leaky" feature is to be able to spawn threads in my shared object. Consider the following:

```rs

// ... code ommitted

#[derive(Debug)]
pub struct MyState {
    pub tx: Sender<u32>,
}

impl Drop for MyState {
    fn drop(&mut self) {
        // this is called every time a library is unloaded
        // if you don't provide a drop implementation, the thread will continue
        // to run!
        // this means every time you hotreload, you just add another thread that keeps
        // running.
        let _ = self.tx.send(2);
    }
}

impl Default for MyState {
    fn default() -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            println!("im in a thread");
            let sleep_dur = std::time::Duration::from_millis(500);
            loop {
                if let Ok(message) = rx.try_recv() {
                    println!("BREAKING OUT");
                    break;
                }
                thread::sleep(sleep_dur);
            }
        });
        MyState {
            tx,
        }
    }
}

// ... code ommitted
```

If we want to have a thread spawn when we create a new state, we also need a way to terminate
this thread. Otherwise, every time the library is reloaded, we actually create a new thread, and the old thread keeps running!

We provide now a Drop implementation which handles gracefully closing down the thread before the library is unloaded.

This drop implementation is called by the host executable. **You only need to implement drop if there is something you need to manually close such as threads. Otherwise, You don't need to implement drop.**
