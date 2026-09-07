//! Diagnostics and counters produced by a conversion run.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Deduplicated warning messages with occurrence counts.
#[derive(Debug, Default, Clone)]
pub struct Warnings {
    counts: BTreeMap<String, usize>,
}

impl Warnings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, message: impl Into<String>) {
        *self.counts.entry(message.into()).or_insert(0) += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /// Distinct messages with how often each fired, in a stable order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, usize)> {
        self.counts.iter().map(|(m, n)| (m.as_str(), *n))
    }

    pub fn merge(&mut self, other: &Warnings) {
        for (message, count) in &other.counts {
            *self.counts.entry(message.clone()).or_insert(0) += count;
        }
    }
}

impl fmt::Display for Warnings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (message, count) in self.iter() {
            if count > 1 {
                writeln!(f, "  ({count}x) {message}")?;
            } else {
                writeln!(f, "  {message}")?;
            }
        }
        Ok(())
    }
}

/// The outcome of converting one file.
#[derive(Debug, Default, Clone)]
pub struct FileReport {
    pub features: usize,
    pub warnings: Warnings,
    /// Code-list file names the output references through a `codelists/`
    /// `codeSpace` path.
    pub code_spaces: BTreeSet<String>,
}

/// The outcome of converting a whole dataset.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub converted: usize,
    pub copied: usize,
    pub features: usize,
    pub warnings: Warnings,
    /// Union of every converted file's referenced code-list names.
    pub code_spaces: BTreeSet<String>,
}

impl Report {
    pub fn absorb(&mut self, file: &FileReport) {
        self.converted += 1;
        self.features += file.features;
        self.warnings.merge(&file.warnings);
        self.code_spaces.extend(file.code_spaces.iter().cloned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_repeats_and_merges() {
        let mut a = Warnings::new();
        a.add("x");
        a.add("x");
        a.add("y");
        assert_eq!(a.len(), 2);

        let mut b = Warnings::new();
        b.add("x");
        b.merge(&a);
        assert_eq!(b.iter().collect::<Vec<_>>(), [("x", 3), ("y", 1)]);
    }
}
