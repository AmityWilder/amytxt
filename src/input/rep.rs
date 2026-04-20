use super::KeyInput;
use std::time::{Duration, Instant};

/// Repeat a [`KeyInput`] while it is held down
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyRepeater {
    /// The key to repeat and the timestamp of when it will be to repeat
    ///
    /// Only the most recently pressed key can be repeated
    pub input: Option<(KeyInput, Instant)>,
    /// How long to wait before repeating a key input after the initial press
    pub start_delay: Duration,
    /// How long to wait between key input repetitions after `start_delay`
    pub period: Duration,
}

impl Default for KeyRepeater {
    fn default() -> Self {
        Self::new(Duration::ZERO, Duration::ZERO)
    }
}

impl KeyRepeater {
    /// Construct a new [`KeyRepeater`] without an input initially
    pub const fn new(start_delay: Duration, period: Duration) -> Self {
        Self {
            input: None,
            start_delay,
            period,
        }
    }

    /// Indicate that a key has been pressed and replace the held key
    pub fn press(&mut self, input: KeyInput) {
        self.input = Some((input, Instant::now() + self.start_delay));
    }

    /// Indicate that a key has been released and clear the held key if it matches
    pub fn release(&mut self, input: &KeyInput) {
        if self.input.as_ref().is_some_and(|(key, _)| key == input) {
            self.input = None;
        }
    }

    /// Get the key input if there is one and this tick should repeat it
    pub fn check_repeat(&self) -> Option<KeyInput> {
        self.input.and_then(|(key, pressed)| {
            let now = Instant::now();
            now.checked_duration_since(pressed)
                .is_some_and(|duration| duration.as_nanos() % self.period.as_nanos() == 0)
                .then_some(key)
        })
    }
}
