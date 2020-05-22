use super::errors::*;
use difflib::{sequencematcher::Opcode, sequencematcher::SequenceMatcher};
use regex::Regex;
use std::{
  convert::TryFrom,
  fmt::{self, Display, Formatter},
};

// WARN: Index starts at 1
#[derive(Debug)]
pub struct DiffRange {
  before: (usize, usize),
  after: (usize, usize),
}

impl DiffRange {
  // WARN: Opcode Index starts at 0
  pub fn new(ops: &[Opcode]) -> Option<DiffRange> {
    match (ops.first(), ops.last()) {
      (Some(first), Some(last)) => Some(DiffRange {
        before: (first.first_start + 1, last.first_end - first.first_start),
        after: (first.second_start + 1, last.second_end - first.second_start),
      }),
      _ => None,
    }
  }
}

impl Display for DiffRange {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(
      f,
      "@@ -{},{} +{},{} @@",
      self.before.0, self.before.1, self.after.0, self.after.1,
    )
  }
}

impl TryFrom<&str> for DiffRange {
  type Error = Failure;

  fn try_from(candidate: &str) -> SadResult<Self> {
    let preg = r"^@@ -(\d+),(\d+) \+(\d+),(\d+) @@$";
    let re = Regex::new(preg).into_sadness()?;
    let captures = re
      .captures(candidate)
      .ok_or_else(|| Failure::Parse(candidate.into()))?;
    let before_start = captures
      .get(1)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let before_inc = captures
      .get(2)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_start = captures
      .get(3)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;
    let after_inc = captures
      .get(4)
      .ok_or_else(|| Failure::Parse(candidate.into()))?
      .as_str()
      .parse::<usize>()
      .into_sadness()?;

    Ok(DiffRange {
      before: (before_start, before_inc),
      after: (after_start, after_inc),
    })
  }
}

#[derive(Debug)]
pub struct Diff {
  range: DiffRange,
  new_lines: Vec<String>,
}

pub type Diffs = Vec<Diff>;

pub trait Patchable {
  fn new(unified: usize, before: &str, after: &str) -> Self;
  fn patch(&self, before: &[&str]) -> String;
}

impl Patchable for Diffs {
  fn new(unified: usize, before: &str, after: &str) -> Self {
    let before = before.split_terminator('\n').collect::<Vec<&str>>();
    let after = after.split_terminator('\n').collect::<Vec<&str>>();

    let mut ret = Vec::new();
    let mut matcher = SequenceMatcher::new(&before, &after);

    for group in &matcher.get_grouped_opcodes(unified) {
      let mut new_lines = Vec::new();
      for code in group {
        if code.tag == "replace" || code.tag == "insert" {
          for line in after.iter().take(code.second_end).skip(code.second_start) {
            new_lines.push(line.to_string());
          }
        }
      }
      let diff = Diff {
        range: DiffRange::new(group).unwrap(),
        new_lines,
      };
      ret.push(diff);
    }
    ret
  }

  fn patch(&self, before: &[&str]) -> String {
    let mut ret = String::new();
    let mut prev = 0;

    for diff in self.iter() {
      let (before_start, before_inc) = diff.range.before;
      for i in prev..before_start {
        before.get(i).map(|b| ret.push_str(b)).unwrap();
        ret.push('\n');
      }
      for line in diff.new_lines.iter() {
        ret.push_str(line);
        ret.push('\n')
      }
      prev = before_start - 1 + before_inc;
    }
    for i in prev..before.len() {
      before.get(i).map(|b| ret.push_str(b)).unwrap();
      ret.push('\n')
    }
    ret
  }
}

pub fn udiff(unified: usize, name: &str, before: &str, after: &str) -> String {
  let before = before.split_terminator('\n').collect::<Vec<&str>>();
  let after = after.split_terminator('\n').collect::<Vec<&str>>();
  let mut ret = String::new();
  ret.push_str(&format!("\ndiff --git {} {}", name, name));
  ret.push_str(&format!("\n--- {}", name));
  ret.push_str(&format!("\n+++ {}", name));

  let mut matcher = SequenceMatcher::new(&before, &after);
  for group in &matcher.get_grouped_opcodes(unified) {
    ret.push_str(&format!("\n{}", DiffRange::new(group).unwrap()));
    for code in group {
      if code.tag == "equal" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n {}", line))
        }
        continue;
      }
      if code.tag == "replace" || code.tag == "delete" {
        for line in before.iter().take(code.first_end).skip(code.first_start) {
          ret.push_str(&format!("\n-{}", line))
        }
      }
      if code.tag == "replace" || code.tag == "insert" {
        for line in after.iter().take(code.second_end).skip(code.second_start) {
          ret.push_str(&format!("\n+{}", line))
        }
      }
    }
  }
  ret
}
