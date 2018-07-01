extern crate notify;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use std::env;
use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

struct Watchdog<'a, T: Reactor + 'a> {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::DebouncedEvent>,
    reactor: &'a T,
}

impl<'a, T: Reactor> Watchdog<'a, T> {
    fn new(reactor: &T) -> Watchdog<T> {
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

    fn run(self) {
        for _ in self.rx {
            self.reactor.run().expect("Run failure");
        }
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();

    if args.len() < 3 {
        eprintln!("Need 3 arguments!");
        std::process::exit(1);
    }

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
    let watchdog = Watchdog::new(&dropbox_mirror);
    watchdog.run();
}

struct Mirror {
    source: String,
    target: String,
    ignorefile: String,
}

trait Reactor {
    fn run(&self) -> Result<(), Box<Error>>;
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

    fn monitored(&self) -> String {
        //(*&self.source).clone()
        //String::from(&self.source[..])
        self.source.clone()
    }
}

struct Diff;

impl Diff {
    fn show_not_copied() {
        unimplemented!();
    }
}
