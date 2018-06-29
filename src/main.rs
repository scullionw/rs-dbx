extern crate notify;

use notify::{RecommendedWatcher, Watcher, RecursiveMode};

use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::error::Error;


fn watch<T: Reactor>(dir: &str, reactor: T) -> notify::Result<()> {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx, Duration::from_secs(10)));
    try!(watcher.watch(dir, RecursiveMode::Recursive));

    for _ in rx {
        reactor.run().expect("Run failure");
    }
    
    Ok(())
}


fn main() {

    let args = env::args().skip(1).collect::<Vec<String>>();

    let source = &args[0];
    let target = &args[1];
    let ignorefile = &args[2];

    if !Path::new(source).is_dir() {
        eprintln!("Source is not a directory!");
        std::process::exit(1)
    }

    if !Path::new(target).is_dir() {
        eprintln!("Target is not a directory!");
        std::process::exit(1)
    }

    if !Path::new(ignorefile).is_file() {
        eprintln!("Ignore is not a file!");
        std::process::exit(1)
    }

    let dropbox_mirror = Mirror::new(source, target, ignorefile);

    
    //dropbox_mirror.run().expect("Run failure");
    let _ = watch(source, dropbox_mirror).unwrap();

}

struct Mirror {
    source: String,
    target: String,
    ignorefile: String
}

trait Reactor {
    fn run(&self) -> Result<(), Box<Error>>;
}

impl Mirror {
    fn new(source: &str, target: &str, ignorefile: &str) -> Mirror {
        Mirror { 
            source: source.to_owned(),
            target: target.to_owned(),
            ignorefile: ignorefile.to_owned()
        }
    }
}

impl Reactor for Mirror {
    fn run(&self) -> Result<(), Box<Error>> {
    let output = Command::new("rsync")
                            .arg("-a")
                            .arg("--delete")
                            .arg("--exclude-from")
                            .arg(&self.ignorefile)
                            .arg(&self.source) // TODO: add trailing slash
                            .arg(&self.target)
                            .output()
                            .expect("rsync failed to start");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));

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



