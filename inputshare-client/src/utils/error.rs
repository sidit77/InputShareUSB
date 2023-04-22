use std::error::Error as StdError;
use std::fmt::{Formatter, Write};
use std::panic::Location;

use eyre::{Chain, EyreHandler};
use tracing_error::SpanTrace;

pub struct SpanTraceHandler {
    spantrace: Option<SpanTrace>,
    location: Option<&'static Location<'static>>
}

impl EyreHandler for SpanTraceHandler {
    fn debug(&self, error: &(dyn StdError + 'static), f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return core::fmt::Debug::fmt(error, f);
        }

        write!(f, "{}", error)?;

        if let Some(cause) = error.source() {
            write!(f, "\n\nCaused by:")?;
            let multiple = cause.source().is_some();
            for (n, error) in Chain::new(cause).enumerate() {
                writeln!(f)?;
                if multiple {
                    write!(indenter::indented(f).ind(n), "{}", error)?;
                } else {
                    write!(indenter::indented(f), "{}", error)?;
                }
            }
        }

        if let Some(location) = self.location {
            write!(f, "\n\nLocation:\n")?;
            write!(indenter::indented(f), "{}", location)?;
        }

        if let Some(spantrace) = &self.spantrace {
            write!(f, "\n\nSpan Trace:\n")?;
            write!(f, "{}", spantrace)?;
        }

        Ok(())
    }

    fn track_caller(&mut self, location: &'static Location<'static>) {
        self.location = Some(location)
    }
}

pub fn set_eyre_hook() {
    eyre::set_hook(Box::new(move |_| {
        Box::new(SpanTraceHandler {
            spantrace: Some(SpanTrace::capture()),
            location: None
        })
    }))
    .expect("failed to install eyre hook")
}

pub fn strip_color(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut escape = false;
    for c in s.chars() {
        if escape {
            if c == 'm' {
                escape = false;
            }
        } else if c == '\u{001b}' {
            escape = true;
        } else {
            result.push(c);
        }
    }
    result
}
