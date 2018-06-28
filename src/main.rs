extern crate notify;

use notify::{RecommendedWatcher, Watcher, RecursiveMode};

use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::error::Error;


fn watch(dir: &str) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx, Duration::from_secs(2)));

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    try!(watcher.watch(dir, RecursiveMode::Recursive));

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}

fn validate_path(filepath: &str) -> Result<String, &'static str>  {
    let path = Path::new(filepath);
    if path.is_dir() {
        Ok(filepath.to_owned())
    } else {
        Err("Not a directory!")
    }
}

fn main() {

    let args = env::args()
                    .skip(1)
                    .take(3)
                    .map(move |x| validate_path(&x).unwrap() )
                    .collect::<Vec<String>>();

    let dropbox_mirror = Mirror::new(&args[0], &args[1], &args[2]);
    

    loop {
        if let Err(e) = watch(&args[0]) { // pop watch list before calling rsync
            println!("error: {:?}", e)
        } else {
            dropbox_mirror.run().expect("Run failure");
        }
    }
}

struct Mirror<'a> {
    source: &'a str,
    target: &'a str,
    ignorefile: &'a str,
}

impl<'a> Mirror<'a> {
    fn new<'b>(source: &'b str, target: &'b str, ignorefile: &'b str) -> Mirror<'b> {
        Mirror { source, target, ignorefile }
    }

    fn run(&self) -> Result<(), Box<Error>> {
        let output = Command::new("rsync")
                             .arg("--delete")
                             .arg("--exclude-from")
                             .arg(&self.ignorefile)
                             .arg(&self.source) // TODO: add trailing slash
                             .arg(&self.target)
                             .output()
                             .expect("rsync failed to start");

        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        assert!(output.status.success());
        // let s = if output.status.success() {
        //     String::from_utf8_lossy(&output.stdout)
        // } else {
        //     String::from_utf8_lossy(&output.stderr)
        // };
        Ok(())
    }
}


struct Diff;

impl Diff {
    fn show_not_copied() {
        unimplemented!();
    }
}



