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
    mode: DiffMode,
}

impl<T: Reactor> Watchdog<T> {
    fn new(reactor: T, mode: DiffMode) -> Watchdog<T> {
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
            mode,
        }
    }

    fn run(self) -> Result<(), &'static str> {
        for _ in self.rx {
            self.reactor.run()?;
            show_not_copied(self.mode, &self.reactor.monitored(), &self.reactor.target())?;
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
    let watchdog = Watchdog::new(dropbox_mirror, DiffMode::NotCopied);

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
    fn target(&self) -> String;
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

    fn target(&self) -> String {
        self.target.clone()
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

use std::fs;
use std::process::Stdio;

// create or append to file
fn show_not_copied(mode: DiffMode, source_dir: &str, target_dir: &str) -> Result<(), &'static str> {
    match mode {
        DiffMode::NotCopied => {
            let diff_output = Command::new("diff")
                .args(&["-rq", source_dir, target_dir])
                .stdout(Stdio::piped())
                .spawn() // Why not output?
                .expect("diff failed to run");

            // if !diff_output.status.success() {
            //     println!("status: {}", diff_output.status);
            //     println!("stdout: {}", String::from_utf8_lossy(&diff_output.stdout));
            //     eprintln!("stderr: {}", String::from_utf8_lossy(&diff_output.stderr));
            //     Err("diff failed")
            // } else {
            //     println!("stdout: {}", String::from_utf8_lossy(&diff_output.stdout));
            //     Ok(())
            // }

            let grep_output = Command::new("grep")
                .arg(["Only in", source_dir].join(" "))
                .stdin(diff_output.stdout.unwrap())
                .output()
                .expect("grep failed to run");

            if !grep_output.status.success() {
                println!("status: {}", grep_output.status);
                println!("stdout: {}", String::from_utf8_lossy(&grep_output.stdout));
                eprintln!("stderr: {}", String::from_utf8_lossy(&grep_output.stderr));
                Err("diff or grep failed")
            } else {
                let result = String::from_utf8_lossy(&grep_output.stdout);
                fs::write(".rsynclog", &result[..]).unwrap();
                Ok(())
            }
        }
        DiffMode::Off => Ok(()),
    }
}
