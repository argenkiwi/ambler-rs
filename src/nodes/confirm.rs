use crate::Node;
use std::io;

/// Where [`Confirm`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Yes,
    /// The answer was no, or input ended.
    No,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read the answer")]
    Io(#[from] io::Error),
}

pub trait Utils {
    /// Prints `message` and reads a line, or `None` at end of input.
    fn read_line(&self, message: &str) -> io::Result<Option<String>>;
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn read_line(&self, message: &str) -> io::Result<Option<String>> {
        use std::io::Write;
        println!("{message}");
        io::stdout().flush()?;
        let mut input = String::new();
        Ok(match io::stdin().read_line(&mut input)? {
            0 => None,
            _ => Some(input.trim_end_matches(['\r', '\n']).to_string()),
        })
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Asks a yes/no question until it gets `y`/`yes` or `n`/`no`. End of input counts as no.
#[derive(Clone, Debug)]
pub struct Confirm<U = SystemUtils> {
    question: String,
    utils: U,
}

impl Confirm {
    #[must_use]
    pub fn new(question: impl Into<String>) -> Self {
        Self::with_utils(question, SystemUtils)
    }
}

impl<U> Confirm<U> {
    #[must_use]
    pub fn with_utils(question: impl Into<String>, utils: U) -> Self {
        Self {
            question: question.into(),
            utils,
        }
    }
}

impl<S, U> Node<S> for Confirm<U>
where
    S: Send,
    U: Utils + Sync,
{
    type Outcome = Outcome;
    type Error = Error;

    async fn run(&self, _state: &mut S) -> Result<Outcome, Error> {
        loop {
            let Some(answer) = self.utils.read_line(&self.question)? else {
                return Ok(Outcome::No);
            };
            match answer.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(Outcome::Yes),
                "n" | "no" => return Ok(Outcome::No),
                _ => self.utils.print("Please answer y or n."),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct MockUtils {
        lines: Mutex<VecDeque<io::Result<&'static str>>>,
        prints: Mutex<usize>,
    }

    impl MockUtils {
        fn new(lines: Vec<io::Result<&'static str>>) -> Self {
            Self {
                lines: Mutex::new(lines.into()),
                prints: Mutex::new(0),
            }
        }
    }

    impl Utils for &MockUtils {
        fn read_line(&self, _message: &str) -> io::Result<Option<String>> {
            self.lines
                .lock()
                .unwrap()
                .pop_front()
                .transpose()
                .map(|line| line.map(str::to_string))
        }
        fn print(&self, _message: &str) {
            *self.prints.lock().unwrap() += 1;
        }
    }

    async fn run(lines: Vec<io::Result<&'static str>>) -> (Result<Outcome, Error>, usize) {
        let utils = MockUtils::new(lines);
        let result = Confirm::with_utils("?", &utils).run(&mut ()).await;
        (result, *utils.prints.lock().unwrap())
    }

    #[tokio::test]
    async fn yes_and_y_confirm() {
        assert_eq!(run(vec![Ok("y")]).await.0.unwrap(), Outcome::Yes);
        assert_eq!(run(vec![Ok("YES")]).await.0.unwrap(), Outcome::Yes);
    }

    #[tokio::test]
    async fn no_and_n_decline() {
        assert_eq!(run(vec![Ok("n")]).await.0.unwrap(), Outcome::No);
        assert_eq!(run(vec![Ok(" No ")]).await.0.unwrap(), Outcome::No);
    }

    #[tokio::test]
    async fn asks_again_until_the_answer_is_valid() {
        let (result, prints) = run(vec![Ok(""), Ok("maybe"), Ok("y")]).await;

        assert_eq!(result.unwrap(), Outcome::Yes);
        assert_eq!(prints, 2);
    }

    #[tokio::test]
    async fn end_of_input_declines() {
        assert_eq!(run(vec![]).await.0.unwrap(), Outcome::No);
    }

    #[tokio::test]
    async fn fails_if_the_answer_cannot_be_read() {
        let (result, _) = run(vec![Err(io::ErrorKind::BrokenPipe.into())]).await;

        assert!(matches!(result, Err(Error::Io(_))));
    }
}
