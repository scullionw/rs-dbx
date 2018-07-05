extern crate notify;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use std::env;
use std::fs;
use std::marker::{Send, Sync};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Monitor {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::DebouncedEvent>,
}

impl Monitor {
    fn new(dir: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            Watcher::new(tx, Duration::from_secs(2)).expect("Couldn't choose a watcher.");

        watcher.watch(dir, RecursiveMode::Recursive).unwrap();

        Self {
            _watcher: watcher,
            rx,
        }
    }
}

struct Watchdog<T: Reactor + Send + Sync + 'static> {
    reactor: Arc<Mutex<T>>,
    mode: DiffMode,
}

fn useful_notice(event: &notify::DebouncedEvent) -> bool {
    match event {
        notify::DebouncedEvent::Remove(_x)
        | notify::DebouncedEvent::Create(_x)
        | notify::DebouncedEvent::Write(_x) => true,
        notify::DebouncedEvent::Rename(_x, _y) => true,
        _ => false,
    }
}

impl<T: Reactor + Send + Sync + 'static> Watchdog<T> {
    fn new(reactor: T, mode: DiffMode) -> Self {
        Self {
            reactor: Arc::new(Mutex::new(reactor)),
            mode,
        }
    }

    fn run(self) {
        let source_monitor = Monitor::new(&self.reactor.lock().unwrap().monitored());
        let target_monitor = Monitor::new(&self.reactor.lock().unwrap().target());

        let c_reactor = Arc::clone(&self.reactor);

        let source_handle = thread::spawn(move || {
            for (i, event) in source_monitor
                .rx
                .iter()
                .filter(|x| useful_notice(x))
                .enumerate()
            {
                if let Ok(ref lock) = self.reactor.try_lock() {
                    lock.run().unwrap();
                    show_not_copied(self.mode, &lock.monitored(), &lock.target()).unwrap();
                    println!("S -> T = {} : {:?}", i, event)
                } else {
                    println!("S -> T = {} LOCKED : {:?}", i, event)
                }
            }
        });

        let target_handle = thread::spawn(move || {
            for (i, event) in target_monitor
                .rx
                .iter()
                .filter(|x| useful_notice(x))
                .enumerate()
            {
                if let Ok(ref lock) = c_reactor.try_lock() {
                    lock.run_reverse().unwrap();
                    println!("T -> S = {} : {:?}", i, event)
                } else {
                    println!("T -> S = {} LOCKED : {:?}", i, event)
                }
            }
        });

        for handle in vec![source_handle, target_handle] {
            handle.join().unwrap();
        }
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
    let watchdog = Watchdog::new(dropbox_mirror, DiffMode::NotCopied);

    watchdog.run();

    Ok(())
}

#[derive(Clone)]
struct Mirror {
    source: String,
    target: String,
    ignorefile: String,
}

trait Reactor: Clone {
    fn run(&self) -> Result<(), &'static str>;
    fn run_reverse(&self) -> Result<(), &'static str>;
    fn monitored(&self) -> String;
    fn target(&self) -> String;
}

impl Mirror {
    fn new(source: &str, target: &str, ignorefile: &str) -> Self {
        Self {
            source: source.to_owned(),
            target: target.to_owned(),
            ignorefile: ignorefile.to_owned(),
        }
    }
}

impl Reactor for Mirror {
    fn run(&self) -> Result<(), &'static str> {
        rsync(&self.source, &self.target, &self.ignorefile)
    }

    fn run_reverse(&self) -> Result<(), &'static str> {
        rsync(&self.target, &self.source, &self.ignorefile)
    }

    fn monitored(&self) -> String {
        self.source.clone()
    }

    fn target(&self) -> String {
        self.target.clone()
    }
}

fn rsync(source: &str, target: &str, ignorefile: &str) -> Result<(), &'static str> {
    let output = Command::new("rsync")
                            .arg("-a")
                            .arg("--delete")
                            .arg("--exclude-from")
                            .arg(ignorefile)
                            .arg(source) // TODO: add trailing slash
                            .arg(target)
                            .output()
                            .expect("rsync failed to start");

    if output.status.success() {
        Ok(())
    } else {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        Err("rsync failed")
    }
}

#[derive(Copy, Clone)]
enum DiffMode {
    NotCopied,
    Off,
}

// Why did they need unsafe to do this?
// https://github.com/mgattozzi/pipers/blob/master/src/lib.rs#L52:48
// https://www.reddit.com/r/rust/comments/3azfie/how_to_pipe_one_process_into_another/
// https://github.com/rust-lang/rust/pull/42133
// https://github.com/rust-lang/rust/issues/34593
// https://github.com/rust-lang/rfcs/issues/1519
// https://github.com/oconnor663/duct.rs
// https://github.com/oconnor663/os_pipe.rs

// create or append to file
// deal with two-way logs
fn show_not_copied(mode: DiffMode, source_dir: &str, target_dir: &str) -> Result<(), &'static str> {
    match mode {
        DiffMode::NotCopied => {
            let diff_output = Command::new("diff")
                .args(&["-rq", source_dir, target_dir])
                .stdout(Stdio::piped())
                .spawn() // Why not output?
                .expect("diff failed to run");

            let grep_output = Command::new("grep")
                .arg(["Only in", source_dir].join(" "))
                .stdin(diff_output.stdout.unwrap())
                .output()
                .expect("grep failed to run");

            if grep_output.status.success() {
                let result = String::from_utf8_lossy(&grep_output.stdout);
                fs::write(".rsynclog", &result[..]).unwrap();
                Ok(())
            } else {
                println!("status: {}", grep_output.status);
                println!("stdout: {}", String::from_utf8_lossy(&grep_output.stdout));
                eprintln!("stderr: {}", String::from_utf8_lossy(&grep_output.stderr));
                Err("diff or grep failed")
            }
        }
        DiffMode::Off => Ok(()),
    }
}
