use crate::Node;
use crate::nodes::route::USAGE;
use std::convert::Infallible;

pub trait Utils {
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Prints the usage of every verb and the exit codes, on standard output. Terminal: it has no
/// outgoing edges.
#[derive(Clone, Debug, Default)]
pub struct Help<U = SystemUtils> {
    utils: U,
}

impl Help {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> Help<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Help<U>
where
    S: Send,
    U: Utils + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, _state: &mut S) -> Result<(), Infallible> {
        self.utils.print(USAGE);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Fake, State};

    #[tokio::test]
    async fn prints_the_usage_on_standard_output() {
        let fake = Fake::default();
        let mut state = State::default();

        let Ok(()) = Help::with_utils(&fake).run(&mut state).await;

        assert_eq!(fake.printed(), [USAGE]);
        assert!(fake.eprinted().is_empty());
        assert_eq!(state.message, None);
        assert_eq!(state.exit_code, 0);
    }
}
