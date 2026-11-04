use ambler::Walk;
use ambler::nodes::count::{self, Count};
use ambler::nodes::start::{self, Start};
use ambler::nodes::stop::{self, Stop};
use ambler::walks::counter::{Counter, Id, State};
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

/// Plays back typed lines and coin flips, and records everything printed.
struct Script {
    lines: Mutex<VecDeque<&'static str>>,
    flips: Mutex<VecDeque<bool>>,
    printed: Mutex<Vec<String>>,
}

impl Script {
    fn new(lines: &[&'static str], flips: &[bool]) -> Self {
        Self {
            lines: Mutex::new(lines.iter().copied().collect()),
            flips: Mutex::new(flips.iter().copied().collect()),
            printed: Mutex::default(),
        }
    }

    fn record(&self, message: &str) {
        self.printed.lock().unwrap().push(message.to_string());
    }
}

impl start::Utils for &Script {
    fn read_line(&self, _message: &str) -> io::Result<Option<String>> {
        Ok(self.lines.lock().unwrap().pop_front().map(str::to_string))
    }
    fn print(&self, message: &str) {
        self.record(message);
    }
}

impl count::Utils for &Script {
    fn print(&self, message: &str) {
        self.record(message);
    }
    async fn sleep(&self, _duration: Duration) {}
    fn random_bool(&self, _p: f64) -> bool {
        self.flips.lock().unwrap().pop_front().unwrap()
    }
}

impl stop::Utils for &Script {
    fn print(&self, message: &str) {
        self.record(message);
    }
}

fn counter(script: &Script) -> Counter<&Script, &Script, &Script> {
    Counter {
        start: Start::with_utils(script),
        count: Count::with_utils(script),
        stop: Stop::with_utils(script),
    }
}

#[tokio::test]
async fn retries_invalid_input_then_counts_until_the_coin_says_stop() {
    let script = Script::new(&["abc", "3"], &[true, false]);
    let mut state = State { count: 99 };

    counter(&script).run(Id::Start, &mut state).await.unwrap();

    assert_eq!(state.count, 5);
    assert_eq!(
        *script.printed.lock().unwrap(),
        [
            "Error: Invalid input. Please enter a number.",
            "Current count: 3",
            "Current count: 4",
            "Final count: 5",
        ]
    );
}
