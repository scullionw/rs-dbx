extern crate notify;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

struct Watchdog<T: Reactor> {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::DebouncedEvent>,
    reactor: T,
}

impl<T: Reactor> Watchdog<T> {
    fn new(reactor: T) -> Watchdog<T> {
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            Watcher::new(tx, Duration::from_secs(2)).expect("Couldn't choose a watcher.");

        watcher
            .watch(reactor.monitored(), RecursiveMode::Recursive)
            .unwrap();

        Watchdog {
            _watcher: watcher,
            rx,
            reactor,
        }
    }

    fn run(self) -> Result<(), &'static str> {
        for _ in self.rx {
            self.reactor.run()?;
        }
        Ok(())
    }
}

fn main() -> Result<(), &'static str> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    if args.len() < 3 {
        return Err("Need 3 arguments!");
    }

    let source = &args[0];
    let target = &args[1];
    let ignorefile = &args[2];

    if !Path::new(source).is_dir() {
        return Err("Source is not a directory!");
    }

    if !Path::new(target).is_dir() {
        return Err("Target is not a directory!");
    }

    if !Path::new(ignorefile).is_file() {
        return Err("Ignore is not a file!");
    }

    let dropbox_mirror = Mirror::new(source, target, ignorefile);
    let watchdog = Watchdog::new(dropbox_mirror);

    watchdog.run()?;

    Ok(())
}

struct Mirror {
    source: String,
    target: String,
    ignorefile: String,
}

trait Reactor {
    fn run(&self) -> Result<(), &'static str>;
    fn monitored(&self) -> String;
}

impl Mirror {
    fn new(source: &str, target: &str, ignorefile: &str) -> Mirror {
        Mirror {
            source: source.to_owned(),
            target: target.to_owned(),
            ignorefile: ignorefile.to_owned(),
        }
    }
}

impl Reactor for Mirror {
    fn run(&self) -> Result<(), &'static str> {
        let output = Command::new("rsync")
                            .arg("-a")
                            .arg("--delete")
                            .arg("--exclude-from")
                            .arg(&self.ignorefile)
                            .arg(&self.source) // TODO: add trailing slash
                            .arg(&self.target)
                            .output()
                            .expect("rsync failed to start");

        if !output.status.success() {
            println!("status: {}", output.status);
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            Err("rsync failed")
        } else {
            Ok(())
        }
    }

    fn monitored(&self) -> String {
        self.source.clone()
    }
}

struct Diff;

impl Diff {
    fn show_not_copied() {
        unimplemented!();
    }
}
