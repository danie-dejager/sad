use {
  super::{
    argparse::{Action, Engine, Options},
    fs_pipe::{slurp, spit},
    input::RowIn,
    types::Die,
    udiff::{apply_patches, patches, pure_diffs, udiff},
  },
  ansi_term::Colour,
  std::{borrow::ToOwned, ffi::OsString, path::PathBuf},
  tokio::task::spawn_blocking,
};

impl Engine {
  fn replace(&self, before: &str) -> String {
    match self {
      Self::AhoCorasick(ac, replace) => ac.replace_all(before, &[replace.as_str()]),
      Self::Regex(re, replace) => re.replace_all(before, replace.as_str()).into(),
    }
  }
}

impl RowIn {
  const fn path(&self) -> &PathBuf {
    match self {
      Self::Entire(path) | Self::Piecewise(path, _) => path,
    }
  }
}

fn diff(opts: &Options, before: &[String]) -> Vec<String> {
  let b = before.iter().cloned().collect::<String>();
  opts
    .engine
    .replace(&b)
    .split_inclusive('\n')
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>()
}

pub async fn displace(opts: &Options, input: RowIn) -> Result<OsString, Die> {
  let path = input.path().clone();
  let slurped = slurp(&path).await?;
  let before = slurped.content;
  if before.len() == 0 {
    Ok(OsString::new())
  } else {
    let mut name = opts
      .cwd
      .as_ref()
      .and_then(|cwd| path.strip_prefix(cwd).ok())
      .unwrap_or_else(|| path.as_ref())
      .as_os_str()
      .to_owned();

    let after = diff(opts, &before);

    let print = match (&opts.action, input) {
      (Action::Preview, RowIn::Entire(_)) => udiff(None, opts.unified, &name, &before, &after),
      (Action::Preview, RowIn::Piecewise(_, ranges)) => {
        udiff(Some(&ranges), opts.unified, &name, &before, &after)
      }
      (Action::Commit, RowIn::Entire(_)) => {
        spit(&path, &slurped.meta, after).await?;
        name.push("\n");
        name
      }
      (Action::Commit, RowIn::Piecewise(_, ranges)) => {
        let patches = patches(opts.unified, &before, &after);
        let after = apply_patches(patches, &ranges, &before);
        spit(&path, &slurped.meta, after).await?;
        name.push("\n");
        name
      }
      (Action::FzfPreview(_, _), _) => {
        let ranges = pure_diffs(opts.unified, &before, &after);
        if ranges.len() == 0 {
          OsString::new()
        } else {
          let mut fzf_lines = OsString::new();
          for range in ranges {
            let repr = Colour::Red.paint(format!("{range}"));
            fzf_lines.push(&name);
            let line = format!("\x04 {repr}\0");
            fzf_lines.push(&line);
          }
          fzf_lines
        }
      }
    };
    Ok(print)
  }
}
