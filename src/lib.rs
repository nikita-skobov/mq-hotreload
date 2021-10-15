use macroquad::{ctx_helper::ContextHelper, window::Conf};
use std::{any::Any, process::Command, path::PathBuf, fs, io};
use dumbfilewatch::{DFOwner, DFWatcher};
use libloading::{Symbol, Library};

mod dumbfilewatch;


/// provides the external functions that will be called by the host.
/// you can provide these functions yourself, but the macro just makes it easier because
/// the downcast mut is a bit messy. Also, if the downcast mut fails, we panic.
/// if you want to provide your own implementation, you can change how that should
/// be handled.
#[macro_export]
macro_rules! mqhr_funcs {
    ($stateobj:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn mqhr_update(ctxh: ::macroquad::ctx_helper::ContextHelper, mut data: Box<dyn Any>) -> Box<dyn Any> {
            let ctxh = ctxh;
            let boxdata = &mut *data;
            match boxdata.downcast_mut::<$stateobj>() {
                Some(s) => {
                    update_inner(s, ctxh);
                }
                None => {
                    panic!("FAILED DOWNCAST");
                }
            }
        
            data
        }

        #[no_mangle]
        pub unsafe extern "C" fn mqhr_init() -> Box<dyn Any> {
            let mynum = $stateobj::default();
            let mybox: Box<dyn Any> = Box::new(mynum);
            mybox
        }        
    };
}

#[derive(Debug)]
pub struct HostOptions {
    /// name/path to the shared object that will be reloaded
    pub shared_object: PathBuf,
    /// files/folders to watch that should trigger a reload,
    /// if empty, we default to "./src/" and therefore will reload
    /// anytime we detect something in the "./src/" folder has changed.
    /// KEEP IN MIND that this is relative to where the executable is ran from!
    /// "./src/" is a reasonable default if and only if you are running this
    /// from the root of your project. If you are running from a different directory,
    /// it would be better to provide absolute paths to the files you want to watch.
    pub watch_files: Vec<String>,
    /// macroquad config. if None, we will use macroquad's default config
    pub macroquad_conf: Option<Conf>,
    /// duration to poll the watch files (in milliseconds)
    /// default is 200ms
    pub shared_poll_time: u64,
    /// directory where to re-run the 'cargo build --lib' command from.
    /// default is '.' because it is assumed you will be running this from
    /// the directory where your cargo project is. But this option can be changed
    /// to an absolute path if it is located somewhere else.
    pub cargo_project_path: String,
}

impl Default for HostOptions {
    fn default() -> Self {
        HostOptions {
            shared_object: "".into(),
            cargo_project_path: ".".into(),
            watch_files: vec![],
            macroquad_conf: None,
            shared_poll_time: 200,
        }
    }
}

impl HostOptions {
    pub fn new<S: AsRef<str>>(shared_object: S) -> HostOptions {
        let mut opts = HostOptions::default();
        opts.shared_object = shared_object.as_ref().into();
        opts
    }

    pub fn with_poll_interval(mut self, interval: u64) -> HostOptions {
        self.shared_poll_time = interval;
        self
    }

    pub fn with_macroquad_config(mut self, conf: Conf) -> HostOptions {
        self.macroquad_conf = Some(conf);
        self
    }

    pub fn with_project_path<S: AsRef<str>>(mut self, path: S) -> HostOptions {
        self.cargo_project_path = path.as_ref().into();
        self
    }

    pub fn with_watch_file<S: AsRef<str>>(mut self, file: S) -> HostOptions {
        self.watch_files.push(file.as_ref().into());
        self
    }

    /// theres no going back
    pub fn run(self) {
        run_host(self)
    }
}

pub fn run_host(mut opts: HostOptions) {
    let macroquad_conf = opts.macroquad_conf.take().unwrap_or(Conf::default());
    if opts.watch_files.is_empty() {
        opts.watch_files = vec!["./src/".into()];
    }
    let shared_object_exists = opts.shared_object.exists();
    let shared_object_is_file = opts.shared_object.is_file();
    if !shared_object_exists || !shared_object_is_file {
        eprintln!("Failed to find {:?} on startup.", opts.shared_object);
        eprintln!("Attempting initial build in {:?}", opts.cargo_project_path);
        if let Err(e) = build_lib(&opts.cargo_project_path) {
            eprintln!("Failed to build library:\n{}", e);
            eprintln!("Exiting without launching window");
            std::process::exit(1);
        } else {
            eprintln!("Initial build successful. Launching Host");
        }
    }
    macroquad::Window::from_config(macroquad_conf, real_main(opts));
}

pub fn build_lib(lib_dir: &str) -> Result<(), String> {
    let res = Command::new("cargo")
        .args(&["build", "--lib"])
        .current_dir(lib_dir)
        .output();
    let err_str = match res {
        Ok(out) => {
            if out.status.success() { return Ok(()) }
            String::from_utf8_lossy(&out.stderr).to_string()
        }
        Err(e) => e.to_string(),
    };
    Err(err_str)
}

pub fn lib_built_successfully(lib_dir: &str) -> bool {
    if let Err(e) = build_lib(lib_dir) {
        eprintln!("Failed to build library:\n{}", e);
        false
    } else {
        true
    }
}

pub fn get_all_files_recursively(source: &PathBuf, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let paths = fs::read_dir(source)?;
    for path in paths {
        let path = path?;
        let path = path.path();
        if path.is_dir() {
            get_all_files_recursively(&path, out)?;
        } else {
            out.push(path);
        }
    }

    Ok(())
}

pub fn create_watchers(paths: &Vec<String>) -> Vec<DFWatcher> {
    let mut individual_files = vec![];
    for p in paths {
        let path = PathBuf::from(p);
        if path.is_dir() {
            let _ = get_all_files_recursively(&path, &mut individual_files);
        } else {
            individual_files.push(path);
        }
    }

    let mut watchers = vec![];
    for file in individual_files {
        let w = DFWatcher::new(file);
        watchers.push(w);
    }
    watchers
}

async fn real_main(opts: HostOptions) {
    let watchers = create_watchers(&opts.watch_files);
    // println!("Using watcher files:");
    // for w in &watchers {
    //     println!("{:?}", w.path);
    // }
    let mut watchman = DFOwner::start(opts.shared_poll_time, watchers);


    unsafe {
        let mut data: Box<dyn Any> = Box::new(());
        let mut lib = Library::new(&opts.shared_object).unwrap();
        let mut update_fn: Symbol<unsafe extern "C" fn(ctxh: ContextHelper, data: Box<dyn Any>) -> Box<dyn Any>> = lib.get(b"mqhr_update").unwrap();
        let mut init_fn: Symbol<unsafe extern "C" fn(data: Box<dyn Any>) -> Box<dyn Any>> = lib.get(b"mqhr_init").unwrap();
        data = init_fn(data);

        loop {
            if watchman.should_update() {
                println!("Change detected. Building new shared object");
                if !lib_built_successfully(&opts.cargo_project_path) {
                    break;
                }
                println!("Reloading...");
                if lib.close().is_err() {
                    println!("Error closing shared object.");
                    break;
                }
                lib = Library::new(&opts.shared_object).unwrap();
                update_fn = lib.get(b"mqhr_update").unwrap();
                init_fn = lib.get(b"mqhr_init").unwrap();
                data = init_fn(data);
                println!("Successfully reloaded");
            }

            let context = macroquad::get_context();
            let ctxh = ContextHelper { context };
            data = update_fn(ctxh, data);
            macroquad::window::next_frame().await;
        }
    }
}
