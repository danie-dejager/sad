use {
  super::{
    argparse::Mode,
    subprocess::{stream_subproc, SubprocCommand},
    types::Fail,
  },
  futures::stream::{BoxStream, Stream, StreamExt},
  std::{
    collections::HashMap,
    env::{self, current_exe},
    ffi::OsString,
    path::PathBuf,
    process::Stdio,
  },
  tokio::{io::ErrorKind, process::Command},
  which::which,
};

async fn reset_term() -> Result<(), Fail> {
  if let Ok(path) = which("tput") {
    let status = Command::new(&path)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .arg("reset")
      .status()
      .await
      .map_err(|e| Fail::IO(path, e.kind()))?;

    if status.success() {
      return Ok(());
    }
  }
  if let Ok(path) = which("reset") {
    let status = Command::new(&path)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .status()
      .await
      .map_err(|e| Fail::IO(path, e.kind()))?;
    if status.success() {
      return Ok(());
    }
  }
  Err(Fail::IO(PathBuf::from("reset"), ErrorKind::NotFound))
}

pub fn stream_fzf_proc<'a>(
  bin: PathBuf,
  args: Vec<String>,
  stream: impl Stream<Item = Result<OsString, Fail>> + Unpin + Send + 'a,
) -> Box<dyn Stream<Item = Result<(), Fail>> + Send + 'a> {
  let execute = format!("abort+execute:{}\x04{{+f}}", Mode::PATCH);
  let mut arguments = vec![
    "--read0".to_owned(),
    "--print0".to_owned(),
    "-m".to_owned(),
    "--ansi".to_owned(),
    "--preview-window=70%:wrap".to_owned(),
    format!("--bind=enter:{execute}"),
    format!("--bind=double-click:{execute}"),
    format!("--preview={}\x04{{f}}", Mode::PREVIEW),
  ];
  arguments.extend(args);

  let mut fzf_env = HashMap::new();
  fzf_env.insert(
    Mode::ARGV.to_owned(),
    env::args().collect::<Vec<_>>().join("\x04"),
  );
  fzf_env.insert(
    "SHELL".to_owned(),
    current_exe()
      .or_else(|_| which(env!("CARGO_PKG_NAME")))
      .map_or_else(
        |_| env!("CARGO_PKG_NAME").to_owned(),
        |path| format!("{}", path.display()),
      ),
  );
  fzf_env.insert("LC_ALL".to_owned(), "C".to_owned());

  let cmd = SubprocCommand {
    prog: bin,
    args: arguments,
    env: fzf_env,
  };
  let stream = BoxStream::from(stream_subproc(cmd, stream)).then(|line| async {
    match line {
      Ok(o) => Ok(o),
      e => {
        let _ = reset_term().await;
        e
      }
    }
  });
  Box::new(stream)
}
